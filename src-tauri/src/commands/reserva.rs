//! commands/reserva.rs — Comandos Tauri del módulo de reservas (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::repositories::reserva::{Reserva, ReservaDatos};
use crate::services::reserva::{ReservaCancelada, ReservaService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_eliminacion, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista reservas con filtros opcionales (búsqueda o estado)
#[tauri::command]
pub fn listar_reservas(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    estado: Option<String>,
) -> Cmd<Vec<Reserva>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ReservaService::listar(&mut c, busqueda.as_deref(), estado.as_deref())
        .map_err(|e| e.to_payload())
}

/// Próximas reservas (recogida hoy o en el futuro, no canceladas)
#[tauri::command]
pub fn proximas_reservas(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<i64>,
) -> Cmd<Vec<Reserva>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ReservaService::proximas(&mut c, limit.unwrap_or(10)).map_err(|e| e.to_payload())
}

/// Obtiene una reserva por id
#[tauri::command]
pub fn obtener_reserva(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<Reserva> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ReservaService::obtener(&mut c, id).map_err(|e| e.to_payload())
}

/// Crea una reserva
#[tauri::command]
pub fn crear_reserva(
    state: State<'_, AppState>,
    session_id: String,
    datos: ReservaDatos,
) -> Cmd<Reserva> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ReservaService::crear(&mut c, &state.config, datos).map_err(|e| e.to_payload())
}

/// Actualiza una reserva por id
#[tauri::command]
pub fn actualizar_reserva(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: ReservaDatos,
) -> Cmd<Reserva> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ReservaService::actualizar(&mut c, &state.config, id, datos).map_err(|e| e.to_payload())
}

/// Cancela una reserva (no se puede cancelar una ya completada)
#[tauri::command]
pub fn cancelar_reserva(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
) -> Cmd<ReservaCancelada> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ReservaService::cancelar(&mut c, id).map_err(|e| e.to_payload())
}

/// Elimina una reserva
#[tauri::command]
pub fn eliminar_reserva(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    let session = require_eliminacion(&state, &session_id)?;
    let mut c = conn(&state)?;
    ReservaService::eliminar(&mut c, &session.username, id).map_err(|e| e.to_payload())
}
