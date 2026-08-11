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

/// Responsable del vehículo el día de la infracción — cruce con `rentas`:
/// la renta del mismo vehículo cuyo rango [fecha_recogida, devolución real (o
/// retorno)] contiene la fecha de la infracción. Se calcula en el SELECT (no
/// se persiste); `id_renta`/`id_cliente` de `comparendos` quedan libres.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsableComparendo {
    pub id_renta: i64,
    pub nombre_cliente: String,
    /// Número de contrato (secuencia anual; formatear con `anio_contrato`)
    pub no_contrato: i64,
    pub anio_contrato: i64,
    pub fecha_recogida: String,
    pub fecha_retorno: String,
    pub estado_renta: String,
}

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
    /// Número oficial del comparendo (fuente SIMIT o registro manual)
    pub numero_comparendo: Option<String>,
    pub id_renta: Option<i64>,
    pub id_cliente: Option<i64>,
    pub estado: String,
    pub observaciones: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    /// Quién tenía el vehículo el día de la infracción (cruce con rentas)
    pub responsable: Option<ResponsableComparendo>,
}

/// Datos de entrada para crear/actualizar (validados por el servicio)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ComparendoDatos {
    pub placa: String,
    pub fecha_infraccion: String,
    pub hora_infraccion: String,
    pub monto: String,
    /// Número oficial del comparendo (opcional; usado para deduplicar SIMIT)
    pub numero_comparendo: Option<String>,
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

/// SELECT interno del cruce con rentas (columnas con prefijos y alias).
/// Mantener alineado con `SELECT_COLS_CRUCE_OUTER` y `ComparendoRow`.
pub const SELECT_COLS_CRUCE: &str = "\
    c.id, c.placa, COALESCE(a.marca || ' ' || a.modelo, '') AS vehiculo, \
    CAST(c.fecha_infraccion AS VARCHAR(10)) AS fecha_infraccion, \
    CAST(c.hora_infraccion AS VARCHAR(13)) AS hora_infraccion, \
    CAST(c.monto AS VARCHAR(12)) AS monto, c.numero_comparendo, c.id_renta, c.id_cliente, c.estado, \
    CAST(c.observaciones AS VARCHAR(2000)) AS observaciones, \
    CAST(c.created_at AS VARCHAR(30)) AS created_at, CAST(c.updated_at AS VARCHAR(30)) AS updated_at";

/// SELECT externo del cruce (columnas sin prefijos + campos `resp_*` del
/// responsable). Mantener alineado con `SELECT_COLS_CRUCE` y `ComparendoRow`.
pub const SELECT_COLS_CRUCE_OUTER: &str = "\
    id, placa, vehiculo, fecha_infraccion, hora_infraccion, monto, numero_comparendo, \
    id_renta, id_cliente, estado, observaciones, created_at, updated_at, \
    resp_id, resp_nombre, resp_contrato, resp_anio, resp_recogida, resp_retorno, resp_estado";

/// Construye el SELECT de comparendos con el cruce de responsabilidad: para
/// cada comparendo se busca la renta del mismo vehículo cuyo rango
/// [fecha_recogida, devolución real o retorno] contiene la fecha de la
/// infracción. `ROW_NUMBER()` deduplica por si hubiera más de una (se queda
/// con la de recogida más reciente). `condiciones` se inyecta en el WHERE
/// interno (empieza con AND) y `orden` en el ORDER BY externo.
fn sql_con_responsable(condiciones: &str, orden: &str) -> String {
    format!(
        "SELECT {SELECT_COLS_CRUCE_OUTER} FROM ( \
         SELECT {SELECT_COLS_CRUCE}, \
                resp.id AS resp_id, resp.nombre_cliente AS resp_nombre, \
                resp.no_contrato AS resp_contrato, resp.anio_contrato AS resp_anio, \
                CAST(resp.fecha_recogida AS VARCHAR(10)) AS resp_recogida, \
                CAST(resp.fecha_retorno AS VARCHAR(10)) AS resp_retorno, \
                resp.estado AS resp_estado, \
                ROW_NUMBER() OVER (PARTITION BY c.id ORDER BY resp.fecha_recogida DESC) AS resp_rn \
         FROM comparendos c \
         LEFT JOIN autos a ON a.placa = c.placa \
         LEFT JOIN rentas resp ON resp.placa = c.placa \
              AND c.fecha_infraccion BETWEEN resp.fecha_recogida \
                  AND COALESCE(resp.fecha_devolucion_real, resp.fecha_retorno) \
              AND resp.deleted_at IS NULL AND resp.estado <> 'Cancelada' \
         WHERE c.deleted_at IS NULL {condiciones} \
         ) c2 WHERE c2.resp_rn = 1 {orden}"
    )
}

