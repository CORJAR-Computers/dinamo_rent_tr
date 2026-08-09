//! services/mantenimiento.rs — Lógica de negocio de mantenimiento de vehículos
//!
//! Valida placa/tipo/fecha/costo, sincroniza `autos.proximo_aceite` cuando se
//! registra un cambio de aceite (para que el dashboard dispare la alerta por km)
//! y calcula alertas por kilometraje (aceite y frenos).

use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr as _;
use rsfbclient::{Execute, Queryable};
use serde::Serialize;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::validators::validate_no_xss;
use crate::core::PooledConnection;
use crate::repositories::auto::AutoRepository;
use crate::repositories::mantenimiento::{Mantenimiento, MantenimientoDatos, MantenimientoRepository};

/// Tipos de mantenimiento por defecto cuando config.ini no define `business.tipos_mantenimiento`
const TIPOS_MANTENIMIENTO_FALLBACK: [&str; 8] = [
    "Cambio Aceite",
    "Frenos",
    "Llantas",
    "Batería",
    "Tecno-Mecánica",
    "Lavado General",
    "Reparación Mecánica",
    "Otro",
];

/// Tipo que sincroniza el km del próximo cambio de aceite del vehículo
const TIPO_CAMBIO_ACEITE: &str = "Cambio Aceite";

/// Total por placa o tipo (para el frontend)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalMantenimiento {
    pub clave: String,
    pub total: String,
}

/// Resumen de totales para la página de mantenimiento
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalesMantenimiento {
    pub total_general: String,
    pub por_placa: Vec<TotalMantenimiento>,
    pub por_tipo: Vec<TotalMantenimiento>,
}

/// Alerta por kilometraje (cambio de aceite o frenos próximo/vencido)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertaKm {
    pub placa: String,
    pub marca: String,
    pub modelo: String,
    /// "Cambio de aceite" o "Cambio de frenos"
    pub tipo: String,
    pub km_actual: i64,
    pub km_proximo: i64,
    /// Km restantes (negativo = vencido)
    pub km_restante: i64,
    pub critica: bool,
}

pub struct MantenimientoService;

impl MantenimientoService {
    /// Lista mantenimientos con filtros opcionales (búsqueda libre, placa o tipo)
    pub fn listar(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
        placa: Option<&str>,
        tipo: Option<&str>,
    ) -> Result<Vec<Mantenimiento>, AppError> {
        let term = busqueda.unwrap_or("").trim();
        let placa = placa.unwrap_or("").trim();
        let tipo = tipo.unwrap_or("").trim();
        if !term.is_empty() {
            MantenimientoRepository::buscar(conn, term)
        } else if !placa.is_empty() && !tipo.is_empty() && tipo != "Todos" {
            // Filtros combinados: placa + tipo (en SQL, consistente con gastos)
            MantenimientoRepository::obtener_por_placa_tipo(conn, placa, tipo)
        } else if !placa.is_empty() {
            MantenimientoRepository::obtener_por_placa(conn, placa)
        } else if !tipo.is_empty() && tipo != "Todos" {
            MantenimientoRepository::obtener_por_tipo(conn, tipo)
        } else {
            MantenimientoRepository::obtener_todos(conn)
        }
    }

    /// Mantenimientos recientes
    pub fn recientes(conn: &mut PooledConnection, limit: i64) -> Result<Vec<Mantenimiento>, AppError> {
        MantenimientoRepository::obtener_recientes(conn, limit.max(1))
    }

    /// Obtiene un mantenimiento por id
    pub fn obtener(conn: &mut PooledConnection, id: i64) -> Result<Mantenimiento, AppError> {
        MantenimientoRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe el mantenimiento #{id}")))
    }

