//! commands/auto.rs — Comandos Tauri del módulo de vehículos (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::repositories::auto::{Auto, AutoDatos};
use crate::services::auto::{AlertaVencimiento, AutoService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_eliminacion, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista vehículos con filtros opcionales (búsqueda o estado)
#[tauri::command]
pub fn listar_autos(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    estado: Option<String>,
) -> Cmd<Vec<Auto>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    AutoService::listar(&mut c, busqueda.as_deref(), estado.as_deref()).map_err(|e| e.to_payload())
}

/// Obtiene un vehículo por placa
#[tauri::command]
pub fn obtener_auto(state: State<'_, AppState>, session_id: String, placa: String) -> Cmd<Auto> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    AutoService::obtener(&mut c, &placa).map_err(|e| e.to_payload())
}

/// Crea un vehículo
#[tauri::command]
pub fn crear_auto(state: State<'_, AppState>, session_id: String, datos: AutoDatos) -> Cmd<Auto> {
    let session = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    AutoService::crear(&mut c, &state.config, &session.username, datos).map_err(|e| e.to_payload())
}

/// Actualiza un vehículo por placa
#[tauri::command]
pub fn actualizar_auto(
    state: State<'_, AppState>,
    session_id: String,
    placa: String,
    datos: AutoDatos,
) -> Cmd<Auto> {
    let session = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    AutoService::actualizar(&mut c, &state.config, &session.username, &placa, datos)
        .map_err(|e| e.to_payload())
}

/// Elimina un vehículo por placa
#[tauri::command]
pub fn eliminar_auto(state: State<'_, AppState>, session_id: String, placa: String) -> Cmd<()> {
    let session = require_eliminacion(&state, &session_id)?;
    let mut c = conn(&state)?;
    AutoService::eliminar(&mut c, &session.username, &placa).map_err(|e| e.to_payload())
}

/// Alertas de vencimientos (SOAT, técnico, extintor, batería, aceite)
#[tauri::command]
pub fn alertas_autos(
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<Vec<AlertaVencimiento>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    AutoService::alertas_vencimiento(&mut c, &state.config).map_err(|e| e.to_payload())
}
