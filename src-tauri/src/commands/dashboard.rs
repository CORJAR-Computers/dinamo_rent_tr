//! commands/dashboard.rs — Comando Tauri del Dashboard (thin wrapper)

use crate::core::error::ErrorPayload;
use crate::services::dashboard::{DashboardData, DashboardService};
use crate::services::AppState;
use tauri::State;

use super::require_session;

type Cmd<T> = Result<T, ErrorPayload>;

/// KPIs agregados para la pantalla de inicio
#[tauri::command]
pub fn get_dashboard_data(
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<DashboardData> {
    require_session(&state, &session_id)?;
    DashboardService::get_dashboard_data(&state).map_err(|e| e.to_payload())
}
