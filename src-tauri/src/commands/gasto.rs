//! commands/gasto.rs — Comandos Tauri del módulo de gastos (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::repositories::gasto::{Gasto, GastoDatos};
use crate::services::gasto::{GastoService, TotalesGastos};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_eliminacion, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista gastos con filtros opcionales (búsqueda, placa o categoría)
#[tauri::command]
pub fn listar_gastos(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    placa: Option<String>,
    categoria: Option<String>,
) -> Cmd<Vec<Gasto>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    GastoService::listar(
        &mut c,
        busqueda.as_deref(),
        placa.as_deref(),
        categoria.as_deref(),
    )
    .map_err(|e| e.to_payload())
}

/// Gastos recientes (para el inicio o un panel)
#[tauri::command]
pub fn gastos_recientes(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<i64>,
) -> Cmd<Vec<Gasto>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    GastoService::recientes(&mut c, limit.unwrap_or(10)).map_err(|e| e.to_payload())
}

/// Obtiene un gasto por id
#[tauri::command]
pub fn obtener_gasto(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<Gasto> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    GastoService::obtener(&mut c, id).map_err(|e| e.to_payload())
}

/// Crea un gasto (registra al usuario de la sesión como autor)
#[tauri::command]
pub fn crear_gasto(
    state: State<'_, AppState>,
    session_id: String,
    datos: GastoDatos,
) -> Cmd<Gasto> {
    let session = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    GastoService::crear(&mut c, &state.config, &session.username, datos).map_err(|e| e.to_payload())
}

/// Actualiza un gasto por id
#[tauri::command]
pub fn actualizar_gasto(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: GastoDatos,
) -> Cmd<Gasto> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    GastoService::actualizar(&mut c, &state.config, id, datos).map_err(|e| e.to_payload())
}

/// Elimina un gasto
#[tauri::command]
pub fn eliminar_gasto(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    require_eliminacion(&state, &session_id)?;
    let mut c = conn(&state)?;
    GastoService::eliminar(&mut c, id).map_err(|e| e.to_payload())
}

/// Totales general, por placa y por categoría
#[tauri::command]
pub fn totales_gastos(state: State<'_, AppState>, session_id: String) -> Cmd<TotalesGastos> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    GastoService::totales(&mut c).map_err(|e| e.to_payload())
}
