//! commands/cliente.rs — Comandos Tauri del módulo de clientes (thin wrappers)

use crate::core::crypto::PiiCipher;
use crate::core::error::ErrorPayload;
use crate::repositories::cliente::ClienteDatos;
use crate::services::cliente::{ClienteConPii, ClienteService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_eliminacion, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista clientes con filtros opcionales (búsqueda o estado)
#[tauri::command]
pub fn listar_clientes(
    state: State<'_, AppState>,
    session_id: String,
    busqueda: Option<String>,
    estado: Option<String>,
) -> Cmd<Vec<ClienteConPii>> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    let cipher = PiiCipher::new(&state.pii_key());
    ClienteService::listar(&mut c, &cipher, busqueda.as_deref(), estado.as_deref())
        .map_err(|e| e.to_payload())
}

/// Obtiene un cliente por id
#[tauri::command]
pub fn obtener_cliente(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
) -> Cmd<ClienteConPii> {
    require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    let cipher = PiiCipher::new(&state.pii_key());
    ClienteService::obtener(&mut c, &cipher, id).map_err(|e| e.to_payload())
}

/// Crea un cliente
#[tauri::command]
pub fn crear_cliente(
    state: State<'_, AppState>,
    session_id: String,
    datos: ClienteDatos,
) -> Cmd<ClienteConPii> {
    let session = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    let cipher = PiiCipher::new(&state.pii_key());
    ClienteService::crear(&mut c, &state.config, &cipher, &session.username, datos).map_err(|e| e.to_payload())
}

/// Actualiza un cliente por id
#[tauri::command]
pub fn actualizar_cliente(
    state: State<'_, AppState>,
    session_id: String,
    id: i64,
    datos: ClienteDatos,
) -> Cmd<ClienteConPii> {
    let session = require_session(&state, &session_id)?;
    let mut c = conn(&state)?;
    let cipher = PiiCipher::new(&state.pii_key());
    ClienteService::actualizar(&mut c, &state.config, &cipher, &session.username, id, datos)
        .map_err(|e| e.to_payload())
}

/// Elimina un cliente por id
#[tauri::command]
pub fn eliminar_cliente(state: State<'_, AppState>, session_id: String, id: i64) -> Cmd<()> {
    let session = require_eliminacion(&state, &session_id)?;
    let mut c = conn(&state)?;
    ClienteService::eliminar(&mut c, &session.username, id).map_err(|e| e.to_payload())
}
