//! repositories/auditoria.rs — Repositorio de la tabla auditoria
//!
//! Consulta de solo lectura con filtros por usuario, acción y rango de fechas,
//! paginación (ROWS/OFFSET de Firebird) y listado de acciones distintas.

use rsfbclient::{IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

use serde::Serialize;

/// Evento de auditoría (serializable al frontend, camelCase)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditoriaEvento {
    pub id: i64,
    pub usuario: String,
    pub accion: String,
    pub mensaje: Option<String>,
    pub ip: String,
    /// Timestamp como string (Firebird)
    pub fecha: String,
}

/// Filtros de consulta (todos opcionales)
#[derive(Debug, Clone, Default)]
pub struct AuditoriaFiltros {
    pub usuario: Option<String>,
    pub accion: Option<String>,
    /// Rango de fechas inclusivo (AAAA-MM-DD)
    pub fecha_desde: Option<String>,
    pub fecha_hasta: Option<String>,
    /// Búsqueda libre en usuario/acción/mensaje
    pub busqueda: Option<String>,
}

/// Orden de columnas del SELECT de auditoría (debe coincidir con `AuditoriaRow`)
pub const SELECT_COLS: &str = "\
    id, COALESCE(usuario, ''), accion, \
    CAST(mensaje AS VARCHAR(2000)), COALESCE(ip, ''), CAST(fecha AS VARCHAR(30))";

#[allow(clippy::type_complexity)]
pub type AuditoriaRow = (i64, String, String, Option<String>, String, String);

fn from_row(r: AuditoriaRow) -> AuditoriaEvento {
    AuditoriaEvento {
        id: r.0,
        usuario: r.1,
        accion: r.2,
        mensaje: r.3,
        ip: r.4,
        fecha: r.5,
    }
}

pub struct AuditoriaRepository;

impl AuditoriaRepository {
    /// Consulta paginada con filtros opcionales. Devuelve (eventos, total sin paginar).
    pub fn listar(
        conn: &mut PooledConnection,
        filtros: &AuditoriaFiltros,
        limite: i64,
        offset: i64,
    ) -> Result<(Vec<AuditoriaEvento>, i64), AppError> {
        let (where_sql, params) = build_where(filtros);

        // ParamsType no es Clone: se reconstruye para cada consulta.
        let to_params = |v: &[String]| {
            ParamsType::Positional(v.iter().map(|p| p.into_param()).collect())
        };

        // COUNT con los parámetros como ParamsType
        let count_sql = format!("SELECT COUNT(*) FROM auditoria{where_sql}");
        let count: Option<(i64,)> = conn.query_first(&count_sql, to_params(&params))?;

        // Firebird 3+: ROWS ... TO ... ; para paginación con límite opcional.
        // Si limite <= 0 se devuelven todos (sin paginar).
        let limit = limite.max(0);
        let offset = offset.max(0);
        let rows_sql = if limit > 0 {
            // ROWS m TO n es inclusivo: offset+1 .. offset+limit
            let from = offset + 1;
            let to = offset + limit;
            format!(
                "SELECT {SELECT_COLS} FROM auditoria{where_sql} \
                 ORDER BY fecha DESC, id DESC ROWS {from} TO {to}"
            )
        } else {
            format!(
                "SELECT {SELECT_COLS} FROM auditoria{where_sql} \
                 ORDER BY fecha DESC, id DESC"
            )
        };
        let rows: Vec<AuditoriaRow> = conn.query(&rows_sql, to_params(&params))?;

        Ok((
            rows.into_iter().map(from_row).collect(),
            count.map(|(c,)| c).unwrap_or(0),
        ))
    }

    /// Acciones distintas existentes en el log (para el filtro del frontend)
    pub fn acciones(conn: &mut PooledConnection) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> = conn.query(
            "SELECT DISTINCT accion FROM auditoria ORDER BY accion",
            (),
        )?;
        Ok(rows.into_iter().map(|(a,)| a).collect())
    }

    /// Usuarios distintos que han generado eventos (para el filtro del frontend)
    pub fn usuarios(conn: &mut PooledConnection) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> = conn.query(
            "SELECT DISTINCT COALESCE(usuario, '') FROM auditoria \
             ORDER BY 1",
            (),
        )?;
        Ok(rows.into_iter().map(|(u,)| u).collect())
    }
}

/// Construye el WHERE (sin la palabra WHERE) y los parámetros posicionales.
/// Usa `CAST(? AS DATE)` para comparar el timestamp con rangos de fecha.
fn build_where(filtros: &AuditoriaFiltros) -> (String, Vec<String>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(u) = filtros.usuario.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        clauses.push("usuario = ?".into());
        params.push(u.to_string());
    }
    if let Some(a) = filtros.accion.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        clauses.push("accion = ?".into());
        params.push(a.to_string());
    }
    if let Some(d) = filtros.fecha_desde.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        clauses.push("CAST(fecha AS DATE) >= CAST(? AS DATE)".into());
        params.push(d.to_string());
    }
    if let Some(h) = filtros.fecha_hasta.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        clauses.push("CAST(fecha AS DATE) <= CAST(? AS DATE)".into());
        params.push(h.to_string());
    }
    if let Some(b) = filtros.busqueda.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        let like = format!("%{b}%");
        clauses.push(
            "UPPER(COALESCE(usuario, '')) LIKE UPPER(?) \
             OR UPPER(accion) LIKE UPPER(?) \
             OR UPPER(COALESCE(mensaje, '')) LIKE UPPER(?)"
                .into(),
        );
        params.extend([like.clone(), like.clone(), like]);
    }

    if clauses.is_empty() {
        (String::new(), params)
    } else {
        (format!(" WHERE {}", clauses.join(" AND ")), params)
    }
}
