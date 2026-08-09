//! commands/usuario.rs — Comandos Tauri del módulo de usuarios (thin wrappers).
//!
//! Todos los comandos requieren rol de administración de usuarios
//! (`roles_con_usuarios` de config.ini — por defecto: Administrador).

use crate::core::error::ErrorPayload;
use crate::repositories::usuario::Usuario;
use crate::services::auth::AuthService;
use crate::services::usuario::{
    UsuarioConCambio, UsuarioDatos, UsuarioDatosActualizar, UsuarioService,
};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_usuario_admin};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista usuarios (búsqueda opcional por username, nombre o rol)
#[tauri::command]
pub fn listar_usuarios(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
) -> Cmd<Vec<Usuario>> {
    require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    UsuarioService::listar(&mut c, busqueda.as_deref()).map_err(|e| e.to_payload())
}

/// Crea un usuario con contraseña inicial (opción: cambio obligatorio en primer login)
#[tauri::command]
pub fn crear_usuario(
    state: State<'_, AppState>,
    session_id: String,
    datos: UsuarioDatos,
) -> Cmd<Usuario> {
    let session = require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    UsuarioService::crear(&mut c, &state.config, &session.username, datos)
        .map_err(|e| e.to_payload())
}

/// Actualiza nombre, rol, email y estado activo de un usuario
#[tauri::command]
pub fn actualizar_usuario(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: UsuarioDatosActualizar,
) -> Cmd<Usuario> {
    let session = require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    UsuarioService::actualizar(&mut c, &state.config, &session.username, id, datos)
        .map_err(|e| e.to_payload())
}

/// Elimina un usuario (protege la propia cuenta y al último administrador)
#[tauri::command]
pub fn eliminar_usuario(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    let session = require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    UsuarioService::eliminar(&mut c, &session.username, id).map_err(|e| e.to_payload())
}

/// Fuerza el cambio de contraseña: aplica la nueva y marca cambio obligatorio en el próximo login
#[tauri::command]
pub fn forzar_cambio_password_usuario(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    nueva_password: String,
) -> Cmd<UsuarioConCambio> {
    let session = require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    UsuarioService::forzar_cambio_password(&mut c, &session.username, id, &nueva_password)
        .map_err(|e| e.to_payload())
}

/// Desbloquea una cuenta (resetea intentos fallidos en BD y en el tracker de sesiones)
#[tauri::command]
pub fn desbloquear_usuario(
    state: State<'_, AppState>,
    session_id: String,
    username: String,
) -> Cmd<bool> {
    require_usuario_admin(&state, &session_id)?;
    AuthService::unlock_account(&state, &username).map_err(|e| e.to_payload())
}
