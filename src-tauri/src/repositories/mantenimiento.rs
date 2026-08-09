//! repositories/mantenimiento.rs — Repositorio de mantenimiento de vehículos
//!
//! Queries explícitas en dialecto Firebird con rsfbclient.
//! - DECIMAL → CAST a VARCHAR (parseo exacto en el servicio/frontend)
//! - DATE/TIMESTAMP → CAST a VARCHAR
//! - JOIN con autos para mostrar marca/modelo en la UI

use chrono::NaiveDate;
use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

use serde::Serialize;

/// Registro de mantenimiento completo (serializable al frontend, camelCase)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mantenimiento {
    pub id: i64,
    pub placa: String,
    /// Marca + modelo del vehículo (JOIN con autos, solo para la UI)
    pub vehiculo: String,
    pub tipo: String,
    pub fecha: String,
    pub descripcion: Option<String>,
    pub observaciones: Option<String>,
    /// Costo como string (decimal exacto)
    pub costo: String,
    pub km_proximo_cambio_aceite: Option<i64>,
    /// Total del mantenimiento (backend: = costo)
    pub total: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Datos de entrada para crear/actualizar (validados por el servicio)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MantenimientoDatos {
    pub placa: String,
    pub tipo: String,
    pub fecha: String,
    pub descripcion: Option<String>,
    pub observaciones: Option<String>,
    pub costo: String,
    pub km_proximo_cambio_aceite: Option<i64>,
}

/// Construye parámetros posicionales de cualquier longitud (tuplas `IntoParams`
/// limitadas a 15 elementos en rsfbclient).
macro_rules! params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into_param()),+])
    };
}

/// Orden de columnas del SELECT de mantenimiento (debe coincidir con `MantenimientoRow`)
pub const SELECT_COLS: &str = "\
    m.id, m.placa, COALESCE(a.marca || ' ' || a.modelo, ''), m.pieza_varias_tipo, \
    CAST(m.pieza_varias_fecha AS VARCHAR(10)), \
    CAST(m.pieza_varias_desc AS VARCHAR(250)), CAST(m.pieza_varias_obs AS VARCHAR(2000)), \
    CAST(m.cost_varios AS VARCHAR(12)), m.km_proximo_cambio_aceite, \
    CAST(m.total_mantenimiento AS VARCHAR(12)), \
    CAST(m.created_at AS VARCHAR(30)), CAST(m.updated_at AS VARCHAR(30))";

/// Fila de SELECT de mantenimiento (tupla — mantener alineada con `SELECT_COLS`)
#[allow(clippy::type_complexity)]
pub type MantenimientoRow = (
    i64,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<i64>,
    String,
    Option<String>,
    Option<String>,
);

fn from_row(r: MantenimientoRow) -> Mantenimiento {
    Mantenimiento {
        id: r.0,
        placa: r.1,
        vehiculo: r.2,
        tipo: r.3,
        fecha: r.4,
        descripcion: r.5,
        observaciones: r.6,
        costo: r.7,
        km_proximo_cambio_aceite: r.8,
        total: r.9,
        created_at: r.10,
        updated_at: r.11,
    }
}

/// Mapea errores de Firebird a AppError (FK de placa)
fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("foreign key")
        || lower.contains("not a valid reference")
        || lower.contains("referential")
    {
        AppError::Business(
            "La placa seleccionada no existe. Verifica que el vehículo esté registrado.".into(),
        )
    } else {
        AppError::Database(msg)
    }
}

pub struct MantenimientoRepository;

