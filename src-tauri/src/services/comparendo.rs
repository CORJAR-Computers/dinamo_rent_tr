//! services/comparendo.rs — Lógica de negocio de comparendos (multas de tránsito)
//!
//! Valida placa/fecha/hora/monto/estado, gestiona el marcado de pago y calcula
//! totales (general, pendiente, por placa y por estado).

use std::sync::Arc;

use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr as _;
use serde::Serialize;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::validators::{validate_no_xss, mayusculas};
use crate::core::PooledConnection;
use crate::repositories::comparendo::{Comparendo, ComparendoDatos, ComparendoRepository};
use crate::repositories::auto::AutoRepository;

/// Estados válidos de un comparendo
pub const ESTADOS_COMPARENDO: [&str; 2] = ["Pendiente", "Pagado"];

/// Días sin que el SIMIT confirme un comparendo para considerarlo «no
/// confirmado» (posible pagado/eliminado en el portal sin avisar a la BD).
/// El Agente corre cada 2 h, así que 3 días ≈ 36 corridas sin verlo.
pub const DIAS_SIN_CONFIRMAR_SIMIT: i64 = 3;

/// Total por placa o estado (para el frontend)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalComparendo {
    pub clave: String,
    pub total: String,
}

/// Resumen de totales para la página de comparendos
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalesComparendos {
    /// Suma de todos los comparendos
    pub total_general: String,
    /// Suma de los comparendos pendientes
    pub total_pendiente: String,
    pub por_placa: Vec<TotalComparendo>,
    pub por_estado: Vec<TotalComparendo>,
}

pub struct ComparendoService;

impl ComparendoService {
    /// Lista comparendos con filtros opcionales (búsqueda libre, placa o
    /// estado). Si `no_confirmados` es true devuelve solo los de origen SIMIT
    /// que el SIMIT dejó de confirmar (ultimo_visto_simit anterior a
    /// `DIAS_SIN_CONFIRMAR_SIMIT` o nunca confirmado).
    pub fn listar(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
        placa: Option<&str>,
        estado: Option<&str>,
        no_confirmados: bool,
    ) -> Result<Vec<Comparendo>, AppError> {
        if no_confirmados {
            let corte = (Local::now().date_naive() - chrono::Duration::days(DIAS_SIN_CONFIRMAR_SIMIT))
                .format("%Y-%m-%d")
                .to_string();
            return ComparendoRepository::obtener_no_confirmados_simit(conn, &corte);
        }
        let term = busqueda.unwrap_or("").trim();
        let placa = placa.unwrap_or("").trim();
        let estado = estado.unwrap_or("").trim();
        if !term.is_empty() {
            ComparendoRepository::buscar(conn, term)
        } else if !placa.is_empty() {
            ComparendoRepository::obtener_por_placa(conn, placa)
        } else if !estado.is_empty() && estado != "Todos" {
            ComparendoRepository::obtener_por_estado(conn, estado)
        } else {
            ComparendoRepository::obtener_todos(conn)
        }
    }

    /// Obtiene un comparendo por id
    pub fn obtener(conn: &mut PooledConnection, id: i64) -> Result<Comparendo, AppError> {
        ComparendoRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe el comparendo #{id}")))
    }

    /// Crea un comparendo
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        mut datos: ComparendoDatos,
    ) -> Result<Comparendo, AppError> {
        normalizar(&mut datos);
        validar(conn, &datos, cfg)?;
        // Atribución automática: si no se indicó renta/cliente, se resuelve qué
        // renta cubría el vehículo el día de la infracción y se persiste el
        // vínculo (misma lógica que el Agente SIMIT al importar).
        if datos.id_renta.is_none() && datos.id_cliente.is_none() {
            if let Some((id_renta, id_cliente)) =
                ComparendoRepository::renta_del_dia(conn, &datos.placa, &datos.fecha_infraccion)?
            {
                datos.id_renta = Some(id_renta);
                datos.id_cliente = id_cliente;
            }
        }
        let id = ComparendoRepository::insertar(conn, &datos)?;
        Self::obtener(conn, id)
    }

    /// Actualiza un comparendo por id
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        id: i64,
        mut datos: ComparendoDatos,
    ) -> Result<Comparendo, AppError> {
        Self::obtener(conn, id)?;
        normalizar(&mut datos);
        validar(conn, &datos, cfg)?;
        ComparendoRepository::actualizar(conn, id, &datos)?;
        Self::obtener(conn, id)
    }

    /// Marca un comparendo como pagado (no se puede deshacer desde este flujo)
    pub fn marcar_pagado(conn: &mut PooledConnection, id: i64) -> Result<Comparendo, AppError> {
        let actual = Self::obtener(conn, id)?;
        if actual.estado == "Pagado" {
            return Ok(actual);
        }
        ComparendoRepository::cambiar_estado(conn, id, "Pagado")?;
        Self::obtener(conn, id)
    }

    /// Elimina un comparendo
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        Self::obtener(conn, id)?;
        ComparendoRepository::eliminar(conn, id)
    }

    /// Total de comparendos (dashboard)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        ComparendoRepository::contar(conn)
    }

    /// Totales general, pendiente, por placa y por estado (página de comparendos)
    pub fn totales(conn: &mut PooledConnection) -> Result<TotalesComparendos, AppError> {
        Ok(TotalesComparendos {
            total_general: ComparendoRepository::total_general(conn)?,
            total_pendiente: ComparendoRepository::total_pendiente(conn)?,
            por_placa: ComparendoRepository::total_por_placa(conn)?
                .into_iter()
                .map(|(clave, total)| TotalComparendo { clave, total })
                .collect(),
            por_estado: ComparendoRepository::total_por_estado(conn)?
                .into_iter()
                .map(|(clave, total)| TotalComparendo { clave, total })
                .collect(),
        })
    }
}

