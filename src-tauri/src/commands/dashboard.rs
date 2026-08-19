//! commands/dashboard.rs — Comando Tauri del Dashboard (thin wrapper)

use crate::core::error::ErrorPayload;
use crate::services::dashboard::{DashboardData, DashboardService};
use crate::services::AppState;
use tauri::State;

use super::require_session;

type Cmd<T> = Result<T, ErrorPayload>;

/// KPIs agregados para la pantalla de inicio.
///
/// TODO Tarea 3.4 (Bloque 3 — Performance): envolver en
/// `tauri::async_runtime::spawn_blocking`. `get_dashboard_data` agrega
/// total_autos + autos_por_estado + total_clientes + clientes_recientes +
/// alertas_vencimiento + rentas_activas en una sola invocación (6-8 queries
/// según `DashboardService`) — es el comando más pesado del módulo de
/// dashboard y se ejecuta en cada navegación a `/`. Mientras tanto sigue
/// siendo síncrono porque `DashboardService::get_dashboard_data` toma
/// `&AppState` (no se puede mover al closure sin refactor).
#[tauri::command]
pub fn get_dashboard_data(
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<DashboardData> {
    require_session(&state, &session_id)?;
    DashboardService::get_dashboard_data(&state).map_err(|e| e.to_payload())
}