    /// Crea un mantenimiento (sincroniza autos.proximo_aceite si es cambio de aceite)
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        mut datos: MantenimientoDatos,
    ) -> Result<Mantenimiento, AppError> {
        normalizar(&mut datos);
        validar(conn, &datos, cfg)?;
        let id = MantenimientoRepository::insertar(conn, &datos)?;
        sincronizar_proximo_aceite(conn, &datos)?;
        Self::obtener(conn, id)
    }

    /// Actualiza un mantenimiento por id
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        id: i64,
        mut datos: MantenimientoDatos,
    ) -> Result<Mantenimiento, AppError> {
        Self::obtener(conn, id)?;
        normalizar(&mut datos);
        validar(conn, &datos, cfg)?;
        MantenimientoRepository::actualizar(conn, id, &datos)?;
        sincronizar_proximo_aceite(conn, &datos)?;
        Self::obtener(conn, id)
    }

    /// Elimina un mantenimiento (SOFT-DELETE). Si era un cambio de aceite,
    /// recalcula `autos.proximo_aceite` desde el último cambio de aceite del
    /// historial restante NO borrado (o lo limpia si ya no hay ninguno).
    ///
    /// TRANSACCIÓN: soft-delete mantenimiento + (si era cambio de aceite)
    /// recálculo de `autos.proximo_aceite` + INSERT auditoría. Atómico: si el
    /// recálculo o la auditoría fallan, el soft-delete se revierte y el
    /// historial queda intacto. (Grupo D: adaptado a soft-delete — el DELETE
    /// original de Grupo B se reemplazó por UPDATE ... SET deleted_at.)
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        let actual = Self::obtener(conn, id)?;
        let es_cambio_aceite = actual.tipo == TIPO_CAMBIO_ACEITE;
        let placa = actual.placa.clone();
        let tipo = actual.tipo.clone();

        conn.with_transaction(|tx| -> Result<(), rsfbclient::FbError> {
            // 1) Soft-delete del mantenimiento (Grupo D)
            tx.execute(
                "UPDATE mantenimiento_vehiculos SET deleted_at = CURRENT_TIMESTAMP \
                 WHERE id = ? AND deleted_at IS NULL",
                (id,),
            )?;
            // 2) Si era un cambio de aceite, recalcular autos.proximo_aceite
            //    desde el historial restante NO borrado (Grupo D: filtro
            //    deleted_at IS NULL para no usar el registro recién borrado).
            if es_cambio_aceite {
                let km: Option<(Option<i64>,)> = tx.query_first(
                    "SELECT first 1 km_proximo_cambio_aceite FROM mantenimiento_vehiculos \
                     WHERE placa = ? AND pieza_varias_tipo = ? AND km_proximo_cambio_aceite > 0 \
                       AND deleted_at IS NULL \
                     ORDER BY pieza_varias_fecha DESC, id DESC",
                    (placa.clone(), TIPO_CAMBIO_ACEITE.to_string()),
                )?;
                let nuevo_km = km.and_then(|(k,)| k);
                tx.execute(
                    "UPDATE autos SET proximo_aceite = ?, updated_at = CURRENT_TIMESTAMP \
                     WHERE placa = ?",
                    (nuevo_km, placa.clone()),
                )?;
            }
            // 3) Auditoría
            tx.execute(
                "INSERT INTO auditoria (usuario, accion, mensaje, ip, fecha) \
                 VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
                (
                    "sistema".to_string(),
                    "ELIMINAR MANTENIMIENTO".to_string(),
                    format!("mant={id}, placa={placa}, tipo={tipo}"),
                    "local".to_string(),
                ),
            )?;
            Ok(())
        })
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Total de mantenimientos (dashboard)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        MantenimientoRepository::contar(conn)
    }

    /// Totales general, por placa y por tipo
    pub fn totales(conn: &mut PooledConnection) -> Result<TotalesMantenimiento, AppError> {
        Ok(TotalesMantenimiento {
            total_general: MantenimientoRepository::total_general(conn)?,
            por_placa: MantenimientoRepository::total_por_placa(conn)?
                .into_iter()
                .map(|(clave, total)| TotalMantenimiento { clave, total })
                .collect(),
            por_tipo: MantenimientoRepository::total_por_tipo(conn)?
                .into_iter()
                .map(|(clave, total)| TotalMantenimiento { clave, total })
                .collect(),
        })
    }

    /// Alertas por kilometraje: cambios de aceite y frenos próximos o vencidos,
    /// según `autos.proximo_aceite` / `autos.proximo_frenos` contra el kilometraje
    /// actual, con el margen `business.km_alert_aceite` de config.ini.
    pub fn alertas_km(conn: &mut PooledConnection, cfg: &Arc<AppConfig>) -> Result<Vec<AlertaKm>, AppError> {
        let autos = AutoRepository::obtener_todos(conn)?;
        let margen = cfg.km_alert_aceite.max(0);
        let mut alertas = Vec::new();

        for a in &autos {
            for (tipo, proximo) in [
                ("Cambio de aceite", a.proximo_aceite),
                ("Cambio de frenos", a.proximo_frenos),
            ] {
                if let Some(proximo) = proximo {
                    if proximo > 0 {
                        let km_actual = a.kilometraje as i64;
                        let km_restante = proximo - km_actual;
                        if km_restante <= margen {
                            let critica = km_restante <= 0;
                            alertas.push(AlertaKm {
                                placa: a.placa.clone(),
                                marca: a.marca.clone(),
                                modelo: a.modelo.clone(),
                                tipo: tipo.into(),
                                km_actual,
                                km_proximo: proximo,
                                km_restante,
                                critica,
                            });
                        }
                    }
                }
            }
        }

        // Primero las críticas (vencidas), luego por cercanía
        alertas.sort_by_key(|a| (a.critica, a.km_restante));
        Ok(alertas)
    }
}

