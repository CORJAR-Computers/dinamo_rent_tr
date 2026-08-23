//! services/auto.rs — Lógica de negocio de vehículos (puerto de auto_service.py)
//!
//! Valida datos, aplica reglas de negocio (estados permitidos, fechas) y
//! calcula alertas de vencimientos (SOAT, tecno-mecánica, extintor, batería, aceite).

use std::sync::Arc;

use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr as _;
use serde::Serialize;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::validators::{validate_no_xss, mayusculas};
use crate::core::PooledConnection;
use crate::repositories::auto::{Auto, AutoDatos, AutoRepository};

/// Alerta de vencimiento o mantenimiento de un vehículo
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertaVencimiento {
    pub placa: String,
    pub marca: String,
    pub modelo: String,
    pub tipo: String,
    pub fecha: Option<String>,
    /// Días restantes (negativo = vencido). `None` para alertas por kilometraje.
    pub dias_restantes: Option<i64>,
    pub detalle: String,
    /// Vencida o a menos de 3 días
    pub critica: bool,
}

/// Días de anticipación para batería (no configurable en config.ini)
const ALERTA_BATERIA_DIAS: i64 = 30;

/// Margen de alerta de un tipo de vencimiento
struct Margen {
    tipo: &'static str,
    alert_days: i64,
    fecha: fn(&Auto) -> Option<String>,
}

pub struct AutoService;

impl AutoService {
    /// Lista vehículos con filtros opcionales (búsqueda libre o por estado)
    pub fn listar(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
        estado: Option<&str>,
    ) -> Result<Vec<Auto>, AppError> {
        let term = busqueda.unwrap_or("").trim();
        if !term.is_empty() {
            AutoRepository::buscar(conn, term)
        } else if let Some(estado) = estado.filter(|e| !e.trim().is_empty() && e.trim() != "Todos") {
            AutoRepository::obtener_por_estado(conn, estado.trim())
        } else {
            AutoRepository::obtener_todos(conn)
        }
    }

    /// Obtiene un vehículo por placa
    pub fn obtener(conn: &mut PooledConnection, placa: &str) -> Result<Auto, AppError> {
        AutoRepository::obtener_por_placa(conn, placa)?.ok_or_else(|| {
            AppError::NotFound(format!("No existe un vehículo con placa {placa}"))
        })
    }

