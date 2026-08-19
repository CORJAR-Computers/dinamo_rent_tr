//! commands/ — Wrappers #[tauri::command] (thin: validan sesión/RBAC, llaman servicio, mapean error)

pub mod app;
pub mod auditoria;
pub mod auth;
pub mod auto;
pub mod backup;
pub mod business;
pub mod comparendo;
pub mod cliente;
pub mod dashboard;
pub mod empresa;
pub mod gasto;
pub mod informe;
pub mod logs;
pub mod mantenimiento;
pub mod pii;
pub mod reserva;
pub mod renta;
pub mod simit;
pub mod usuario;

use crate::core::error::{AppError, ErrorPayload};
use crate::core::rbac::SessionData;
use crate::core::PooledConnection;
use crate::services::AppState;

/// Requiere sesión activa y devuelve los datos de la sesión
pub fn require_session(state: &AppState, session_id: &str) -> Result<SessionData, ErrorPayload> {
    let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
    crate::core::rbac::require_active_session(&mut sessions, session_id)
        .map_err(|e| e.to_payload())
}

/// Requiere sesión activa con rol de administración de usuarios
/// (roles_con_usuarios en config.ini — por defecto solo Administrador).
pub fn require_usuario_admin(state: &AppState, session_id: &str) -> Result<SessionData, ErrorPayload> {
    let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
    // Fallback: si config.ini no define roles_con_usuarios, solo Administrador
    let roles: Vec<&str> = if state.config.roles_con_usuarios.is_empty() {
        vec!["Administrador"]
    } else {
        state.config.roles_con_usuarios.iter().map(|s| s.as_str()).collect()
    };
    crate::core::rbac::require_role(&mut sessions, session_id, &roles)
        .map_err(|e| e.to_payload())
}

/// Requiere sesión activa con rol habilitado para informes
/// (roles_con_informes en config.ini — por defecto solo Administrador).
pub fn require_informes(state: &AppState, session_id: &str) -> Result<SessionData, ErrorPayload> {
    let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
    // Fallback: si config.ini no define roles_con_informes, solo Admin
    let roles: Vec<&str> = if state.config.roles_con_informes.is_empty() {
        vec!["Administrador"]
    } else {
        state.config.roles_con_informes.iter().map(|s| s.as_str()).collect()
    };
    crate::core::rbac::require_role(&mut sessions, session_id, &roles)
        .map_err(|e| e.to_payload())
}

/// Requiere sesión activa con rol habilitado para eliminar registros
/// (roles_con_eliminar en config.ini — por defecto Administrador y Supervisor).
pub fn require_eliminacion(state: &AppState, session_id: &str) -> Result<SessionData, ErrorPayload> {
    let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
    // Fallback: si config.ini no define roles_con_eliminar, Admin + Supervisor
    let roles: Vec<&str> = if state.config.roles_con_eliminar.is_empty() {
        vec!["Administrador", "Supervisor"]
    } else {
        state.config.roles_con_eliminar.iter().map(|s| s.as_str()).collect()
    };
    crate::core::rbac::require_role(&mut sessions, session_id, &roles)
        .map_err(|e| e.to_payload())
}

/// Obtiene una conexión del pool
pub fn conn(state: &AppState) -> Result<PooledConnection, ErrorPayload> {
    state.pool.get().map_err(|e| AppError::from(e).to_payload())
}
