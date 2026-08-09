//! repositories/gasto.rs — Repositorio de gastos (caja menor)
//!
//! Queries explícitas en dialecto Firebird con rsfbclient.
//! - DECIMAL → CAST a VARCHAR (parseo exacto en el servicio/frontend)
//! - DATE/TIMESTAMP → CAST a VARCHAR

use chrono::NaiveDate;
use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

use serde::Serialize;

/// Gasto completo (serializable al frontend, camelCase)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gasto {
    pub id: i64,
    pub placa: Option<String>,
    pub fecha: String,
    pub categoria: String,
    pub descripcion: String,
    /// Monto como string (decimal exacto)
    pub monto: String,
    pub comprobante: Option<String>,
    pub usuario: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Datos de entrada para crear/actualizar (validados por el servicio)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GastoDatos {
    pub placa: Option<String>,
    pub fecha: String,
    pub categoria: String,
    pub descripcion: String,
    pub monto: String,
    pub comprobante: Option<String>,
}

/// Construye parámetros posicionales de cualquier longitud (tuplas `IntoParams`
/// limitadas a 15 elementos en rsfbclient). Usa `IntoParam` para que las fechas
/// viajen como TIMESTAMP (el driver no serializa String a DATE).
macro_rules! params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into_param()),+])
    };
}

/// Orden de columnas del SELECT de gastos (debe coincidir con `GastoRow`)
pub const SELECT_COLS: &str = "\
    id, placa, CAST(fecha AS VARCHAR(10)), categoria, descripcion, \
    CAST(monto AS VARCHAR(12)), comprobante, usuario, \
    CAST(created_at AS VARCHAR(30)), CAST(updated_at AS VARCHAR(30))";