impl MantenimientoRepository {
    /// Lista todos los mantenimientos (más recientes primero)
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Mantenimiento>, AppError> {
        let rows: Vec<MantenimientoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM mantenimiento_vehiculos m \
                 LEFT JOIN autos a ON a.placa = m.placa \
                 ORDER BY m.pieza_varias_fecha DESC, m.id DESC"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Historial por placa exacta
    pub fn obtener_por_placa(conn: &mut PooledConnection, placa: &str) -> Result<Vec<Mantenimiento>, AppError> {
        let rows: Vec<MantenimientoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM mantenimiento_vehiculos m \
                 LEFT JOIN autos a ON a.placa = m.placa \
                 WHERE m.placa = ? \
                 ORDER BY m.pieza_varias_fecha DESC, m.id DESC"
            ),
            (placa.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por tipo de mantenimiento exacto
    pub fn obtener_por_tipo(conn: &mut PooledConnection, tipo: &str) -> Result<Vec<Mantenimiento>, AppError> {
        let rows: Vec<MantenimientoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM mantenimiento_vehiculos m \
                 LEFT JOIN autos a ON a.placa = m.placa \
                 WHERE m.pieza_varias_tipo = ? \
                 ORDER BY m.pieza_varias_fecha DESC, m.id DESC"
            ),
            (tipo.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por placa Y tipo combinados (filtros simultáneos de la UI)
    pub fn obtener_por_placa_tipo(
        conn: &mut PooledConnection,
        placa: &str,
        tipo: &str,
    ) -> Result<Vec<Mantenimiento>, AppError> {
        let rows: Vec<MantenimientoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM mantenimiento_vehiculos m \
                 LEFT JOIN autos a ON a.placa = m.placa \
                 WHERE m.placa = ? AND m.pieza_varias_tipo = ? \
                 ORDER BY m.pieza_varias_fecha DESC, m.id DESC"
            ),
            (placa.to_string(), tipo.to_string()),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Último km de cambio de aceite programado para una placa (excluye km 0/NULL).
    /// Lo usa el servicio al eliminar un mantenimiento para recalcular
    /// `autos.proximo_aceite` desde el historial restante.
    pub fn ultimo_km_aceite(conn: &mut PooledConnection, placa: &str) -> Result<Option<i64>, AppError> {
        let row: Option<(Option<i64>,)> = conn.query_first(
            "SELECT first 1 km_proximo_cambio_aceite FROM mantenimiento_vehiculos \
             WHERE placa = ? AND pieza_varias_tipo = ? AND km_proximo_cambio_aceite > 0 \
             ORDER BY pieza_varias_fecha DESC, id DESC",
            (placa.to_string(), "Cambio Aceite".to_string()),
        )?;
        Ok(row.and_then(|(k,)| k))
    }

    /// Busca por placa, tipo o descripción (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Mantenimiento>, AppError> {
        let like = format!("%{}%", term.trim());
        let rows: Vec<MantenimientoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM mantenimiento_vehiculos m \
                 LEFT JOIN autos a ON a.placa = m.placa \
                 WHERE UPPER(m.placa) LIKE UPPER(?) OR UPPER(m.pieza_varias_tipo) LIKE UPPER(?) \
                    OR UPPER(COALESCE(m.pieza_varias_desc, '')) LIKE UPPER(?) \
                 ORDER BY m.pieza_varias_fecha DESC, m.id DESC"
            ),
            (like.clone(), like.clone(), like),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Mantenimientos recientes (los últimos `limit`)
    pub fn obtener_recientes(conn: &mut PooledConnection, limit: i64) -> Result<Vec<Mantenimiento>, AppError> {
        let rows: Vec<MantenimientoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM mantenimiento_vehiculos m \
                 LEFT JOIN autos a ON a.placa = m.placa \
                 ORDER BY m.pieza_varias_fecha DESC, m.id DESC ROWS {limit}"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Obtiene un mantenimiento por id
    pub fn obtener_por_id(conn: &mut PooledConnection, id: i64) -> Result<Option<Mantenimiento>, AppError> {
        let row: Option<MantenimientoRow> = conn.query_first(
            &format!(
                "SELECT {SELECT_COLS} FROM mantenimiento_vehiculos m \
                 LEFT JOIN autos a ON a.placa = m.placa WHERE m.id = ?"
            ),
            (id,),
        )?;
        Ok(row.map(from_row))
    }

    /// Crea un mantenimiento y devuelve el id nuevo (RETURNING evita races con MAX(id))
    pub fn insertar(conn: &mut PooledConnection, d: &MantenimientoDatos) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO mantenimiento_vehiculos (\
                    placa, pieza_varias_tipo, pieza_varias_fecha, pieza_varias_desc, \
                    pieza_varias_obs, cost_varios, km_proximo_cambio_aceite, total_mantenimiento \
                 ) VALUES (?, ?, ?, ?, ?, CAST(? AS DECIMAL(12,2)), ?, CAST(? AS DECIMAL(12,2))) RETURNING id",
                params![
                    d.placa.to_string(),
                    d.tipo.to_string(),
                    parse_fecha(&d.fecha)?,
                    opt_str(&d.descripcion),
                    opt_str(&d.observaciones),
                    d.costo.to_string(),
                    d.km_proximo_cambio_aceite,
                    d.costo.to_string(),
                ],
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Actualiza un mantenimiento por id
    pub fn actualizar(conn: &mut PooledConnection, id: i64, d: &MantenimientoDatos) -> Result<(), AppError> {
        conn.execute(
            "UPDATE mantenimiento_vehiculos SET \
                placa = ?, pieza_varias_tipo = ?, pieza_varias_fecha = ?, pieza_varias_desc = ?, \
                pieza_varias_obs = ?, cost_varios = CAST(? AS DECIMAL(12,2)), \
                km_proximo_cambio_aceite = ?, total_mantenimiento = CAST(? AS DECIMAL(12,2)), \
                updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
            params![
                d.placa.to_string(),
                d.tipo.to_string(),
                parse_fecha(&d.fecha)?,
                opt_str(&d.descripcion),
                opt_str(&d.observaciones),
                d.costo.to_string(),
                d.km_proximo_cambio_aceite,
                d.costo.to_string(),
                id,
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Elimina un mantenimiento
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute("DELETE FROM mantenimiento_vehiculos WHERE id = ?", (id,))
            .map_err(map_fb_error)?;
        Ok(())
    }

    /// Total de mantenimientos registrados
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM mantenimiento_vehiculos", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Suma total de todos los mantenimientos
    pub fn total_general(conn: &mut PooledConnection) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(total_mantenimiento), 0) AS VARCHAR(12)) FROM mantenimiento_vehiculos",
            (),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma agrupada por placa (solo placas con mantenimientos)
    pub fn total_por_placa(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa, CAST(SUM(total_mantenimiento) AS VARCHAR(12)) FROM mantenimiento_vehiculos \
             GROUP BY placa ORDER BY SUM(total_mantenimiento) DESC",
            (),
        )?;
        Ok(rows)
    }

    /// Suma agrupada por tipo de mantenimiento
    pub fn total_por_tipo(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT pieza_varias_tipo, CAST(SUM(total_mantenimiento) AS VARCHAR(12)) \
             FROM mantenimiento_vehiculos GROUP BY pieza_varias_tipo ORDER BY SUM(total_mantenimiento) DESC",
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
