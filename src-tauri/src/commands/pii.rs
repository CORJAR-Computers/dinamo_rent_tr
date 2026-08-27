//! commands/pii.rs — Comandos de gestión de la clave PII (db_encryption_key)
//!
//! Solo administradores (roles_con_usuarios): consultar estado, probar una
//! clave candidata, guardarla y eliminarla.

use crate::core::error::ErrorPayload;
use crate::services::pii::{ClaveGuardada, PiiAnalisis, PiiService};
use crate::services::AppState;
use tauri::State;

use super::require_usuario_admin;

type Cmd<T> = Result<T, ErrorPayload>;

/// Estado actual del descifrado PII (clave configurada, clientes legacy, etc.)
#[tauri::command]
pub fn get_pii_status(state: State<'_, AppState>, session_id: String) -> Cmd<PiiAnalisis> {
    require_usuario_admin(&state, &session_id)?;
    PiiService::estado(&state).map_err(|e| e.to_payload())
}

/// Prueba una clave candidata SIN guardarla (muestra cuántos clientes descifra)
#[tauri::command]
pub fn probar_clave_pii(
    state: State<'_, AppState>,
    session_id: String,
    clave: String,
) -> Cmd<PiiAnalisis> {
    require_usuario_admin(&state, &session_id)?;
    PiiService::probar_clave(&state, &clave).map_err(|e| e.to_payload())
}

/// Guarda la clave en config.ini y la aplica en caliente
#[tauri::command]
pub fn guardar_clave_pii(
    state: State<'_, AppState>,
    session_id: String,
    clave: String,
) -> Cmd<ClaveGuardada> {
    require_usuario_admin(&state, &session_id)?;
    PiiService::guardar_clave(&state, &clave).map_err(|e| e.to_payload())
}

/// Elimina la clave configurada (vuelve a ocultar los datos Fernet)
#[tauri::command]
pub fn eliminar_clave_pii(state: State<'_, AppState>, session_id: String) -> Cmd<ClaveGuardada> {
    require_usuario_admin(&state, &session_id)?;
    PiiService::eliminar_clave(&state).map_err(|e| e.to_payload())
}
