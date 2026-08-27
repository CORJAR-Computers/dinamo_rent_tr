//! repositories/reserva.rs — Repositorio de reservas (puerto de reserva_repository_sa.py)
//!
//! Queries explícitas en dialecto Firebird con rsfbclient.
//! - DECIMAL → CAST a VARCHAR (parseo exacto en el servicio/frontend)
//! - DATE/TIME/TIMESTAMP → CAST a VARCHAR
//!
//! > **TODO (Bloque 4 / TAREA 4.2)**: este repositorio aún define helpers
//! > locales (`map_fb_error`, `opt_str`, `params!`, ...) duplicados con
//! > `crate::core::repository`. Migración pendiente — ver
//! > `src/core/repository.rs` para el módulo centralizado.

use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::repository::{map_fb_error_fk, opt_str, params, parse_fecha, parse_hora_opt};
use crate::core::PooledConnection;

use serde::Serialize;

/// Reserva completa (serializable al frontend, camelCase)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reserva {
    pub id: i64,
    pub id_cliente: Option<i64>,
    pub nombre_cliente: String,
    pub nacionalidad: Option<String>,
    pub categoria_vehiculo: Option<String>,
    pub placa_asignada: Option<String>,
    pub fecha_recogida: String,
    pub hora_recogida: Option<String>,
    pub ubicacion_recogida: Option<String>,
    pub fecha_retorno: String,
    pub hora_retorno: Option<String>,
    pub ubicacion_retorno: Option<String>,
    pub dias_calculados: i64,
    pub horas_extras: i64,
    /// Monto como string (decimal exacto)
    pub valor_dia: String,
    pub valor_hora_adic: String,
    pub costo_lavado: String,
    pub abono: String,
    pub total: String,
    pub observaciones: Option<String>,
    pub estado: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Datos de entrada para crear/actualizar (validados por el servicio)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReservaDatos {
    pub id_cliente: Option<i64>,
    pub nombre_cliente: String,
    pub nacionalidad: Option<String>,
    pub categoria_vehiculo: Option<String>,
    pub placa_asignada: Option<String>,
    pub fecha_recogida: String,
    pub hora_recogida: Option<String>,
    pub ubicacion_recogida: Option<String>,
    pub fecha_retorno: String,
    pub hora_retorno: Option<String>,
    pub ubicacion_retorno: Option<String>,
    pub dias_calculados: i64,
    pub horas_extras: i64,
    pub valor_dia: String,
    pub valor_hora_adic: String,
    pub costo_lavado: String,
    pub abono: String,
    pub total: String,
    pub observaciones: Option<String>,
    pub estado: String,
}

/// Orden de columnas del SELECT de reservas (debe coincidir con `ReservaRow`)
pub const SELECT_COLS: &str = "\
    id, id_cliente, nombre_cliente, nacionalidad, categoria_vehiculo, placa_asignada, \
    CAST(fecha_recogida AS VARCHAR(10)), CAST(hora_recogida AS VARCHAR(13)), ubicacion_recogida, \
    CAST(fecha_retorno AS VARCHAR(10)), CAST(hora_retorno AS VARCHAR(13)), ubicacion_retorno, \
    dias_calculados, horas_extras, \
    CAST(valor_dia AS VARCHAR(12)), CAST(valor_hora_adic AS VARCHAR(12)), \
    CAST(costo_lavado AS VARCHAR(12)), \
    CAST(abono AS VARCHAR(12)), CAST(total AS VARCHAR(12)), \
    CAST(observaciones AS VARCHAR(2000)), estado, \
    CAST(created_at AS VARCHAR(30)), CAST(updated_at AS VARCHAR(30))";

/// Fila de SELECT de reservas (tupla larga — mantener alineada con `SELECT_COLS`)
#[allow(clippy::type_complexity)]
pub type ReservaRow = (
    i64,
    Option<i64>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    i64,
    i64,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

fn from_row(r: ReservaRow) -> Reserva {
    Reserva {
        id: r.0,
        id_cliente: r.1,
        nombre_cliente: r.2,
        nacionalidad: r.3,
        categoria_vehiculo: r.4,
        placa_asignada: r.5,
        fecha_recogida: r.6,
        hora_recogida: r.7.map(|h| hora_corta(&h)),
        ubicacion_recogida: r.8,
        fecha_retorno: r.9,
        hora_retorno: r.10.map(|h| hora_corta(&h)),
        ubicacion_retorno: r.11,
        dias_calculados: r.12,
        horas_extras: r.13,
        valor_dia: r.14,
        valor_hora_adic: r.15,
        costo_lavado: r.16,
        abono: r.17,
        total: r.18,
        observaciones: r.19,
        estado: r.20,
        created_at: r.21,
        updated_at: r.22,
    }
}

/// Mapea errores de Firebird a AppError (FKs de cliente/auto)
fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    map_fb_error_fk(
        e,
        "El cliente o el vehículo seleccionado no existe (o está referenciado por otros registros).",
    )
}

