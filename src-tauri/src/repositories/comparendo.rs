//! repositories/comparendo.rs — Repositorio de comparendos (multas de tránsito)
//!
//! Queries explícitas en dialecto Firebird con rsfbclient.
//! - DECIMAL → CAST a VARCHAR (parseo exacto en el servicio/frontend)
//! - DATE/TIME/TIMESTAMP → CAST a VARCHAR

use chrono::{NaiveDate, NaiveTime};
use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

use serde::Serialize;

/// Comparendo completo (serializable al frontend, camelCase)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparendo {
    pub id: i64,
    pub placa: String,
    /// Vehículo (JOIN con autos): marca + modelo
    pub vehiculo: String,
    pub fecha_infraccion: String,
    pub hora_infraccion: String,
    /// Monto como string (decimal exacto)
    pub monto: String,
    pub id_renta: Option<i64>,
    pub id_cliente: Option<i64>,
    pub estado: String,
    pub observaciones: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Datos de entrada para crear/actualizar (validados por el servicio)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ComparendoDatos {
    pub placa: String,
    pub fecha_infraccion: String,
    pub hora_infraccion: String,
    pub monto: String,
    pub id_renta: Option<i64>,
    pub id_cliente: Option<i64>,
    pub estado: String,
    pub observaciones: Option<String>,
}

/// Construye parámetros posicionales de cualquier longitud (tuplas `IntoParams`
/// limitadas a 15 elementos en rsfbclient). Usa `IntoParam` para que las fechas
/// viajen como DATE/TIME (el driver no serializa String a esos tipos).
macro_rules! params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into_param()),+])
    };
}

/// Orden de columnas del SELECT de comparendos (debe coincidir con `ComparendoRow`)
pub const SELECT_COLS: &str = "\
    c.id, c.placa, COALESCE(a.marca || ' ' || a.modelo, ''), \
    CAST(c.fecha_infraccion AS VARCHAR(10)), CAST(c.hora_infraccion AS VARCHAR(13)), \
    CAST(c.monto AS VARCHAR(12)), c.id_renta, c.id_cliente, c.estado, \
    CAST(c.observaciones AS VARCHAR(2000)), \
    CAST(c.created_at AS VARCHAR(30)), CAST(c.updated_at AS VARCHAR(30))";

/// Fila de SELECT de comparendos (tupla — mantener alineada con `SELECT_COLS`)
#[allow(clippy::type_complexity)]
pub type ComparendoRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    Option<i64>,
    Option<i64>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn from_row(r: ComparendoRow) -> Comparendo {
    Comparendo {
        id: r.0,
        placa: r.1,
        vehiculo: r.2,
        fecha_infraccion: r.3,
        hora_infraccion: r.4.split(':').take(2).collect::<Vec<_>>().join(":"),
        monto: r.5,
        id_renta: r.6,
        id_cliente: r.7,
        estado: r.8,
        observaciones: r.9,
        created_at: r.10,
        updated_at: r.11,
    }
}

/// Mapea errores de Firebird a AppError (FK de placa/renta/cliente)
fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("foreign key")
        || lower.contains("not a valid reference")
        || lower.contains("referential")
    {
        AppError::Business(
            "La placa, la renta o el cliente seleccionado no existe. Verifica el registro.".into(),
        )
    } else {
        AppError::Database(msg)
    }
}

pub struct ComparendoRepository;

