//! commands/auth.rs — Comandos Tauri de autenticación

use crate::core::error::ErrorPayload;
use crate::services::auth::{AuthService, LoginResult, LoginStatus};
use crate::services::AppState;
use tauri::State;

type Cmd<T> = Result<T, ErrorPayload>;

/// Login — no requiere sesión previa
#[tauri::command]
pub fn login(state: State<'_, AppState>, username: String, password: String) -> Cmd<LoginResult> {
    // En Tauri desktop, "IP" = local. Se registra con "local" para auditoría.
    AuthService::login(&state, &username, &password, Some("local")).map_err(|e| e.to_payload())
}

/// Cierra la sesión actual
#[tauri::command]
pub fn logout(state: State<'_, AppState>, session_id: String) {
    AuthService::logout(&state, &session_id);
}

/// Cambia la contraseña del usuario (autenticado o cambio obligatorio)
#[tauri::command]
pub fn change_password(
    state: State<'_, AppState>,
    username: String,
    current_password: String,
    new_password: String,
) -> Cmd<()> {
    AuthService::cambiar_password(&state, &username, &current_password, &new_password)
        .map_err(|e| e.to_payload())
}

/// Estado de login de un usuario (intentos restantes, bloqueo) — pre-login
#[tauri::command]
pub fn get_login_status(state: State<'_, AppState>, username: String) -> LoginStatus {
    AuthService::get_login_status(&state, &username)
}

/// Estado de la sesión actual (para guard de rutas / refresh de UI)
#[tauri::command]
pub fn get_session(state: State<'_, AppState>, session_id: String) -> Cmd<crate::core::rbac::SessionData> {
    let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
    crate::core::rbac::require_active_session(&mut sessions, &session_id).map_err(|e| e.to_payload())
}

/// Preferencia de tema del usuario conectado ('light' | 'dark' | 'auto' | null)
#[tauri::command]
pub fn obtener_tema(state: State<'_, AppState>, session_id: String) -> Cmd<Option<String>> {
    let session = super::require_session(&state, &session_id)?;
    let mut c = super::conn(&state)?;
    crate::repositories::usuario::UsuarioRepository::obtener_tema(&mut c, session.user_id)
        .map_err(|e| e.to_payload())
}

/// Guarda la preferencia de tema del usuario conectado
#[tauri::command]
pub fn guardar_tema(state: State<'_, AppState>, session_id: String, tema: String) -> Cmd<()> {
    let session = super::require_session(&state, &session_id)?;
    if !matches!(tema.as_str(), "light" | "dark" | "auto") {
        return Err(crate::core::error::AppError::Validation(
            "Tema inválido. Valores permitidos: light, dark, auto.".into(),
        )
        .to_payload());
    }
    let mut c = super::conn(&state)?;
    crate::repositories::usuario::UsuarioRepository::guardar_tema(&mut c, session.user_id, &tema)
        .map_err(|e| e.to_payload())
}