pub struct ReservaRepository;

impl ReservaRepository {
    /// Lista reservas aplicando filtros combinables (búsqueda, estado, rango de fechas)
    pub fn listar_con_filtros(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
        estado: Option<&str>,
        fecha_desde: Option<&str>,
        fecha_hasta: Option<&str>,
    ) -> Result<Vec<Reserva>, AppError> {
        let mut where_clauses = vec!["deleted_at IS NULL".to_string()];
        let mut params = Vec::new();

        if let Some(term) = busqueda.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let like = format!("%{term}%");
            where_clauses.push(
                "(UPPER(nombre_cliente) LIKE UPPER(?) OR UPPER(placa_asignada) LIKE UPPER(?) OR UPPER(nacionalidad) LIKE UPPER(?))".to_string()
            );
            params.push(like.clone().into_param());
            params.push(like.clone().into_param());
            params.push(like.into_param());
        }

        if let Some(est) = estado
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && *s != "Todos")
        {
            where_clauses.push("estado = ?".to_string());
            params.push(est.to_string().into_param());
        }

        if let Some(desde) = fecha_desde.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            where_clauses.push("fecha_retorno >= ?".to_string());
            params.push(desde.to_string().into_param());
        }

        if let Some(hasta) = fecha_hasta.map(|s| s.trim()).filter(|s| !s.is_empty()) {
            where_clauses.push("fecha_recogida <= ?".to_string());
            params.push(hasta.to_string().into_param());
        }

        let sql = format!(
            "SELECT {SELECT_COLS} FROM reservas WHERE {} ORDER BY fecha_recogida DESC, id DESC",
            where_clauses.join(" AND ")
        );

        let rows: Vec<ReservaRow> = conn.query(&sql, ParamsType::Positional(params))?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Lista todas las reservas (más recientes primero)
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Reserva>, AppError> {
        let rows: Vec<ReservaRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM reservas WHERE deleted_at IS NULL ORDER BY fecha_recogida DESC, id DESC"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Busca reservas por cliente, placa o nacionalidad (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Reserva>, AppError> {
        let like = format!("%{}%", term.trim());
        let rows: Vec<ReservaRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM reservas \
                 WHERE (UPPER(nombre_cliente) LIKE UPPER(?) OR UPPER(placa_asignada) LIKE UPPER(?) \
                    OR UPPER(nacionalidad) LIKE UPPER(?)) \
                 AND deleted_at IS NULL ORDER BY fecha_recogida DESC, id DESC"
            ),
            (like.clone(), like.clone(), like),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por estado (Pendiente / Confirmada / Cancelada / Completada / Todos)
    pub fn obtener_por_estado(
        conn: &mut PooledConnection,
        estado: &str,
    ) -> Result<Vec<Reserva>, AppError> {
        let rows: Vec<ReservaRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM reservas WHERE deleted_at IS NULL AND estado = ? \
 ORDER BY fecha_recogida DESC, id DESC"
            ),
            (estado.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Próximas reservas (recogida hoy o en el futuro, no canceladas)
    pub fn obtener_proximas(
        conn: &mut PooledConnection,
        limit: i64,
    ) -> Result<Vec<Reserva>, AppError> {
        let rows: Vec<ReservaRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM reservas \
                 WHERE estado <> 'Cancelada' AND fecha_recogida >= CURRENT_DATE \
                 ORDER BY fecha_recogida, hora_recogida \
                 ROWS {limit}"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Obtiene una reserva por id
    pub fn obtener_por_id(
        conn: &mut PooledConnection,
        id: i64,
    ) -> Result<Option<Reserva>, AppError> {
        let row: Option<ReservaRow> = conn.query_first(
            &format!("SELECT {SELECT_COLS} FROM reservas WHERE deleted_at IS NULL AND id = ?"),
            (id,),
        )?;
        Ok(row.map(from_row))
    }

    /// Crea una reserva y devuelve el id nuevo (RETURNING evita races con MAX(id))
    pub fn insertar(conn: &mut PooledConnection, d: &ReservaDatos) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO reservas (\
                    id_cliente, nombre_cliente, nacionalidad, categoria_vehiculo, placa_asignada, \
                    fecha_recogida, hora_recogida, ubicacion_recogida, fecha_retorno, hora_retorno, \
                    ubicacion_retorno, dias_calculados, horas_extras, valor_dia, valor_hora_adic, \
                    costo_lavado, abono, total, observaciones, estado \
                 ) VALUES (\
                    ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
                    ?, ?, ?, CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), \
                    CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), ?, ? \
                 ) RETURNING id",
                params![
                    d.id_cliente,
                    d.nombre_cliente.to_string(),
                    opt_str(&d.nacionalidad),
                    opt_str(&d.categoria_vehiculo),
                    opt_str(&d.placa_asignada),
                    parse_fecha(&d.fecha_recogida)?,
                    parse_hora_opt(&d.hora_recogida)?,
                    opt_str(&d.ubicacion_recogida),
                    parse_fecha(&d.fecha_retorno)?,
                    parse_hora_opt(&d.hora_retorno)?,
                    opt_str(&d.ubicacion_retorno),
                    d.dias_calculados,
                    d.horas_extras,
                    d.valor_dia.to_string(),
                    d.valor_hora_adic.to_string(),
                    d.costo_lavado.to_string(),
                    d.abono.to_string(),
                    d.total.to_string(),
                    opt_str(&d.observaciones),
                    d.estado.to_string(),
                ],
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Actualiza una reserva por id
    pub fn actualizar(
        conn: &mut PooledConnection,
        id: i64,
        d: &ReservaDatos,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE reservas SET \
                id_cliente = ?, nombre_cliente = ?, nacionalidad = ?, categoria_vehiculo = ?, \
                placa_asignada = ?, fecha_recogida = ?, hora_recogida = ?, \
                ubicacion_recogida = ?, fecha_retorno = ?, hora_retorno = ?, \
                ubicacion_retorno = ?, dias_calculados = ?, horas_extras = ?, \
                valor_dia = CAST(? AS DECIMAL(12,2)), valor_hora_adic = CAST(? AS DECIMAL(12,2)), \
                costo_lavado = CAST(? AS DECIMAL(12,2)), \
                abono = CAST(? AS DECIMAL(12,2)), total = CAST(? AS DECIMAL(12,2)), \
                observaciones = ?, estado = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
            params![
                d.id_cliente,
                d.nombre_cliente.to_string(),
                opt_str(&d.nacionalidad),
                opt_str(&d.categoria_vehiculo),
                opt_str(&d.placa_asignada),
                parse_fecha(&d.fecha_recogida)?,
                parse_hora_opt(&d.hora_recogida)?,
                opt_str(&d.ubicacion_recogida),
                parse_fecha(&d.fecha_retorno)?,
                parse_hora_opt(&d.hora_retorno)?,
                opt_str(&d.ubicacion_retorno),
                d.dias_calculados,
                d.horas_extras,
                d.valor_dia.to_string(),
                d.valor_hora_adic.to_string(),
                d.costo_lavado.to_string(),
                d.abono.to_string(),
                d.total.to_string(),
                opt_str(&d.observaciones),
                d.estado.to_string(),
                id,
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Cambia el estado de una reserva (cancelar / completar / confirmar).
    /// Genérica sobre la conexión para poder llamarse dentro de una
    /// transacción (`with_transaction`) además de en operaciones directas.
    pub fn cambiar_estado<C>(conn: &mut C, id: i64, estado: &str) -> Result<(), AppError>
    where
        C: Execute,
    {
        conn.execute(
            "UPDATE reservas SET estado = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            (estado.to_string(), id),
        )?;
        Ok(())
    }

    /// Soft-delete de una reserva (las rentas asociadas quedan con id_reserva NULL por SET NULL)
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute(
            "UPDATE reservas SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
            (id,),
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Total de reservas
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> =
            conn.query_first("SELECT COUNT(*) FROM reservas WHERE deleted_at IS NULL", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Conteo por estado (para el dashboard / filtros)
    pub fn contar_por_estado(conn: &mut PooledConnection) -> Result<Vec<(String, i64)>, AppError> {
        let rows: Vec<(String, i64)> = conn.query(
            "SELECT estado, COUNT(*) FROM reservas WHERE deleted_at IS NULL GROUP BY estado ORDER BY estado",
            (),
        )?;
        Ok(rows)
    }
}

/// Recorta 'HH:MM:SS.0000' (Firebird) a 'HH:MM' para la UI
fn hora_corta(h: &str) -> String {
    h.split(':').take(2).collect::<Vec<_>>().join(":")
}
