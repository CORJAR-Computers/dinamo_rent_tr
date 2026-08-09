//! commands/mantenimiento.rs — Comandos Tauri del módulo de mantenimiento (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::repositories::mantenimiento::{Mantenimiento, MantenimientoDatos};
use crate::services::mantenimiento::{AlertaKm, MantenimientoService, TotalesMantenimiento};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista mantenimientos con filtros opcionales (búsqueda, placa o tipo)
#[tauri::command]
pub fn listar_mantenimientos(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    placa: Option<String>,
    tipo: Option<String>,
) -> Cmd<Vec<Mantenimiento>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::listar(&mut c, busqueda.as_deref(), placa.as_deref(), tipo.as_deref())
        .map_err(|e| e.to_payload())
}

/// Mantenimientos recientes
#[tauri::command]
pub fn mantenimientos_recientes(
    state: State<'_, AppState>,
    session_id: String,
    limit: Option<i64>,
) -> Cmd<Vec<Mantenimiento>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::recientes(&mut c, limit.unwrap_or(10)).map_err(|e| e.to_payload())
}

/// Obtiene un mantenimiento por id
#[tauri::command]
pub fn obtener_mantenimiento(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<Mantenimiento> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::obtener(&mut c, id).map_err(|e| e.to_payload())
}

/// Crea un mantenimiento
#[tauri::command]
pub fn crear_mantenimiento(
    state: State<'_, AppState>,
    session_id: String,
    datos: MantenimientoDatos,
) -> Cmd<Mantenimiento> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::crear(&mut c, &state.config, datos).map_err(|e| e.to_payload())
}

/// Actualiza un mantenimiento por id
#[tauri::command]
pub fn actualizar_mantenimiento(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: MantenimientoDatos,
) -> Cmd<Mantenimiento> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::actualizar(&mut c, &state.config, id, datos).map_err(|e| e.to_payload())
}

/// Elimina un mantenimiento
#[tauri::command]
pub fn eliminar_mantenimiento(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::eliminar(&mut c, id).map_err(|e| e.to_payload())
}

/// Totales general, por placa y por tipo
#[tauri::command]
pub fn totales_mantenimiento(state: State<'_, AppState>, session_id: String) -> Cmd<TotalesMantenimiento> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::totales(&mut c).map_err(|e| e.to_payload())
}

/// Alertas por kilometraje (cambio de aceite y frenos próximos o vencidos)
#[tauri::command]
pub fn alertas_km_mantenimiento(state: State<'_, AppState>, session_id: String) -> Cmd<Vec<AlertaKm>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    MantenimientoService::alertas_km(&mut c, &state.config).map_err(|e| e.to_payload())
}