/// Normaliza campos (trim, mayúsculas en placa, defaults)
fn normalizar(d: &mut MantenimientoDatos) {
    d.placa = d.placa.trim().to_uppercase();
    d.tipo = d.tipo.trim().to_string();
    d.fecha = d.fecha.trim().to_string();
    d.descripcion = d.descripcion.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.observaciones = d.observaciones.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.costo = d.costo.trim().replace(',', ".");
}

/// Valida los datos del mantenimiento
fn validar(conn: &mut PooledConnection, d: &MantenimientoDatos, cfg: &Arc<AppConfig>) -> Result<(), AppError> {
    // Placa (debe existir en autos)
    if d.placa.is_empty() || d.placa.len() > 20 {
        return Err(AppError::Validation(
            "La placa es obligatoria (máx. 20 caracteres).".into(),
        ));
    }
    if !AutoRepository::existe(conn, &d.placa)? {
        return Err(AppError::Business(
            "La placa seleccionada no existe. Verifica que el vehículo esté registrado.".into(),
        ));
    }

    // Tipo
    let tipos: Vec<&str> = if cfg.tipos_mantenimiento.is_empty() {
        TIPOS_MANTENIMIENTO_FALLBACK.to_vec()
    } else {
        cfg.tipos_mantenimiento.iter().map(|s| s.as_str()).collect()
    };
    if d.tipo.is_empty() {
        return Err(AppError::Validation("El tipo de mantenimiento es obligatorio.".into()));
    }
    if !tipos.contains(&d.tipo.as_str()) {
        return Err(AppError::Validation(format!(
            "Tipo inválido '{}'. Permitidos: {}",
            d.tipo,
            tipos.join(", ")
        )));
    }

    // Fecha
    NaiveDate::parse_from_str(&d.fecha, "%Y-%m-%d").map_err(|_| {
        AppError::Validation("La fecha del mantenimiento no es válida (formato AAAA-MM-DD).".into())
    })?;

    // Costo
    let costo = Decimal::from_str(&d.costo).unwrap_or_else(|_| Decimal::from(-1));
    if costo < Decimal::ZERO {
        return Err(AppError::Validation("El costo no es un número válido.".into()));
    }
    if costo == Decimal::ZERO {
        return Err(AppError::Validation("El costo debe ser mayor que cero.".into()));
    }
    if costo > Decimal::from(9_999_999_999i64) / Decimal::from(100) {
        return Err(AppError::Validation(
            "El costo es demasiado grande (máx. 99.999.999,99).".into(),
        ));
    }

    // Km próximo cambio de aceite (no negativo)
    if let Some(km) = d.km_proximo_cambio_aceite {
        if km < 0 {
            return Err(AppError::Validation(
                "El km del próximo cambio de aceite no puede ser negativo.".into(),
            ));
        }
    }

    // XSS en campos de texto libre
    for (campo, val) in [
        ("la descripción", d.descripcion.as_deref()),
        ("las observaciones", d.observaciones.as_deref()),
    ] {
        if let Some(v) = val {
            if !v.is_empty() && validate_no_xss(v, 2000).is_err() {
                return Err(AppError::Validation(format!(
                    "{campo} contiene caracteres no permitidos."
                )));
            }
        }
    }
    Ok(())
}

/// Si el mantenimiento es un cambio de aceite, sincroniza `autos.proximo_aceite`
/// con el km programado (o lo limpia si el registro ya no lo indica) para que las
/// alertas del dashboard se disparen o se apaguen correctamente.
fn sincronizar_proximo_aceite(conn: &mut PooledConnection, d: &MantenimientoDatos) -> Result<(), AppError> {
    if d.tipo == TIPO_CAMBIO_ACEITE {
        let km = d.km_proximo_cambio_aceite.filter(|&k| k > 0);
        AutoRepository::actualizar_proximo_aceite(conn, &d.placa, km)?;
    }
    Ok(())
}
