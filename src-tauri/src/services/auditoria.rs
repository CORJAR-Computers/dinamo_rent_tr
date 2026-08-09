//! services/auditoria.rs — Lógica de negocio de la vista de auditoría
//!
//! Validación de filtros (fechas, límites) y consulta paginada de eventos.

use chrono::NaiveDate;
use serde::Serialize;

use crate::core::error::AppError;
use crate::core::PooledConnection;
use crate::repositories::auditoria::{AuditoriaEvento, AuditoriaFiltros, AuditoriaRepository};

/// Resultado paginado para el frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditoriaResultado {
    pub eventos: Vec<AuditoriaEvento>,
    /// Total de eventos que coinciden con los filtros (sin paginar)
    pub total: i64,
    pub pagina: i64,
    pub por_pagina: i64,
}

pub struct AuditoriaService;

impl AuditoriaService {
    /// Consulta paginada con filtros opcionales (usuario, acción, rango de fechas, búsqueda)
    pub fn listar(
        conn: &mut PooledConnection,
        filtros: AuditoriaFiltros,
        pagina: Option<i64>,
        por_pagina: Option<i64>,
    ) -> Result<AuditoriaResultado, AppError> {
        let page = pagina.unwrap_or(1).max(1);
        let per = por_pagina.unwrap_or(50).clamp(1, 500);
        validar_filtros(&filtros)?;

        let offset = (page - 1) * per;
        let (eventos, total) = AuditoriaRepository::listar(conn, &filtros, per, offset)?;
        Ok(AuditoriaResultado {
            eventos,
            total,
            pagina: page,
            por_pagina: per,
        })
    }

    /// Acciones distintas disponibles para el filtro
    pub fn acciones(conn: &mut PooledConnection) -> Result<Vec<String>, AppError> {
        AuditoriaRepository::acciones(conn)
    }

    /// Usuarios distintos disponibles para el filtro
    pub fn usuarios(conn: &mut PooledConnection) -> Result<Vec<String>, AppError> {
        AuditoriaRepository::usuarios(conn)
    }
}

/// Valida los filtros de fecha (formato AAAA-MM-DD) y relaciones básicas
fn validar_filtros(f: &AuditoriaFiltros) -> Result<(), AppError> {
    for (campo, valor) in [
        ("fecha desde", f.fecha_desde.as_deref()),
        ("fecha hasta", f.fecha_hasta.as_deref()),
    ] {
        if let Some(v) = valor.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err() {
                return Err(AppError::Validation(format!(
                    "La {campo} no es válida (formato AAAA-MM-DD)."
                )));
            }
        }
    }
    // fecha_desde <= fecha_hasta (cuando ambas existen)
    if let (Some(d), Some(h)) = (f.fecha_desde.as_deref(), f.fecha_hasta.as_deref()) {
        let d = d.trim();
        let h = h.trim();
        if !d.is_empty() && !h.is_empty() {
            let desde = NaiveDate::parse_from_str(d, "%Y-%m-%d").ok();
            let hasta = NaiveDate::parse_from_str(h, "%Y-%m-%d").ok();
            if let (Some(desde), Some(hasta)) = (desde, hasta) {
                if desde > hasta {
                    return Err(AppError::Validation(
                        "La fecha desde no puede ser posterior a la fecha hasta.".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}