    /// Crea un vehículo validando datos y placa única
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        usuario: &str,
        mut datos: AutoDatos,
    ) -> Result<Auto, AppError> {
        normalizar(&mut datos);
        validar(&datos, cfg)?;
        if AutoRepository::existe(conn, &datos.placa)? {
            return Err(AppError::Duplicate(format!(
                "Ya existe un vehículo con placa {}",
                datos.placa
            )));
        }
        AutoRepository::insertar(conn, &datos)?;
        crate::core::audit::log_audit(conn, usuario, "CREAR VEHICULO", &format!("placa={}", datos.placa), "local")?;
        AutoRepository::obtener_por_placa(conn, &datos.placa)?.ok_or_else(|| {
            AppError::Generic("No se pudo recuperar el vehículo recién creado".into())
        })
    }

    /// Actualiza un vehículo por placa
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        usuario: &str,
        placa: &str,
        mut datos: AutoDatos,
    ) -> Result<Auto, AppError> {
        let placa = placa.trim().to_uppercase();
        AutoRepository::obtener_por_placa(conn, &placa)?
            .ok_or_else(|| AppError::NotFound(format!("No existe un vehículo con placa {placa}")))?;
        // La placa no cambia en edición
        datos.placa = placa.clone();
        normalizar(&mut datos);
        validar(&datos, cfg)?;
        AutoRepository::actualizar(conn, &placa, &datos)?;
        crate::core::audit::log_audit(conn, usuario, "ACTUALIZAR VEHICULO", &format!("placa={placa}"), "local")?;
        AutoRepository::obtener_por_placa(conn, &placa)?.ok_or_else(|| {
            AppError::Generic("No se pudo recuperar el vehículo actualizado".into())
        })
    }

    /// Elimina un vehículo por placa
    pub fn eliminar(conn: &mut PooledConnection, usuario: &str, placa: &str) -> Result<(), AppError> {
        AutoRepository::eliminar(conn, placa)?;
        crate::core::audit::log_audit(conn, usuario, "ELIMINAR VEHICULO", &format!("placa={placa}"), "local")
    }

    /// Total de vehículos (dashboard)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        AutoRepository::contar(conn)
    }

    /// Conteo por estado (dashboard)
    pub fn contar_por_estado(
        conn: &mut PooledConnection,
    ) -> Result<Vec<(String, i64)>, AppError> {
        AutoRepository::contar_por_estado(conn)
    }

    /// Alertas de vencimientos próximos (SOAT, tecno-mecánica, extintor, batería, aceite).
    /// Incluye vencidas y las que vencen dentro del margen configurado.
    pub fn alertas_vencimiento(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
    ) -> Result<Vec<AlertaVencimiento>, AppError> {
        let autos = AutoRepository::obtener_todos(conn)?;
        let hoy = Local::now().date_naive();
        let mut alertas = Vec::new();

        let margenes: [Margen; 4] = [
            Margen {
                tipo: "SOAT",
                alert_days: cfg.alert_soat_days,
                fecha: |a| a.vencimiento_soat.clone(),
            },
            Margen {
                tipo: "Tecno-mecánica",
                alert_days: cfg.alert_tecno_mecanica_days,
                fecha: |a| a.vencimiento_tecnico.clone(),
            },
            Margen {
                tipo: "Extintor",
                alert_days: cfg.alert_extintor_days,
                fecha: |a| a.vencimiento_extintor.clone(),
            },
            Margen {
                tipo: "Batería",
                alert_days: ALERTA_BATERIA_DIAS,
                fecha: |a| a.vencimiento_bateria.clone(),
            },
        ];

        for auto in &autos {
            for m in &margenes {
                if let Some(fecha_str) = (m.fecha)(auto) {
                    if let Ok(f) = NaiveDate::parse_from_str(&fecha_str, "%Y-%m-%d") {
                        let dias = f.signed_duration_since(hoy).num_days();
                        if dias <= m.alert_days {
                            let critica = dias <= 3;
                            alertas.push(AlertaVencimiento {
                                placa: auto.placa.clone(),
                                marca: auto.marca.clone(),
                                modelo: auto.modelo.clone(),
                                tipo: m.tipo.to_string(),
                                fecha: Some(fecha_str),
                                dias_restantes: Some(dias),
                                detalle: estado_vencimiento(dias),
                                critica,
                            });
                        }
                    }
                }
            }
            // Cambio de aceite por kilometraje (margen de config.ini business.km_alert_aceite)
            if let Some(proximo) = auto.proximo_aceite {
                let km_restante = proximo - auto.kilometraje as i64;
                if proximo > 0 && km_restante <= cfg.km_alert_aceite {
                    let critica = km_restante <= 0;
                    alertas.push(AlertaVencimiento {
                        placa: auto.placa.clone(),
                        marca: auto.marca.clone(),
                        modelo: auto.modelo.clone(),
                        tipo: "Cambio de aceite".into(),
                        fecha: None,
                        dias_restantes: Some(km_restante),
                        detalle: if critica {
                            format!("Vencido: kilometraje actual {:.0} supera el próximo cambio ({proximo} km)", auto.kilometraje)
                        } else {
                            format!("Cambio de aceite en {km_restante} km (próximo: {proximo} km)")
                        },
                        critica,
                    });
                }
            }
        }

        // Orden: primero vencimientos por fecha (más vencidos primero, luego por
        // cercanía) y al final las alertas por kilometraje (no son días).
        alertas.sort_by_key(|a| {
            let es_km = a.tipo == "Cambio de aceite";
            (es_km, a.dias_restantes.unwrap_or(0))
        });
        Ok(alertas)
    }
}

/// Texto legible del estado del vencimiento
fn estado_vencimiento(dias: i64) -> String {
    match dias.cmp(&0) {
        std::cmp::Ordering::Less => format!("Vencido hace {} días", -dias),
        std::cmp::Ordering::Equal => "Vence hoy".into(),
        std::cmp::Ordering::Greater => format!("Vence en {dias} días"),
    }
}