impl ComparendoRepository {
    /// Lista todos los comparendos (más recientes primero por fecha e id)
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Comparendo>, AppError> {
        let rows: Vec<ComparendoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM comparendos c \
                 LEFT JOIN autos a ON a.placa = c.placa \
                 ORDER BY c.fecha_infraccion DESC, c.id DESC"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Busca comparendos por placa u observaciones (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Comparendo>, AppError> {
        let like = format!("%{}%", term.trim());
        let rows: Vec<ComparendoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM comparendos c \
                 LEFT JOIN autos a ON a.placa = c.placa \
                 WHERE UPPER(c.placa) LIKE UPPER(?) \
                    OR UPPER(COALESCE(c.observaciones, '')) LIKE UPPER(?) \
                 ORDER BY c.fecha_infraccion DESC, c.id DESC"
            ),
            (like.clone(), like),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por placa exacta (historial de un vehículo)
    pub fn obtener_por_placa(conn: &mut PooledConnection, placa: &str) -> Result<Vec<Comparendo>, AppError> {
        let rows: Vec<ComparendoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM comparendos c \
                 LEFT JOIN autos a ON a.placa = c.placa \
                 WHERE c.placa = ? ORDER BY c.fecha_infraccion DESC, c.id DESC"
            ),
            (placa.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por estado exacto (Pendiente / Pagado)
    pub fn obtener_por_estado(conn: &mut PooledConnection, estado: &str) -> Result<Vec<Comparendo>, AppError> {
        let rows: Vec<ComparendoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM comparendos c \
                 LEFT JOIN autos a ON a.placa = c.placa \
                 WHERE c.estado = ? ORDER BY c.fecha_infraccion DESC, c.id DESC"
            ),
            (estado.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Obtiene un comparendo por id
    pub fn obtener_por_id(conn: &mut PooledConnection, id: i64) -> Result<Option<Comparendo>, AppError> {
        let row: Option<ComparendoRow> = conn.query_first(
            &format!(
                "SELECT {SELECT_COLS} FROM comparendos c \
                 LEFT JOIN autos a ON a.placa = c.placa WHERE c.id = ?"
            ),
            (id,),
        )?;
        Ok(row.map(from_row))
    }

    /// Crea un comparendo y devuelve el id nuevo (RETURNING evita races con MAX(id))
    pub fn insertar(conn: &mut PooledConnection, d: &ComparendoDatos) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO comparendos \
                    (placa, fecha_infraccion, hora_infraccion, monto, id_renta, id_cliente, estado, observaciones) \
                 VALUES (?, ?, ?, CAST(? AS DECIMAL(12,2)), ?, ?, ?, ?) RETURNING id",
                params![
                    d.placa.to_string(),
                    parse_fecha(&d.fecha_infraccion)?,
                    parse_hora(&d.hora_infraccion)?,
                    d.monto.to_string(),
                    d.id_renta,
                    d.id_cliente,
                    d.estado.to_string(),
                    opt_str(&d.observaciones),
                ],
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Actualiza un comparendo por id
    pub fn actualizar(conn: &mut PooledConnection, id: i64, d: &ComparendoDatos) -> Result<(), AppError> {
        conn.execute(
            "UPDATE comparendos SET placa = ?, fecha_infraccion = ?, hora_infraccion = ?, \
             monto = CAST(? AS DECIMAL(12,2)), id_renta = ?, id_cliente = ?, estado = ?, \
             observaciones = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
            params![
                d.placa.to_string(),
                parse_fecha(&d.fecha_infraccion)?,
                parse_hora(&d.hora_infraccion)?,
                d.monto.to_string(),
                d.id_renta,
                d.id_cliente,
                d.estado.to_string(),
                opt_str(&d.observaciones),
                id,
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Cambia el estado (Pendiente ↔ Pagado)
    pub fn cambiar_estado(conn: &mut PooledConnection, id: i64, estado: &str) -> Result<(), AppError> {
        conn.execute(
            "UPDATE comparendos SET estado = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            (estado.to_string(), id),
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Elimina un comparendo
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute("DELETE FROM comparendos WHERE id = ?", (id,))
            .map_err(map_fb_error)?;
        Ok(())
    }

    /// Total de comparendos registrados
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM comparendos", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Suma total de todos los comparendos
    pub fn total_general(conn: &mut PooledConnection) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM comparendos",
            (),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de comparendos pendientes (lo que falta por pagar)
    pub fn total_pendiente(conn: &mut PooledConnection) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM comparendos \
             WHERE estado = 'Pendiente'",
            (),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de comparendos agrupada por placa (solo placas con comparendos)
    pub fn total_por_placa(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa, CAST(SUM(monto) AS VARCHAR(12)) FROM comparendos \
             GROUP BY placa ORDER BY SUM(monto) DESC",
            (),
        )?;
        Ok(rows)
    }

    /// Suma de comparendos agrupada por estado
    pub fn total_por_estado(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT estado, CAST(SUM(monto) AS VARCHAR(12)) FROM comparendos \
             GROUP BY estado ORDER BY SUM(monto) DESC",
            (),
        )?;
        Ok(rows)
    }
}

fn opt_str(v: &Option<String>) -> Option<String> {
    v.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Parsea fecha 'AAAA-MM-DD' a NaiveDate (el servicio ya la validó)
fn parse_fecha(v: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("Fecha inválida (formato AAAA-MM-DD).".into()))
}

/// Parsea hora 'HH:MM[:SS]' a NaiveTime (el servicio ya la validó)
fn parse_hora(v: &str) -> Result<NaiveTime, AppError> {
    let h = v.trim();
    let h = if h.len() == 5 { format!("{h}:00") } else { h.to_string() };
    NaiveTime::parse_from_str(&h, "%H:%M:%S")
        .map_err(|_| AppError::Validation("Hora inválida (formato HH:MM).".into()))
}
