//! commands/comparendo.rs — Comandos Tauri del módulo de comparendos (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::repositories::comparendo::{Comparendo, ComparendoDatos};
use crate::services::comparendo::{ComparendoService, TotalesComparendos};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_eliminacion, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista comparendos con filtros opcionales (búsqueda, placa, estado o
/// «no confirmados por SIMIT»)
#[tauri::command]
pub fn listar_comparendos(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    placa: Option<String>,
    estado: Option<String>,
    no_confirmados: Option<bool>,
) -> Cmd<Vec<Comparendo>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ComparendoService::listar(
        &mut c,
        busqueda.as_deref(),
        placa.as_deref(),
        estado.as_deref(),
        no_confirmados.unwrap_or(false),
    )
    .map_err(|e| e.to_payload())
}

/// Obtiene un comparendo por id
#[tauri::command]
pub fn obtener_comparendo(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
) -> Cmd<Comparendo> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ComparendoService::obtener(&mut c, id).map_err(|e| e.to_payload())
}

/// Crea un comparendo
#[tauri::command]
pub fn crear_comparendo(
    state: State<'_, AppState>,
    session_id: String,
    datos: ComparendoDatos,
) -> Cmd<Comparendo> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ComparendoService::crear(&mut c, &state.config, datos).map_err(|e| e.to_payload())
}

/// Actualiza un comparendo por id
#[tauri::command]
pub fn actualizar_comparendo(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: ComparendoDatos,
) -> Cmd<Comparendo> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ComparendoService::actualizar(&mut c, &state.config, id, datos).map_err(|e| e.to_payload())
}

/// Marca un comparendo como pagado
#[tauri::command]
pub fn marcar_pagado_comparendo(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
) -> Cmd<Comparendo> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ComparendoService::marcar_pagado(&mut c, id).map_err(|e| e.to_payload())
}

/// Elimina un comparendo
#[tauri::command]
pub fn eliminar_comparendo(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    require_eliminacion(&state, &session_id)?;
    let mut c = conn(&state)?;
    ComparendoService::eliminar(&mut c, id).map_err(|e| e.to_payload())
}

/// Totales general, pendiente, por placa y por estado
#[tauri::command]
pub fn totales_comparendos(
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<TotalesComparendos> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ComparendoService::totales(&mut c).map_err(|e| e.to_payload())
}