/// Normaliza campos (trim → mayúsculas, defaults)
fn normalizar(d: &mut AutoDatos) {
    d.placa = mayusculas(&d.placa);
    d.marca = mayusculas(&d.marca);
    d.modelo = mayusculas(&d.modelo);
    d.tipo = mayusculas(&d.tipo);
    d.estado = d.estado.trim().to_string(); // estado con capitalización
    d.costo_fijo_mensual = d.costo_fijo_mensual.trim().replace(',', ".");
    if d.costo_fijo_mensual.is_empty() {
        // vacío → "0.00": evita SQLCODE -303 al enlazar '' a CAST(? AS DECIMAL)
        d.costo_fijo_mensual = "0.00".into();
    }
    if d.tipo.is_empty() {
        d.tipo = "Automóvil".into();
    }
    if d.estado.is_empty() {
        d.estado = "Disponible".into();
    }
    if d.fecha_ingreso.trim().is_empty() {
        d.fecha_ingreso = Local::now().date_naive().format("%Y-%m-%d").to_string();
    }
}

/// Valida los datos del vehículo (espejo de las validaciones de auto_service.py)
fn validar(d: &AutoDatos, cfg: &Arc<AppConfig>) -> Result<(), AppError> {
    if d.placa.is_empty() || d.placa.len() > 20 {
        return Err(AppError::Validation(
            "La placa es obligatoria (máx. 20 caracteres).".into(),
        ));
    }
    if !d.placa.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(AppError::Validation(
            "La placa solo puede contener letras, números y guiones.".into(),
        ));
    }
    if d.marca.is_empty() || d.marca.len() > 80 {
        return Err(AppError::Validation(
            "La marca es obligatoria (máx. 80 caracteres).".into(),
        ));
    }
    if d.modelo.is_empty() || d.modelo.len() > 80 {
        return Err(AppError::Validation(
            "El modelo es obligatorio (máx. 80 caracteres).".into(),
        ));
    }
    if !cfg.estados_auto.is_empty() && !cfg.estados_auto.contains(&d.estado) {
        return Err(AppError::Validation(format!(
            "Estado inválido '{}'. Permitidos: {}",
            d.estado,
            cfg.estados_auto.join(", ")
        )));
    }
    // Fechas
    if NaiveDate::parse_from_str(&d.fecha_ingreso, "%Y-%m-%d").is_err() {
        return Err(AppError::Validation(
            "La fecha de ingreso no es válida (formato AAAA-MM-DD).".into(),
        ));
    }
    for (campo, valor) in [
        ("vencimiento SOAT", &d.vencimiento_soat),
        ("vencimiento técnico", &d.vencimiento_tecnico),
        ("vencimiento extintor", &d.vencimiento_extintor),
        ("vencimiento batería", &d.vencimiento_bateria),
    ] {
        if let Some(v) = valor {
            if !v.is_empty() && NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err() {
                return Err(AppError::Validation(format!(
                    "La fecha de {campo} no es válida."
                )));
            }
        }
    }
    // Monto
    if !d.costo_fijo_mensual.is_empty()
        && Decimal::from_str(&d.costo_fijo_mensual).is_err()
    {
        return Err(AppError::Validation(
            "El costo fijo mensual no es un número válido.".into(),
        ));
    }
    if d.kilometraje < 0.0 {
        return Err(AppError::Validation(
            "El kilometraje no puede ser negativo.".into(),
        ));
    }
    // Sanitización XSS en campos de texto libre
    for (campo, valor) in [
        ("observaciones", &d.observaciones),
        ("ubicación", &d.ubicacion),
        ("propietario", &d.propietario),
        ("no. motor", &d.no_motor),
        ("no. chasis", &d.no_chasis),
    ] {
        if let Some(v) = valor {
            if !v.is_empty() {
                validate_no_xss(v, 2000).map_err(|_| {
                    AppError::Validation(format!("El campo {campo} contiene caracteres no permitidos."))
                })?;
            }
        }
    }
    Ok(())
}

