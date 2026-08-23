//! repositories/usuario.rs — Repositorio de usuarios (puerto de usuario_repository_sa.py)
//!
//! Queries explícitas en dialecto Firebird, estilo rsfbclient (FromRow).
//!
//! > **TODO (Bloque 4 / TAREA 4.2)**: este repositorio aún define helpers
//! > locales (`map_fb_error`, `opt_str`, `params!`, ...) duplicados con
//! > `crate::core::repository`. Migración pendiente — ver
//! > `src/core/repository.rs` para el módulo centralizado.

use rsfbclient::{Execute, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usuario {
    pub id: i64,
    pub username: String,
    pub nombre: Option<String>,
    pub rol: Option<String>,
    pub email: Option<String>,
    pub activo: bool,
    pub debe_cambiar_password: bool,
    pub intentos_fallidos: i64,
    pub ultimo_acceso: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UsuarioAuth {
    pub id: i64,
    pub username: String,
    pub password: String,
    pub nombre: Option<String>,
    pub rol: Option<String>,
    pub activo: bool,
    pub debe_cambiar_password: bool,
    pub intentos_fallidos: i64,
}

impl Usuario {
    #[allow(clippy::too_many_arguments)]
    fn from_row(
        id: i64,
        username: String,
        nombre: Option<String>,
        rol: Option<String>,
        email: Option<String>,
        activo: i16,
        debe_cambiar_password: i16,
        intentos_fallidos: i64,
        ultimo_acceso: Option<String>,
        created_at: Option<String>,
    ) -> Self {
        Self {
            id,
            username,
            nombre,
            rol,
            email,
            activo: activo == 1,
            debe_cambiar_password: debe_cambiar_password == 1,
            intentos_fallidos,
            ultimo_acceso,
            created_at,
        }
    }
}

/// Fila de usuarios para SELECT de gestión (tupla larga)
#[allow(clippy::type_complexity)]
/// Fila de SELECT de autenticación: id, username, password, nombre, rol, activo, debe_cambiar, intentos
pub type UsuarioAuthRow = (i64, String, String, Option<String>, Option<String>, i16, i16, i64);

/// Fila de SELECT de gestión: id, username, nombre, rol, email, activo, debe_cambiar,
/// intentos, ultimo_acceso (varchar), created_at (varchar)
pub type UsuarioRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    i16,
    i16,
    i64,
    Option<String>,
    Option<String>,
);

/// Columnas del SELECT de gestión (con timestamps como VARCHAR para evitar
/// problemas de tipo del driver). Se reutiliza en todas las queries de listado.
const SELECT_COLS: &str = "\
    id, username, nombre, rol, email, activo, debe_cambiar_password, intentos_fallidos, \
    CAST(ultimo_acceso AS VARCHAR(30)), CAST(created_at AS VARCHAR(30))";

pub struct UsuarioRepository;

impl UsuarioRepository {
    /// Obtiene usuario con hash de contraseña para autenticación
    pub fn obtener_para_autenticacion(
        conn: &mut PooledConnection,
        username: &str,
    ) -> Result<Option<UsuarioAuth>, AppError> {
        let row: Option<UsuarioAuthRow> = conn
            .query_first(
                "SELECT id, username, password, nombre, rol, activo, debe_cambiar_password, intentos_fallidos
                 FROM usuarios WHERE username = ?",
                (username.trim().to_string(),),
            )?;
        Ok(row.map(
            |(id, username, password, nombre, rol, activo, debe, intentos)| UsuarioAuth {
                id,
                username,
                password,
                nombre,
                rol,
                activo: activo == 1,
                debe_cambiar_password: debe == 1,
                intentos_fallidos: intentos,
            },
        ))
    }

    /// Obtiene un usuario por username (sin password) — para gestión
    pub fn obtener_por_username(
        conn: &mut PooledConnection,
        username: &str,
    ) -> Result<Option<Usuario>, AppError> {        let row: Option<UsuarioRow> = conn.query_first(
            &format!("SELECT {SELECT_COLS} FROM usuarios WHERE username = ? AND deleted_at IS NULL"),
            (username.trim().to_string(),),
        )?;
        Ok(row.map(
            |(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)| {
                Usuario::from_row(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)
            },
        ))   
    }

    /// Lista todos los usuarios (orden alfabético)
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Usuario>, AppError> {        let rows: Vec<UsuarioRow> =
            conn.query(&format!("SELECT {SELECT_COLS} FROM usuarios WHERE deleted_at IS NULL AND deleted_at IS NULL ORDER BY username"), ())?;
        Ok(rows
            .into_iter()
            .map(|(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)| {
                Usuario::from_row(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)
            })
            .collect())   
    }

    /// Obtiene un usuario por id (gestión)
    pub fn obtener_por_id(
        conn: &mut PooledConnection,
        id: i64,
    ) -> Result<Option<Usuario>, AppError> {
        let row: Option<UsuarioRow> = conn.query_first(
            &format!("SELECT {SELECT_COLS} FROM usuarios WHERE id = ?"),
            (id,),
        )?;
        Ok(row.map(
            |(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)| {
                Usuario::from_row(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)
            },
        ))
    }

    /// Busca usuarios por username, nombre o rol (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Usuario>, AppError> {
        let like = format!("%{}%", term.trim());
        let rows: Vec<UsuarioRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM usuarios \
                 WHERE UPPER(username) LIKE UPPER(?) OR UPPER(nombre) LIKE UPPER(?) OR UPPER(rol) LIKE UPPER(?) \
                 AND deleted_at IS NULL ORDER BY username"
            ),
            (like.clone(), like.clone(), like),
        )?;
        Ok(rows
            .into_iter()
            .map(|(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)| {
                Usuario::from_row(id, username, nombre, rol, email, activo, debe, intentos, ultimo, creado)
            })
            .collect())
    }

    /// ¿Existe un username? (chequeo de unicidad, insensible a mayúsculas)
    pub fn existe_username(conn: &mut PooledConnection, username: &str) -> Result<bool, AppError> {
        let count: Option<(i64,)> = conn.query_first(
            "SELECT COUNT(*) FROM usuarios WHERE UPPER(username) = UPPER(?) AND deleted_at IS NULL",
            (username.trim().to_string(),),
        )?;
        Ok(count.map(|(c,)| c).unwrap_or(0) > 0)
    }

    /// Crea un usuario de gestión (admin). Devuelve el id nuevo.
    #[allow(clippy::too_many_arguments)]
    pub fn insertar_completo(
        conn: &mut PooledConnection,
        username: &str,
        password_hash: &str,
        nombre: &str,
        rol: &str,
        email: Option<&str>,
        activo: bool,
        debe_cambiar_password: bool,
    ) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO usuarios (username, password, nombre, rol, email, activo, intentos_fallidos, debe_cambiar_password)
                 VALUES (?, ?, ?, ?, ?, ?, 0, ?) RETURNING id",
                (
                    username.to_string(),
                    password_hash.to_string(),
                    nombre.to_string(),
                    rol.to_string(),
                    email.map(String::from),
                    if activo { 1_i16 } else { 0_i16 },
                    if debe_cambiar_password { 1_i16 } else { 0_i16 },
                ),
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Actualiza datos de gestión de un usuario (nombre, rol, email, activo)
    pub fn actualizar_datos(
        conn: &mut PooledConnection,
        id: i64,
        nombre: &str,
        rol: &str,
        email: Option<&str>,
        activo: bool,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE usuarios SET nombre = ?, rol = ?, email = ?, activo = ?, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?",
            (
                nombre.to_string(),
                rol.to_string(),
                email.map(String::from),
                if activo { 1_i16 } else { 0_i16 },
                id,
            ),
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Elimina un usuario (soft-delete; sin FKs que lo impidan)
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute("UPDATE usuarios SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?", (id,))
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Cuenta administradores activos (protección de último admin)
    pub fn contar_admins(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first(
            "SELECT COUNT(*) FROM usuarios WHERE rol = 'Administrador' AND activo = 1 AND deleted_at IS NULL",
            (),
        )?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Establece una contraseña nueva y marca el cambio como obligatorio en el próximo login
    pub fn reset_password(
        conn: &mut PooledConnection,
        id: i64,
        new_hash: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE usuarios SET password = ?, debe_cambiar_password = 1, intentos_fallidos = 0, \
             updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            (new_hash.to_string(), id),
        )?;
        Ok(())
    }

    /// Registra último acceso y resetea intentos fallidos
    pub fn registrar_acceso(conn: &mut PooledConnection, username: &str) -> Result<(), AppError> {
        conn.execute(
            "UPDATE usuarios SET ultimo_acceso = CURRENT_TIMESTAMP, intentos_fallidos = 0
             WHERE username = ? AND deleted_at IS NULL",
            (username.to_string(),),
        )?;
        Ok(())
    }

    /// Actualiza el hash de contraseña (re-hash Argon2id tras login legacy)
    pub fn actualizar_password(
        conn: &mut PooledConnection,
        username: &str,
        new_hash: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE usuarios SET password = ?, debe_cambiar_password = 0, updated_at = CURRENT_TIMESTAMP
             WHERE username = ?",
            (new_hash.to_string(), username.to_string()),
        )?;
        Ok(())
    }

    /// Persiste el contador de intentos fallidos
    pub fn persistir_intentos(
        conn: &mut PooledConnection,
        username: &str,
        intentos: i64,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE usuarios SET intentos_fallidos = ? WHERE username = ? AND deleted_at IS NULL",
            (intentos, username.to_string()),
        )?;
        Ok(())
    }

    /// Resetea intentos fallidos (desbloqueo manual)
    pub fn reset_intentos(conn: &mut PooledConnection, username: &str) -> Result<(), AppError> {
        conn.execute(
            "UPDATE usuarios SET intentos_fallidos = 0, updated_at = CURRENT_TIMESTAMP
             WHERE username = ? AND deleted_at IS NULL",
            (username.to_string(),),
        )?;
        Ok(())
    }

    /// Obtiene el mapa username → intentos_fallidos para sync del tracker
    pub fn obtener_intentos_pendientes(
        conn: &mut PooledConnection,
    ) -> Result<std::collections::HashMap<String, u32>, AppError> {
        let rows: Vec<(String, i64)> = conn.query(
            "SELECT username, intentos_fallidos FROM usuarios WHERE intentos_fallidos > 0 AND deleted_at IS NULL",
            (),
        )?;
        Ok(rows.into_iter().map(|(u, i)| (u, i as u32)).collect())
    }

    /// Cuenta usuarios (para seed)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM usuarios", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Obtiene la preferencia de tema del usuario ('light' | 'dark' | 'auto' | NULL)
    pub fn obtener_tema(
        conn: &mut PooledConnection,
        id: i64,
    ) -> Result<Option<String>, AppError> {
        let row: Option<(Option<String>,)> =
            conn.query_first("SELECT tema FROM usuarios WHERE id = ? AND deleted_at IS NULL", (id,))?;
        Ok(row.and_then(|(t,)| t))
    }

    /// Guarda la preferencia de tema del usuario
    pub fn guardar_tema(
        conn: &mut PooledConnection,
        id: i64,
        tema: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE usuarios SET tema = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND deleted_at IS NULL",
            (tema.to_string(), id),
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Crea un usuario (seed admin o nuevos)
    pub fn insertar(
        conn: &mut PooledConnection,
        username: &str,
        password_hash: &str,
        nombre: &str,
        rol: &str,
        debe_cambiar_password: bool,
    ) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO usuarios (username, password, nombre, rol, activo, intentos_fallidos, debe_cambiar_password)
                 VALUES (?, ?, ?, ?, 1, 0, ?) RETURNING id",
                (
                    username.to_string(),
                    password_hash.to_string(),
                    nombre.to_string(),
                    rol.to_string(),
                    if debe_cambiar_password { 1_i16 } else { 0_i16 },
                ),
            )?;
        Ok(id)
    }
}

/// Mapea errores de Firebird a AppError (unicidad de username)
fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("duplicate") || lower.contains("unique") {
        AppError::Duplicate("Ya existe un usuario con ese nombre de usuario.".into())
    } else {
        AppError::Database(msg)
    }
}
