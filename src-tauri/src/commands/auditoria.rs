//! commands/auditoria.rs — Comandos Tauri de la vista de auditoría
//!
//! Acceso restringido a roles de administración de usuarios (roles_con_usuarios).

use crate::core::error::{AppError, ErrorPayload};
use crate::repositories::auditoria::AuditoriaFiltros;
use crate::services::auditoria::{AuditoriaResultado, AuditoriaService};
use crate::services::AppState;
use tauri::State;

use super::{conn, require_usuario_admin};

type Cmd<T> = Result<T, ErrorPayload>;

/// Lista eventos de auditoría con filtros opcionales y paginación (solo admin; ejecutado asíncronamente en spawn_blocking).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn listar_auditoria(
    state: State<'_, AppState>,
    session_id: String,
    usuario: Option<String>,
    accion: Option<String>,
    fecha_desde: Option<String>,
    fecha_hasta: Option<String>,
    busqueda: Option<String>,
    pagina: Option<i64>,
    por_pagina: Option<i64>,
) -> Cmd<AuditoriaResultado> {
    require_usuario_admin(&state, &session_id)?;
    let pool = state.pool.clone();
    let filtros = AuditoriaFiltros {
        usuario,
        accion,
        fecha_desde,
        fecha_hasta,
        busqueda,
    };
    tauri::async_runtime::spawn_blocking(move || -> Result<AuditoriaResultado, AppError> {
        let mut c = pool.get().map_err(AppError::from)?;
        AuditoriaService::listar(&mut c, filtros, pagina, por_pagina)
    })
    .await
    .map_err(|e| AppError::Generic(format!("La tarea listar_auditoria falló: {e}")).to_payload())?
    .map_err(|e| e.to_payload())
}

/// Acciones distintas disponibles para el filtro (solo admin)
#[tauri::command]
pub fn acciones_auditoria(state: State<'_, AppState>, session_id: String) -> Cmd<Vec<String>> {
    require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    AuditoriaService::acciones(&mut c).map_err(|e| e.to_payload())
}

/// Usuarios distintos disponibles para el filtro (solo admin)
#[tauri::command]
pub fn usuarios_auditoria(state: State<'_, AppState>, session_id: String) -> Cmd<Vec<String>> {
    require_usuario_admin(&state, &session_id)?;
    let mut c = conn(&state)?;
    AuditoriaService::usuarios(&mut c).map_err(|e| e.to_payload())
}
