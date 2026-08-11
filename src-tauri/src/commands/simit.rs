//! commands/simit.rs — Comandos Tauri del Agente SIMIT (comparendos automáticos)
//!
//! `simit_sync_status`: estado en memoria del agente (habilitado, intervalos,
//! última sincronización y su resultado).
//! `simit_sync_now`: dispara una sincronización manual en un hilo de bloqueo
//! (el driver de BD es síncrono) para no congelar la UI del webview.

use crate::core::error::{AppError, ErrorPayload};
use crate::services::simit::{
    EstadoAgenteSimit, EstadoAgenteSimitManaged, InfoAgenteSimit, ResultadoSincronizacion,
};
use crate::services::AppState;
use tauri::Manager;
use tauri::State;

use super::{require_eliminacion, require_session};

type Cmd<T> = Result<T, ErrorPayload>;

/// Estado del agente gestionado por Tauri (inicializado en setup de lib.rs)
fn estado_agente(app: &tauri::AppHandle) -> Result<std::sync::Arc<EstadoAgenteSimit>, ErrorPayload> {
    app.try_state::<EstadoAgenteSimitManaged>()
        .map(|s| s.0.clone())
        .ok_or_else(|| {
            AppError::Generic("El Agente SIMIT no está inicializado.".into()).to_payload()
        })
}

/// Estado actual del agente SIMIT (útil al abrir la página de Comparendos)
#[tauri::command]
pub fn simit_sync_status(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<InfoAgenteSimit> {
    require_session(&state, &session_id)?;
    let estado = estado_agente(&app)?;
    Ok(estado.info(&state.config))
}

/// Dispara una sincronización manual contra el SIMIT (asíncrono: corre en
/// `spawn_blocking` porque las operaciones de BD son síncronas).
///
/// Restringido a `roles_con_eliminar` (por defecto Administrador y Supervisor):
/// la corrida consume recursos de red contra el portal y modifica la BD.
#[tauri::command]
pub async fn simit_sync_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<ResultadoSincronizacion> {
    require_eliminacion(&state, &session_id)?;
    let pool = state.pool.clone();
    let cfg = state.config.clone();
    let estado = estado_agente(&app)?;

    // Claim atómico: si la sincronización programada ya corre, no se solapa.
    if !estado.claimar() {
        return Err(AppError::Business(
            "Ya hay una sincronización SIMIT en curso. Espera a que termine.".into(),
        )
        .to_payload());
    }

    tauri::async_runtime::spawn_blocking(move || -> Result<ResultadoSincronizacion, AppError> {
        let resultado = crate::services::simit::run_sync(&pool, &cfg, &estado, Some(&app));
        // Registrar el error en el estado (igual que el scheduler) para que el
        // panel muestre por qué falló la última sincronización.
        if let Err(e) = &resultado {
            estado.registrar_error(&e.to_string());
        }
        estado.liberar();
        resultado
    })
    .await
    .map_err(|e| {
        AppError::Generic(format!("La tarea de sincronización SIMIT falló: {e}")).to_payload()
    })?
    .map_err(|e| e.to_payload())
}