/// Fila de SELECT de gastos (tupla — mantener alineada con `SELECT_COLS`)
#[allow(clippy::type_complexity)]
pub type GastoRow = (
    i64,
    Option<String>,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn from_row(r: GastoRow) -> Gasto {
    Gasto {
        id: r.0,
        placa: r.1,
        fecha: r.2,
        categoria: r.3,
        descripcion: r.4,
        monto: r.5,
        comprobante: r.6,
        usuario: r.7,
        created_at: r.8,
        updated_at: r.9,
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

pub struct GastoRepository;

impl GastoRepository {
    /// Lista todos los gastos (más recientes primero por fecha e id)
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Gasto>, AppError> {
        let rows: Vec<GastoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM gastos WHERE deleted_at IS NULL ORDER BY fecha DESC, id DESC"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Busca gastos por placa, descripción, categoría o comprobante (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Gasto>, AppError> {
        let like = format!("%{}%", term.trim());
        let rows: Vec<GastoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM gastos \
                 WHERE deleted_at IS NULL \
                   AND (UPPER(COALESCE(placa, '')) LIKE UPPER(?) \
                        OR UPPER(descripcion) LIKE UPPER(?) \
                        OR UPPER(categoria) LIKE UPPER(?) \
                        OR UPPER(COALESCE(comprobante, '')) LIKE UPPER(?)) \
                 ORDER BY fecha DESC, id DESC"
            ),
            (like.clone(), like.clone(), like.clone(), like),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por placa exacta (totales y detalle de un vehículo)
    pub fn obtener_por_placa(conn: &mut PooledConnection, placa: &str) -> Result<Vec<Gasto>, AppError> {
        let rows: Vec<GastoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM gastos WHERE placa = ? AND deleted_at IS NULL \
                 ORDER BY fecha DESC, id DESC"
            ),
            (placa.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por categoría exacta
    pub fn obtener_por_categoria(conn: &mut PooledConnection, categoria: &str) -> Result<Vec<Gasto>, AppError> {
        let rows: Vec<GastoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM gastos WHERE categoria = ? AND deleted_at IS NULL \
                 ORDER BY fecha DESC, id DESC"
            ),
            (categoria.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por placa Y categoría combinadas (filtros simultáneos de la UI)
    pub fn obtener_por_placa_categoria(
        conn: &mut PooledConnection,
        placa: &str,
        categoria: &str,
    ) -> Result<Vec<Gasto>, AppError> {
        let rows: Vec<GastoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM gastos WHERE placa = ? AND categoria = ? AND deleted_at IS NULL \
                 ORDER BY fecha DESC, id DESC"
            ),
            (placa.to_string(), categoria.to_string()),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Gastos recientes (los últimos `limit`, por fecha de creación)
    pub fn obtener_recientes(conn: &mut PooledConnection, limit: i64) -> Result<Vec<Gasto>, AppError> {
        let rows: Vec<GastoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM gastos WHERE deleted_at IS NULL \
                 ORDER BY fecha DESC, id DESC ROWS {limit}"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Obtiene un gasto por id
    pub fn obtener_por_id(conn: &mut PooledConnection, id: i64) -> Result<Option<Gasto>, AppError> {
        let row: Option<GastoRow> = conn.query_first(
            &format!("SELECT {SELECT_COLS} FROM gastos WHERE id = ? AND deleted_at IS NULL"),
            (id,),
        )?;
        Ok(row.map(from_row))
    }

    /// Crea un gasto y devuelve el id nuevo (RETURNING evita races con MAX(id)).
    /// `usuario` registra al actor de la sesión para trazabilidad.
    pub fn insertar(conn: &mut PooledConnection, d: &GastoDatos, usuario: &str) -> Result<i64, AppError> {
        let usuario = usuario.trim();
        let usuario = if usuario.is_empty() { "Sistema" } else { usuario };
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO gastos (placa, fecha, categoria, descripcion, monto, comprobante, usuario) \
                 VALUES (?, ?, ?, ?, CAST(? AS DECIMAL(12,2)), ?, ?) RETURNING id",
                params![
                    opt_str(&d.placa),
                    parse_fecha(&d.fecha)?,
                    d.categoria.to_string(),
                    d.descripcion.to_string(),
                    d.monto.to_string(),
                    opt_str(&d.comprobante),
                    usuario.to_string(),
                ],
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Actualiza un gasto por id
    pub fn actualizar(conn: &mut PooledConnection, id: i64, d: &GastoDatos) -> Result<(), AppError> {
        conn.execute(
            "UPDATE gastos SET placa = ?, fecha = ?, categoria = ?, descripcion = ?, \
             monto = CAST(? AS DECIMAL(12,2)), comprobante = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
            params![
                opt_str(&d.placa),
                parse_fecha(&d.fecha)?,
                d.categoria.to_string(),
                d.descripcion.to_string(),
                d.monto.to_string(),
                opt_str(&d.comprobante),
                id,
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Elimina un gasto
    /// Soft-delete: marca el gasto como borrado (deleted_at).
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute(
            "UPDATE gastos SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
            (id,),
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Total de gastos registrados
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM gastos WHERE deleted_at IS NULL", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Suma total de todos los gastos
    pub fn total_general(conn: &mut PooledConnection) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM gastos WHERE deleted_at IS NULL",
            (),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de gastos del mes calendario actual (primer día del mes → hoy)
    pub fn total_mes(conn: &mut PooledConnection) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM gastos \
             WHERE fecha >= DATEADD(DAY, 1 - EXTRACT(DAY FROM CURRENT_DATE), CURRENT_DATE) \
               AND deleted_at IS NULL",
            (),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de gastos agrupada por placa (solo placas con gastos)
    pub fn total_por_placa(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(Option<String>, String)> = conn.query(
            "SELECT placa, CAST(SUM(monto) AS VARCHAR(12)) FROM gastos \
             WHERE placa IS NOT NULL AND deleted_at IS NULL GROUP BY placa ORDER BY SUM(monto) DESC",
            (),
        )?;
        Ok(rows
            .into_iter()
            .filter_map(|(placa, total)| placa.map(|p| (p, total)))
            .collect())
    }

    /// Suma de gastos agrupada por categoría
    pub fn total_por_categoria(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT categoria, CAST(SUM(monto) AS VARCHAR(12)) FROM gastos \
             WHERE deleted_at IS NULL GROUP BY categoria ORDER BY SUM(monto) DESC",
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
