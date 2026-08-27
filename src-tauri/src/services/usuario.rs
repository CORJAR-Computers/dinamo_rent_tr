//! services/usuario.rs — Gestión de usuarios (puerto de usuario_service.py)
//!
//! CRUD exclusivo para roles de administración (`roles_con_usuarios` en config).
//! Reglas de negocio:
//!   - username único (insensible a mayúsculas)
//!   - no se puede eliminar la propia cuenta
//!   - no se puede eliminar/desactivar/despromover al ÚLTIMO administrador activo
//!   - forzar cambio de contraseña = nuevo hash + `debe_cambiar_password = 1`
//!   - todas las operaciones se registran en la tabla de auditoría

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::core::audit::log_audit;
use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::security;
use crate::core::validators::{mayusculas, validate_no_xss};
use crate::core::PooledConnection;
use crate::repositories::usuario::{Usuario, UsuarioRepository};

/// Datos para crear un usuario (validados por el servicio)
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UsuarioDatos {
    pub username: String,
    pub password: String,
    pub nombre: String,
    pub rol: String,
    pub email: Option<String>,
    pub activo: bool,
    pub debe_cambiar_password: bool,
}

/// Datos para actualizar un usuario (sin password — la contraseña se gestiona aparte)
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UsuarioDatosActualizar {
    pub nombre: String,
    pub rol: String,
    pub email: Option<String>,
    pub activo: bool,
}

/// Resultado del cambio forzado de contraseña (para la UI)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsuarioConCambio {
    pub usuario: Usuario,
    /// true si el cambio se aplicó y el usuario deberá cambiarla en el próximo login
    pub cambio_forzado: bool,
}

/// Roles asignables por defecto cuando config.ini no define `business.roles_usuarios`
const ROLES_FALLBACK: [&str; 3] = ["Administrador", "Supervisor", "Operador"];

/// Roles asignables según config (con fallback si la clave no está definida)
fn roles_permitidos(cfg: &Arc<AppConfig>) -> Vec<&str> {
    if cfg.roles_usuarios.is_empty() {
        ROLES_FALLBACK.to_vec()
    } else {
        cfg.roles_usuarios.iter().map(|s| s.as_str()).collect()
    }
}

pub struct UsuarioService;

impl UsuarioService {
    /// Lista usuarios (búsqueda opcional por username, nombre o rol)
    pub fn listar(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
    ) -> Result<Vec<Usuario>, AppError> {
        let term = busqueda.unwrap_or("").trim();
        if term.is_empty() {
            UsuarioRepository::obtener_todos(conn)
        } else {
            UsuarioRepository::buscar(conn, term)
        }
    }

    /// Obtiene un usuario por id
    pub fn obtener(conn: &mut PooledConnection, id: i64) -> Result<Usuario, AppError> {
        UsuarioRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe el usuario #{id}")))
    }

