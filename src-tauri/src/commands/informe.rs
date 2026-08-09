//! commands/informe.rs — Comandos Tauri del módulo de informes (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::services::informe::{InformeMensual, InformeService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Balance mensual por rango de fechas
#[tauri::command]
pub fn informe_mensual(
    state: State<'_, AppState>,
    session_id: String,
    fecha_inicio: String,
    fecha_fin: String,
) -> Cmd<InformeMensual> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    InformeService::mensual(&mut c, &fecha_inicio, &fecha_fin).map_err(|e| e.to_payload())
}
