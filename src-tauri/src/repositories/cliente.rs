//! repositories/cliente.rs — Repositorio de clientes (puerto de cliente_repository_sa.py)
//!
//! Las columnas PII (celular, celular2, email, dir_residencia, dir_temporal, no_licencia)
//! se guardan cifradas. Este repositorio NO descifra: el servicio (services/cliente.rs)
//! aplica `PiiCipher` al leer/escribir.

use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;
// Helpers centralizados (Bloque 4 / TAREA 4.2): antes estaban duplicados
// localmente en este archivo. La migración los importa de `core::repository`
// para DRY. Se conserva un wrapper `map_fb_error` (1 línea) que delega en
// `map_fb_error_dup` con el mensaje Duplicate específico de clientes.
use crate::core::repository::{opt_str, params};

use serde::Serialize;

/// Cliente (serializable al frontend, camelCase). PII en claro (ya descifrada).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cliente {
    pub id: i64,
    pub tipo_doc: Option<String>,
    pub no_doc: Option<String>,
    pub nombres: String,
    pub apellidos: Option<String>,
    pub nombre_completo: String,
    pub celular: Option<String>,
    pub celular2: Option<String>,
    pub email: Option<String>,
    pub ciudad: Option<String>,
    pub estado_region: Option<String>,
    pub pais: Option<String>,
    pub nacionalidad: Option<String>,
    pub dir_residencia: Option<String>,
    pub dir_temporal: Option<String>,
    pub hotel: Option<String>,
    pub habitacion: Option<String>,
    pub no_licencia: Option<String>,
    pub tipo_licencia: Option<String>,
    pub vencimiento_licencia: Option<String>,
    pub estado: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Datos de entrada para crear/actualizar (PII en claro; el servicio cifra)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ClienteDatos {
    pub tipo_doc: Option<String>,
    pub no_doc: Option<String>,
    pub nombres: String,
    pub apellidos: Option<String>,
    pub nombre_completo: String,
    pub celular: Option<String>,
    pub celular2: Option<String>,
    pub email: Option<String>,
    pub ciudad: Option<String>,
    pub estado_region: Option<String>,
    pub pais: Option<String>,
    pub nacionalidad: Option<String>,
    pub dir_residencia: Option<String>,
    pub dir_temporal: Option<String>,
    pub hotel: Option<String>,
    pub habitacion: Option<String>,
    pub no_licencia: Option<String>,
    pub tipo_licencia: Option<String>,
    pub vencimiento_licencia: Option<String>,
    pub estado: String,
}

/// Orden de columnas del SELECT de clientes (debe coincidir con `ClienteRow`)
pub const SELECT_COLS: &str = "\
    id, tipo_doc, no_doc, nombres, apellidos, nombre_completo, \
    celular, celular2, email, ciudad, estado_region, pais, nacionalidad, \
    dir_residencia, dir_temporal, hotel, habitacion, \
    no_licencia, tipo_licencia, CAST(vencimiento_licencia AS VARCHAR(10)), \
    estado, CAST(created_at AS VARCHAR(30)), CAST(updated_at AS VARCHAR(30))";

/// Fila de SELECT de clientes (tupla larga — mantener alineada con `SELECT_COLS`)
#[allow(clippy::type_complexity)]
pub type ClienteRow = (
    i64,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

pub fn from_row(r: ClienteRow) -> Cliente {
    Cliente {
        id: r.0,
        tipo_doc: r.1,
        no_doc: r.2,
        nombres: r.3,
        apellidos: r.4,
        nombre_completo: r.5,
        celular: r.6,
        celular2: r.7,
        email: r.8,
        ciudad: r.9,
        estado_region: r.10,
        pais: r.11,
        nacionalidad: r.12,
        dir_residencia: r.13,
        dir_temporal: r.14,
        hotel: r.15,
        habitacion: r.16,
        no_licencia: r.17,
        tipo_licencia: r.18,
        vencimiento_licencia: r.19,
        estado: r.20,
        created_at: r.21,
        updated_at: r.22,
    }
}

/// Mapea errores de Firebird a AppError (duplicidad de documento).
///
/// Wrapper que delega en `crate::core::repository::map_fb_error_dup` con el
/// mensaje específico de clientes. Antes esto estaba duplicado en 3+
/// repositorios (Bloque 4 / TAREA 4.2).
fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    crate::core::repository::map_fb_error_dup(e, "Ya existe un cliente con ese documento.")
}

pub struct ClienteRepository;