    /// Crea un usuario (contraseña inicial + opción de cambio obligatorio)
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        actor: &str,
        mut datos: UsuarioDatos,
    ) -> Result<Usuario, AppError> {
        datos.username = datos.username.trim().to_string();
        datos.nombre = mayusculas(&datos.nombre);
        datos.rol = datos.rol.trim().to_string(); // rol: capitalización fija (Administrador, Supervisor, Operador)
        datos.email = datos
            .email
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()); // email: sin mayúsculas
        validar_base(&datos.nombre, &datos.rol, datos.email.as_deref(), cfg)?;

        // Username: obligatorio, sin espacios, longitud permitida
        if datos.username.is_empty() || datos.username.len() > 50 {
            return Err(AppError::Validation(
                "El nombre de usuario es obligatorio (máx. 50 caracteres).".into(),
            ));
        }
        if !datos
            .username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        {
            return Err(AppError::Validation(
                "El nombre de usuario solo puede contener letras, números, puntos, guiones y guiones bajos."
                    .into(),
            ));
        }
        if UsuarioRepository::existe_username(conn, &datos.username)? {
            return Err(AppError::Duplicate(format!(
                "Ya existe un usuario con el nombre '{username}'.",
                username = datos.username
            )));
        }

        // Contraseña inicial
        let errors = crate::core::validators::validate_password_strength(&datos.password);
        if !errors.is_empty() {
            return Err(AppError::Validation(errors.join("; ")));
        }
        let hash = security::hash_password(&datos.password)?;

        let id = UsuarioRepository::insertar_completo(
            conn,
            &datos.username,
            &hash,
            &datos.nombre,
            &datos.rol,
            datos.email.as_deref(),
            datos.activo,
            datos.debe_cambiar_password,
        )?;
        log_audit(
            conn,
            actor,
            "USUARIO CREADO",
            &format!(
                "username={}, rol={}, activo={}, cambiar={}",
                datos.username, datos.rol, datos.activo, datos.debe_cambiar_password
            ),
            "local",
        )?;
        Self::obtener(conn, id)
    }

    /// Actualiza datos de gestión (nombre, rol, email, activo).
    /// Protege al último administrador activo de ser desactivado/despromovido.
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        actor: &str,
        id: i64,
        mut datos: UsuarioDatosActualizar,
    ) -> Result<Usuario, AppError> {
        let actual = Self::obtener(conn, id)?;
        datos.nombre = mayusculas(&datos.nombre);
        datos.rol = datos.rol.trim().to_string(); // rol: capitalización fija
        datos.email = datos
            .email
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()); // email: sin mayúsculas
        validar_base(&datos.nombre, &datos.rol, datos.email.as_deref(), cfg)?; // Protección: último administrador activo no se puede desactivar ni despromover
        let es_admin_activo = actual.rol.as_deref() == Some("Administrador") && actual.activo;
        if es_admin_activo
            && UsuarioRepository::contar_admins(conn)? <= 1
            && (datos.rol != "Administrador" || !datos.activo)
        {
            return Err(AppError::Business(
                "No se puede modificar al último administrador activo del sistema.".into(),
            ));
        }

        UsuarioRepository::actualizar_datos(
            conn,
            id,
            &datos.nombre,
            &datos.rol,
            datos.email.as_deref(),
            datos.activo,
        )?;
        log_audit(
            conn,
            actor,
            "USUARIO ACTUALIZADO",
            &format!(
                "id={id}, username={}, rol={}, activo={}",
                actual.username, datos.rol, datos.activo
            ),
            "local",
        )?;
        Self::obtener(conn, id)
    }

    /// Elimina un usuario. No permite eliminar la propia cuenta ni al último admin.
    pub fn eliminar(conn: &mut PooledConnection, actor: &str, id: i64) -> Result<(), AppError> {
        let actual = Self::obtener(conn, id)?;
        if actual.username == actor {
            return Err(AppError::Business(
                "No puedes eliminar tu propia cuenta.".into(),
            ));
        }
        let es_admin_activo = actual.rol.as_deref() == Some("Administrador") && actual.activo;
        if es_admin_activo && UsuarioRepository::contar_admins(conn)? <= 1 {
            return Err(AppError::Business(
                "No se puede eliminar al último administrador activo del sistema.".into(),
            ));
        }
        UsuarioRepository::eliminar(conn, id)?;
        log_audit(
            conn,
            actor,
            "USUARIO ELIMINADO",
            &format!("id={id}, username={}", actual.username),
            "local",
        )?;
        Ok(())
    }

    /// Fuerza el cambio de contraseña: aplica el nuevo hash y marca
    /// `debe_cambiar_password = 1` para el próximo login.
    pub fn forzar_cambio_password(
        conn: &mut PooledConnection,
        actor: &str,
        id: i64,
        nueva_password: &str,
    ) -> Result<UsuarioConCambio, AppError> {
        let usuario = Self::obtener(conn, id)?;
        let errors = crate::core::validators::validate_password_strength(nueva_password);
        if !errors.is_empty() {
            return Err(AppError::Validation(errors.join("; ")));
        }
        let hash = security::hash_password(nueva_password)?;
        UsuarioRepository::reset_password(conn, id, &hash)?;
        log_audit(
            conn,
            actor,
            "CONTRASEÑA REINICIADA",
            &format!(
                "id={id}, username={} (cambio obligatorio en próximo login)",
                usuario.username
            ),
            "local",
        )?;
        let usuario = Self::obtener(conn, id)?;
        Ok(UsuarioConCambio {
            usuario,
            cambio_forzado: true,
        })
    }
}

/// Valida nombre, rol y email (compartido entre crear y actualizar)
fn validar_base(
    nombre: &str,
    rol: &str,
    email: Option<&str>,
    cfg: &Arc<AppConfig>,
) -> Result<(), AppError> {
    if nombre.is_empty() || nombre.len() > 100 {
        return Err(AppError::Validation(
            "El nombre del usuario es obligatorio (máx. 100 caracteres).".into(),
        ));
    }
    validate_no_xss(nombre, 100)
        .map_err(|_| AppError::Validation("El nombre contiene caracteres no permitidos.".into()))?;
    if rol.is_empty() {
        return Err(AppError::Validation("El rol es obligatorio.".into()));
    }
    let permitidos = roles_permitidos(cfg);
    if !permitidos.contains(&rol) {
        return Err(AppError::Validation(format!(
            "Rol inválido '{rol}'. Permitidos: {}",
            permitidos.join(", ")
        )));
    }
    if let Some(email) = email {
        if !email.is_empty() && !(email.contains('@') && email.contains('.') && email.len() <= 100)
        {
            return Err(AppError::Validation(
                "El correo electrónico no es válido.".into(),
            ));
        }
    }
    Ok(())
}
