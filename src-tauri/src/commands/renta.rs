//! commands/renta.rs — Comandos Tauri del módulo de rentas (thin wrappers)

use crate::core::error::{AppError, ErrorPayload};
use crate::repositories::extension::ExtensionRentaRepository;
use crate::repositories::renta::{
    ExtensionDatos, Inspeccion, InspeccionDatos, Pago, PagoDatos, Renta, RentaCierreDatos,
    RentaCierreEditDatos, RentaDatos,
};
use crate::services::renta::{RentaCancelada, RentaService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_eliminacion, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista rentas con filtros opcionales (búsqueda, estado o placa).
///
/// TAREA 3.4 (Bloque 3 — Performance): el comando es `async` y corre la
/// consulta en `tauri::async_runtime::spawn_blocking` para no bloquear el
/// event loop de Tauri. `listar_rentas` puede aplicar filtros LIKE sobre
/// `nombre_cliente` y `placa` que escanean la tabla completa si no hay índice
/// funcional — conviene no retener el hilo del runtime mientras Firebird
/// resuelve el WHERE.
#[tauri::command]
pub async fn listar_rentas(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    estado: Option<String>,
    placa: Option<String>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
) -> Cmd<Vec<Renta>> {
    require_session(&state, &session_id)?;
    let pool = state.pool.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<Renta>, AppError> {
        let mut c = pool.get().map_err(AppError::from)?;
        RentaService::listar(
            &mut c,
            busqueda.as_deref(),
            estado.as_deref(),
            placa.as_deref(),
            fecha_desde.as_deref(),
            fecha_hasta.as_deref(),
        )
    })
    .await
    .map_err(|e| AppError::Generic(format!("La tarea listar_rentas falló: {e}")).to_payload())?
    .map_err(|e| e.to_payload())
}

/// Obtiene una renta por id (con pagos e inspecciones; ejecutado asíncronamente en spawn_blocking).
#[tauri::command]
pub async fn obtener_renta(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<Renta> {
    require_session(&state, &session_id)?;
    let pool = state.pool.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Renta, AppError> {
        let mut c = pool.get().map_err(AppError::from)?;
        RentaService::obtener(&mut c, id)
    })
    .await
    .map_err(|e| AppError::Generic(format!("La tarea obtener_renta falló: {e}")).to_payload())?
    .map_err(|e| e.to_payload())
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

/// Cambia el vehículo asignado a una renta activa sin cerrarla
#[tauri::command]
pub fn cambiar_auto_renta(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    placa: String,
) -> Cmd<Renta> {
    let sesion = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::cambiar_auto(&mut c, id, &placa, &sesion.username).map_err(|e| e.to_payload())
}

/// Cierra una renta con la devolución real y recalcula los totales (ejecutado asíncronamente en spawn_blocking).
#[tauri::command]
pub async fn cerrar_renta(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: RentaCierreDatos,
) -> Cmd<Renta> {
    let sesion = require_session(&state, &session_id)?;
    let pool = state.pool.clone();
    let config = state.config.clone();
    let username = sesion.username;
    tauri::async_runtime::spawn_blocking(move || -> Result<Renta, AppError> {
        let mut c = pool.get().map_err(AppError::from)?;
        RentaService::cerrar(&mut c, &config, id, &username, datos)
    })
    .await
    .map_err(|e| AppError::Generic(format!("La tarea cerrar_renta falló: {e}")).to_payload())?
    .map_err(|e| e.to_payload())
}

/// Cancela una renta activa
#[tauri::command]
pub fn cancelar_renta(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
) -> Cmd<RentaCancelada> {
    let sesion = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::cancelar(&mut c, id, &sesion.username).map_err(|e| e.to_payload())
}

/// Extiende una renta activa agregando horas o días extras
#[tauri::command]
pub fn extender_renta(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: ExtensionDatos,
) -> Cmd<Renta> {
    let sesion = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::extender(&mut c, &state.config, id, &sesion.username, datos)
        .map_err(|e| e.to_payload())
}

/// Lista extensiones de una renta
#[tauri::command]
pub fn listar_extensiones(
    state: State<'_, AppState>,
    session_id: String,
    id_renta: i64,
) -> Cmd<Vec<crate::repositories::extension::ExtensionRenta>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    ExtensionRentaRepository::listar_por_renta(&mut c, id_renta).map_err(|e| e.to_payload())
}

/// Edita campos financieros de una renta cerrada (solo Administrador; ejecutado asíncronamente en spawn_blocking).
#[tauri::command]
pub async fn editar_renta_cerrada(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: RentaCierreEditDatos,
) -> Cmd<Renta> {
    let sesion = require_eliminacion(&state, &session_id)?;
    let pool = state.pool.clone();
    let config = state.config.clone();
    let username = sesion.username;
    tauri::async_runtime::spawn_blocking(move || -> Result<Renta, AppError> {
        let mut c = pool.get().map_err(AppError::from)?;
        RentaService::editar_cerrada(&mut c, &config, id, &username, datos)
    })
    .await
    .map_err(|e| {
        AppError::Generic(format!("La tarea editar_renta_cerrada falló: {e}")).to_payload()
    })?
    .map_err(|e| e.to_payload())
}

/// Elimina una renta (pagos e inspecciones en cascada)
#[tauri::command]
pub fn eliminar_renta(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    let sesion = require_eliminacion(&state, &session_id)?;
    let mut c = conn(&state)?;
    RentaService::eliminar(&mut c, id, &sesion.username).map_err(|e| e.to_payload())
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
    RentaService::registrar_pago(&mut c, id_renta, &sesion.username, datos)
        .map_err(|e| e.to_payload())
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