impl ClienteRepository {
    /// Lista todos los clientes (por nombre completo)
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Cliente>, AppError> {
        let rows: Vec<ClienteRow> = conn.query(
            &format!("SELECT {SELECT_COLS} FROM clientes WHERE deleted_at IS NULL ORDER BY nombre_completo"),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Busca clientes por documento o nombre (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Cliente>, AppError> {
        let like = format!("%{}%", term.trim());
        let rows: Vec<ClienteRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM clientes \
                 WHERE (UPPER(nombre_completo) LIKE UPPER(?) OR UPPER(nombres) LIKE UPPER(?) \
                    OR UPPER(no_doc) LIKE UPPER(?) OR UPPER(celular) LIKE UPPER(?)) \
                 AND deleted_at IS NULL ORDER BY nombre_completo"
            ),
            (like.clone(), like.clone(), like.clone(), like),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por estado (Activo / Inactivo / Lista Negra / VIP)
    pub fn obtener_por_estado(conn: &mut PooledConnection, estado: &str) -> Result<Vec<Cliente>, AppError> {
        let rows: Vec<ClienteRow> = conn.query(
            &format!("SELECT {SELECT_COLS} FROM clientes WHERE deleted_at IS NULL AND estado = ? ORDER BY nombre_completo"),
            (estado.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Obtiene un cliente por id
    pub fn obtener_por_id(conn: &mut PooledConnection, id: i64) -> Result<Option<Cliente>, AppError> {
        let row: Option<ClienteRow> = conn.query_first(
            &format!("SELECT {SELECT_COLS} FROM clientes WHERE deleted_at IS NULL AND id = ?"),
            (id,),
        )?;
        Ok(row.map(from_row))
    }

    /// Obtiene un cliente por documento (verificación de unicidad)
    #[allow(dead_code)]
    pub fn obtener_por_documento(
        conn: &mut PooledConnection,
        no_doc: &str,
    ) -> Result<Option<Cliente>, AppError> {
        if no_doc.trim().is_empty() {
            return Ok(None);
        }
        let row: Option<ClienteRow> = conn.query_first(
            &format!("SELECT {SELECT_COLS} FROM clientes WHERE deleted_at IS NULL AND no_doc = ?"),
            (no_doc.trim().to_string(),),
        )?;
        Ok(row.map(from_row))
    }

    /// Últimos clientes creados (dashboard)
    pub fn recientes(conn: &mut PooledConnection, limit: i64) -> Result<Vec<Cliente>, AppError> {
        let rows: Vec<ClienteRow> = conn.query(
            &format!(
                "SELECT FIRST {limit} {SELECT_COLS} FROM clientes WHERE deleted_at IS NULL ORDER BY id DESC"
            ),
            (),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Crea un cliente y devuelve su id (RETURNING evita race conditions de MAX(id))
    pub fn insertar(conn: &mut PooledConnection, d: &ClienteDatos) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO clientes (\
                    tipo_doc, no_doc, nombres, apellidos, nombre_completo, \
                    celular, celular2, email, ciudad, estado_region, pais, nacionalidad, \
                    dir_residencia, dir_temporal, hotel, habitacion, \
                    no_licencia, tipo_licencia, vencimiento_licencia, estado \
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
                params![
                    opt_str(&d.tipo_doc),
                    opt_str(&d.no_doc),
                    d.nombres.to_string(),
                    opt_str(&d.apellidos),
                    d.nombre_completo.to_string(),
                    opt_str(&d.celular),
                    opt_str(&d.celular2),
                    opt_str(&d.email),
                    opt_str(&d.ciudad),
                    opt_str(&d.estado_region),
                    opt_str(&d.pais),
                    opt_str(&d.nacionalidad),
                    opt_str(&d.dir_residencia),
                    opt_str(&d.dir_temporal),
                    opt_str(&d.hotel),
                    opt_str(&d.habitacion),
                    opt_str(&d.no_licencia),
                    opt_str(&d.tipo_licencia),
                    opt_str(&d.vencimiento_licencia),
                    d.estado.to_string(),
                ],
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Actualiza un cliente por id
    pub fn actualizar(conn: &mut PooledConnection, id: i64, d: &ClienteDatos) -> Result<(), AppError> {
        conn.execute(
            "UPDATE clientes SET \
                tipo_doc = ?, no_doc = ?, nombres = ?, apellidos = ?, nombre_completo = ?, \
                celular = ?, celular2 = ?, email = ?, ciudad = ?, estado_region = ?, pais = ?, nacionalidad = ?, \
                dir_residencia = ?, dir_temporal = ?, hotel = ?, habitacion = ?, \
                no_licencia = ?, tipo_licencia = ?, vencimiento_licencia = ?, estado = ?, \
                updated_at = CURRENT_TIMESTAMP \
             WHERE id = ?",
            params![
                opt_str(&d.tipo_doc),
                opt_str(&d.no_doc),
                d.nombres.to_string(),
                opt_str(&d.apellidos),
                d.nombre_completo.to_string(),
                opt_str(&d.celular),
                opt_str(&d.celular2),
                opt_str(&d.email),
                opt_str(&d.ciudad),
                opt_str(&d.estado_region),
                opt_str(&d.pais),
                opt_str(&d.nacionalidad),
                opt_str(&d.dir_residencia),
                opt_str(&d.dir_temporal),
                opt_str(&d.hotel),
                opt_str(&d.habitacion),
                opt_str(&d.no_licencia),
                opt_str(&d.tipo_licencia),
                opt_str(&d.vencimiento_licencia),
                d.estado.to_string(),
                id,
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Soft-delete de un cliente (FKs de rentas/reservas/comparendos son SET NULL)
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        match conn.execute("UPDATE clientes SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?", (id,)) {
            Ok(_) => Ok(()),
            Err(e) => {
                let lower = e.to_string().to_lowercase();
                if lower.contains("foreign key")
                    || lower.contains("still referenced")
                    || lower.contains("not a valid reference")
                {
                    Err(AppError::Business(
                        "El cliente tiene registros asociados (rentas, reservas, comparendos) y no puede eliminarse."
                            .into(),
                    ))
                } else {
                    Err(AppError::Database(e.to_string()))
                }
            }
        }
    }

    /// Total de clientes
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM clientes WHERE deleted_at IS NULL", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }
}


