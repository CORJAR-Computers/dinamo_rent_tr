//! commands/empresa.rs — Configuración de la empresa (setup inicial)
//!
//! - `empresa_publica` (sin sesión): nombre + logo para el login y el menú lateral.
//! - `obtener_empresa` (sesión): configuración completa para la página /empresa
//!   y las impresiones (ContratoRenta, OrdenRenta, OrdenReserva, OrdenComparendo).
//! - `guardar_empresa` (rol de administración): persiste datos + logo.

use crate::core::error::ErrorPayload;
use crate::repositories::empresa::{EmpresaConfig, EmpresaConfigDatos};
use crate::services::empresa::EmpresaService;
use crate::services::AppState;
use tauri::State;

use super::{conn, require_session, require_usuario_admin};

type Cmd<T> = Result<T, ErrorPayload>;

/// Configuración pública (login / menú lateral): solo nombre + logo, sin sesión.
#[tauri::command]
pub fn empresa_publica(state: State<'_, AppState>) -> Cmd<EmpresaConfig> {
    let mut c = conn(&state)?;
    EmpresaService::publica(&mut c, &state.config.data_dir).map_err(|e| e.to_payload())
}

/// Configuración completa de la empresa (requiere sesión activa).
#[tauri::command]
pub fn obtener_empresa(state: State<'_, AppState>, session_id: String) -> Cmd<EmpresaConfig> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    EmpresaService::obtener(&mut c, &state.config.data_dir).map_err(|e| e.to_payload())
}

/// Guarda los datos de la empresa (rol de administración de usuarios).
#[tauri::command]
pub fn guardar_empresa(
    state: State<'_, AppState>,
    session_id: String,
    datos: EmpresaConfigDatos,
) -> Cmd<EmpresaConfig> {
    let sesion = require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    EmpresaService::guardar(&mut c, &state.config.data_dir, datos, &sesion.username)
        .map_err(|e| e.to_payload())
}
