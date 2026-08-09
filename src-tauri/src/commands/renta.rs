//! commands/renta.rs — Comandos Tauri del módulo de rentas (thin wrappers)

use crate::core::error::ErrorPayload;
use crate::repositories::renta::{
    Inspeccion, InspeccionDatos, Pago, PagoDatos, Renta, RentaCierreDatos, RentaDatos,
};
use crate::services::renta::{RentaCancelada, RentaService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista rentas con filtros opcionales (búsqueda, estado o placa)
#[tauri::command]
pub fn listar_rentas(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    estado: Option<String>,
    placa: Option<String>,
) -> Cmd<Vec<Renta>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::listar(&mut c, busqueda.as_deref(), estado.as_deref(), placa.as_deref())
        .map_err(|e| e.to_payload())
}

/// Obtiene una renta por id (con pagos e inspecciones)
#[tauri::command]
pub fn obtener_renta(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<Renta> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::obtener(&mut c, id).map_err(|e| e.to_payload())
}

/// Crea una renta
#[tauri::command]
pub fn crear_renta(
    state: State<'_, AppState>,
    session_id: String,
    datos: RentaDatos,
) -> Cmd<Renta> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::crear(&mut c, &state.config, datos).map_err(|e| e.to_payload())
}

/// Actualiza una renta (no cerradas)
#[tauri::command]
pub fn actualizar_renta(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: RentaDatos,
) -> Cmd<Renta> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::actualizar(&mut c, &state.config, id, datos).map_err(|e| e.to_payload())
}

/// Cierra una renta con la devolución real y recalcula los totales
#[tauri::command]
pub fn cerrar_renta(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: RentaCierreDatos,
) -> Cmd<Renta> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::cerrar(&mut c, &state.config, id, datos).map_err(|e| e.to_payload())
}

/// Cancela una renta activa
#[tauri::command]
pub fn cancelar_renta(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
) -> Cmd<RentaCancelada> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::cancelar(&mut c, id).map_err(|e| e.to_payload())
}

/// Elimina una renta (pagos e inspecciones en cascada)
#[tauri::command]
pub fn eliminar_renta(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::eliminar(&mut c, id).map_err(|e| e.to_payload())
}

/// Registra un pago contra una renta activa (actualiza abono/saldo)
#[tauri::command]
pub fn registrar_pago_renta(
    state: State<'_, AppState>,
    session_id: String,
    id_renta: i64,
    datos: PagoDatos,
) -> Cmd<Pago> {
    let sesion = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::registrar_pago(&mut c, id_renta, &sesion.username, datos).map_err(|e| e.to_payload())
}

/// Registra una inspección (Salida/Entrada) de una renta
#[tauri::command]
pub fn registrar_inspeccion_renta(
    state: State<'_, AppState>,
    session_id: String,
    id_renta: i64,
    datos: InspeccionDatos,
) -> Cmd<Inspeccion> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::registrar_inspeccion(&mut c, id_renta, datos).map_err(|e| e.to_payload())
}

/// Rentas activas (para el calendario y el dashboard)
#[tauri::command]
pub fn rentas_activas(state: State<'_, AppState>, session_id: String) -> Cmd<Vec<Renta>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::activas(&mut c).map_err(|e| e.to_payload())
}