/// Fila de SELECT de comparendos + cruce (tupla — mantener alineada con
/// `SELECT_COLS_CRUCE`/`SELECT_COLS_CRUCE_OUTER`)
#[allow(clippy::type_complexity)]
pub type ComparendoRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<i64>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    // ── cruce de responsabilidad (resp_*) ──
    Option<i64>,
    Option<String>,
    Option<i64>,
    Option<i64>,
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
        numero_comparendo: r.6,
        id_renta: r.7,
        id_cliente: r.8,
        estado: r.9,
        observaciones: r.10,
        created_at: r.11,
        updated_at: r.12,
        responsable: r.13.map(|id_renta| ResponsableComparendo {
            id_renta,
            nombre_cliente: r.14.unwrap_or_default(),
            no_contrato: r.15.unwrap_or(0),
            anio_contrato: r.16.unwrap_or(0),
            fecha_recogida: r.17.unwrap_or_default(),
            fecha_retorno: r.18.unwrap_or_default(),
            estado_renta: r.19.unwrap_or_default(),
        }),
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
    /// Lista todos los comparendos (más recientes primero por fecha e id),
    /// con el responsable del vehículo el día de la infracción
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Comparendo>, AppError> {
        let sql =
            sql_con_responsable("", "ORDER BY c2.fecha_infraccion DESC, c2.id DESC");
        let rows: Vec<ComparendoRow> = conn.query(&sql, ())?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Busca comparendos por placa u observaciones (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Comparendo>, AppError> {
        let like = format!("%{}%", term.trim());
        let sql = sql_con_responsable(
            "AND (UPPER(c.placa) LIKE UPPER(?) \
             OR UPPER(COALESCE(c.observaciones, '')) LIKE UPPER(?))",
            "ORDER BY c2.fecha_infraccion DESC, c2.id DESC",
        );
        let rows: Vec<ComparendoRow> = conn.query(&sql, (like.clone(), like))?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por placa exacta (historial de un vehículo), con el responsable
    pub fn obtener_por_placa(conn: &mut PooledConnection, placa: &str) -> Result<Vec<Comparendo>, AppError> {
        let sql = sql_con_responsable(
            "AND c.placa = ?",
            "ORDER BY c2.fecha_infraccion DESC, c2.id DESC",
        );
        let rows: Vec<ComparendoRow> = conn.query(&sql, (placa.to_string(),))?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por estado exacto (Pendiente / Pagado), con el responsable
    pub fn obtener_por_estado(conn: &mut PooledConnection, estado: &str) -> Result<Vec<Comparendo>, AppError> {
        let sql = sql_con_responsable(
            "AND c.estado = ?",
            "ORDER BY c2.fecha_infraccion DESC, c2.id DESC",
        );
        let rows: Vec<ComparendoRow> = conn.query(&sql, (estado.to_string(),))?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Obtiene un comparendo por id (con el responsable del día)
    pub fn obtener_por_id(conn: &mut PooledConnection, id: i64) -> Result<Option<Comparendo>, AppError> {
        let sql = sql_con_responsable("AND c.id = ?", "");
        let row: Option<ComparendoRow> = conn.query_first(&sql, (id,))?;
        Ok(row.map(from_row))
    }

    /// Crea un comparendo y devuelve el id nuevo (RETURNING evita races con MAX(id))
    pub fn insertar(conn: &mut PooledConnection, d: &ComparendoDatos) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO comparendos \
                    (placa, fecha_infraccion, hora_infraccion, monto, numero_comparendo, \
                     id_renta, id_cliente, estado, observaciones) \
                 VALUES (?, ?, ?, CAST(? AS DECIMAL(12,2)), ?, ?, ?, ?, ?) RETURNING id",
                params![
                    d.placa.to_string(),
                    parse_fecha(&d.fecha_infraccion)?,
                    parse_hora(&d.hora_infraccion)?,
                    d.monto.to_string(),
                    opt_str(&d.numero_comparendo),
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
             monto = CAST(? AS DECIMAL(12,2)), numero_comparendo = ?, id_renta = ?, \
             id_cliente = ?, estado = ?, observaciones = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
            params![
                d.placa.to_string(),
                parse_fecha(&d.fecha_infraccion)?,
                parse_hora(&d.hora_infraccion)?,
                d.monto.to_string(),
                opt_str(&d.numero_comparendo),
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
    /// Soft-delete: marca el comparendo como borrado (deleted_at).
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute(
            "UPDATE comparendos SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
            (id,),
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// ¿Existe un comparendo activo con ese número oficial? (deduplicación SIMIT)
    pub fn existe_por_numero(conn: &mut PooledConnection, numero: &str) -> Result<bool, AppError> {
        let count: Option<(i64,)> = conn.query_first(
            "SELECT COUNT(*) FROM comparendos \
             WHERE numero_comparendo = ? AND deleted_at IS NULL",
            (numero.trim().to_string(),),
        )?;
        Ok(count.map(|(c,)| c).unwrap_or(0) > 0)
    }

    /// Marca como pagado los comparendos activos con ese número oficial que
    /// aún estén pendientes (el Agente SIMIT converge la BD con el SIMIT).
    pub fn marcar_pagado_por_numero(conn: &mut PooledConnection, numero: &str) -> Result<(), AppError> {
        conn.execute(
            "UPDATE comparendos SET estado = 'Pagado', updated_at = CURRENT_TIMESTAMP \
             WHERE numero_comparendo = ? AND estado <> 'Pagado' AND deleted_at IS NULL",
            (numero.trim().to_string(),),
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// ¿Existe un comparendo activo con la misma placa, fecha y monto?
    /// Fallback de deduplicación cuando no hay número oficial.
    pub fn existe_duplicado(
        conn: &mut PooledConnection,
        placa: &str,
        fecha: &str,
        monto: &str,
    ) -> Result<bool, AppError> {
        let count: Option<(i64,)> = conn.query_first(
            "SELECT COUNT(*) FROM comparendos \
             WHERE placa = ? AND fecha_infraccion = ? AND monto = CAST(? AS DECIMAL(12,2)) \
               AND deleted_at IS NULL",
            (placa.to_string(), parse_fecha(fecha)?, monto.to_string()),
        )?;
        Ok(count.map(|(c,)| c).unwrap_or(0) > 0)
    }

    /// Total de comparendos registrados
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM comparendos WHERE deleted_at IS NULL", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Suma total de todos los comparendos
    pub fn total_general(conn: &mut PooledConnection) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM comparendos WHERE deleted_at IS NULL",
            (),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de comparendos pendientes (lo que falta por pagar)
    pub fn total_pendiente(conn: &mut PooledConnection) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM comparendos \
             WHERE estado = 'Pendiente' AND deleted_at IS NULL",
            (),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de comparendos agrupada por placa (solo placas con comparendos)
    pub fn total_por_placa(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa, CAST(SUM(monto) AS VARCHAR(12)) FROM comparendos \
             WHERE deleted_at IS NULL GROUP BY placa ORDER BY SUM(monto) DESC",
            (),
        )?;
        Ok(rows)
    }

    /// Suma de comparendos agrupada por estado
    pub fn total_por_estado(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT estado, CAST(SUM(monto) AS VARCHAR(12)) FROM comparendos \
             WHERE deleted_at IS NULL GROUP BY estado ORDER BY SUM(monto) DESC",
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