/// Normaliza campos (trim → mayúsculas, monto con coma → punto)
fn normalizar(d: &mut ComparendoDatos) {
    d.placa = mayusculas(&d.placa);
    d.fecha_infraccion = d.fecha_infraccion.trim().to_string(); // fecha formato
    d.hora_infraccion = d.hora_infraccion.trim().to_string(); // hora formato
    d.monto = d.monto.trim().replace(',', ".");
    d.estado = d.estado.trim().to_string(); // estado con capitalización
    d.observaciones = d
        .observaciones
        .as_ref()
        .map(|s| mayusculas(s))
        .filter(|s| !s.is_empty());
}

/// Valida los datos del comparendo (requiere conn para verificar la placa)
fn validar(conn: &mut PooledConnection, d: &ComparendoDatos, cfg: &Arc<AppConfig>) -> Result<(), AppError> {
    // Placa: obligatoria y existente en autos
    if d.placa.is_empty() {
        return Err(AppError::Validation("La placa es obligatoria.".into()));
    }
    if d.placa.len() > 20 {
        return Err(AppError::Validation("La placa no puede superar 20 caracteres.".into()));
    }
    validate_no_xss(&d.placa, 20).map_err(|_| {
        AppError::Validation("La placa contiene caracteres no permitidos.".into())
    })?;
    let existe = AutoRepository::obtener_por_placa(conn, &d.placa)?.is_some();
    if !existe {
        return Err(AppError::Business(
            "La placa seleccionada no existe. Verifica que el vehículo esté registrado.".into(),
        ));
    }

    // Fecha y hora de la infracción
    NaiveDate::parse_from_str(&d.fecha_infraccion, "%Y-%m-%d").map_err(|_| {
        AppError::Validation("La fecha de la infracción no es válida (formato AAAA-MM-DD).".into())
    })?;
    if !es_hora_valida(&d.hora_infraccion) {
        return Err(AppError::Validation(
            "La hora de la infracción no es válida (formato HH:MM).".into(),
        ));
    }

    // Monto
    let monto = Decimal::from_str(&d.monto).unwrap_or_else(|_| Decimal::from(-1));
    if monto < Decimal::ZERO {
        return Err(AppError::Validation("El monto no es un número válido.".into()));
    }
    if monto == Decimal::ZERO {
        return Err(AppError::Validation("El monto debe ser mayor que cero.".into()));
    }
    if monto > Decimal::from(9_999_999_999i64) / Decimal::from(100) {
        return Err(AppError::Validation(
            "El monto es demasiado grande (máx. 99.999.999,99).".into(),
        ));
    }

    // Estado
    if !ESTADOS_COMPARENDO.contains(&d.estado.as_str()) {
        return Err(AppError::Validation(format!(
            "Estado inválido '{}'. Permitidos: Pendiente, Pagado.",
            d.estado
        )));
    }

    // Observaciones (texto libre)
    if let Some(v) = &d.observaciones {
        if !v.is_empty() && validate_no_xss(v, 2000).is_err() {
            return Err(AppError::Validation(
                "Las observaciones contienen caracteres no permitidos.".into(),
            ));
        }
    }

    let _ = cfg;
    Ok(())
}

/// ¿Hora válida en formato HH:MM o HH:MM:SS?
fn es_hora_valida(h: &str) -> bool {
    let parts: Vec<&str> = h.split(':').collect();
    if parts.len() != 2 && parts.len() != 3 {
        return false;
    }
    for p in &parts {
        if p.len() != 2 || !p.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    let hh: u32 = parts[0].parse().unwrap_or(99);
    let mm: u32 = parts[1].parse().unwrap_or(99);
    if hh > 23 || mm > 59 {
        return false;
    }
    if parts.len() == 3 {
        let ss: u32 = parts[2].parse().unwrap_or(99);
        if ss > 59 {
            return false;
        }
    }
    true
}
