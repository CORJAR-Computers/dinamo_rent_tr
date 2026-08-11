//! commands/informe.rs — Comandos Tauri del módulo de informes (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::services::informe::{InformeMensual, InformeService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_informes};

type Cmd<T> = Result<T, ErrorPayload>;

/// Balance mensual por rango de fechas
/// (solo roles de `roles_con_informes` en config.ini — por defecto Admin y Supervisor).
#[tauri::command]
pub fn informe_mensual(
    state: State<'_, AppState>,
    session_id: String,
    fecha_inicio: String,
    fecha_fin: String,
) -> Cmd<InformeMensual> {
    require_informes(&state, &session_id)?;
    let mut c = conn(&state)?;
    InformeService::mensual(&mut c, &fecha_inicio, &fecha_fin).map_err(|e| e.to_payload())
}
