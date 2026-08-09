//! commands/business.rs — Listas de negocio (tipos, estados) para los formularios

use crate::core::config::BusinessLists;
use crate::core::error::ErrorPayload;
use crate::services::AppState;
use tauri::State;

use super::require_session;

type Cmd<T> = Result<T, ErrorPayload>;

/// Listas configurables de negocio (tipos de auto, estados, documentos, etc.)
#[tauri::command]
pub fn get_business_lists(state: State<'_, AppState>, session_id: String) -> Cmd<BusinessLists> {
    require_session(&state, &session_id)?;
    Ok(state.config.business_lists())
}
