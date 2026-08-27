//! commands/dashboard.rs — Comando Tauri del Dashboard (thin wrapper)

use crate::core::error::{AppError, ErrorPayload};
use crate::services::dashboard::{DashboardData, DashboardService};
use crate::services::AppState;
use tauri::State;

use super::require_session;

type Cmd<T> = Result<T, ErrorPayload>;

/// KPIs agregados para la pantalla de inicio (ejecutado asíncronamente en spawn_blocking).
#[tauri::command]
pub async fn get_dashboard_data(
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<DashboardData> {
    require_session(&state, &session_id)?;
    let pool = state.pool.clone();
    let config = state.config.clone();
    let pii_key = state.pii_key();
    let pii_key_configurada = state.pii_key_configurada();
    tauri::async_runtime::spawn_blocking(move || -> Result<DashboardData, AppError> {
        let mut conn = pool.get().map_err(AppError::from)?;
        DashboardService::calcular_dashboard_data(&mut conn, &config, &pii_key, pii_key_configurada)
    })
    .await
    .map_err(|e| AppError::Generic(format!("La tarea get_dashboard_data falló: {e}")).to_payload())?
    .map_err(|e| e.to_payload())
}
