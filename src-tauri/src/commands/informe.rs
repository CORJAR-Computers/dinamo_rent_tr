//! commands/informe.rs — Comandos Tauri del módulo de informes (thin wrappers)

use crate::core::error::{AppError, ErrorPayload};
use crate::services::informe::{InformeMensual, InformeService};
use crate::services::AppState;
use tauri::State;

use super::require_informes;

type Cmd<T> = Result<T, ErrorPayload>;

/// Balance mensual por rango de fechas.
///
/// TAREA 3.4 (Bloque 3 — Performance): el comando es `async` y corre la
/// consulta en `tauri::async_runtime::spawn_blocking` para no bloquear el
/// event loop de Tauri mientras Firebird Embedded ejecuta las 5 queries
/// agregadas (UNION ALL consolidado en TAREA 3.1). El patrón es el mismo que
/// usan `backup_ahora` y `simit_sync_now`.
///
/// (solo roles de `roles_con_informes` en config.ini — por defecto solo Admin).
#[tauri::command]
pub async fn informe_mensual(
    state: State<'_, AppState>,
    session_id: String,
    fecha_inicio: String,
    fecha_fin: String,
) -> Cmd<InformeMensual> {
    require_informes(&state, &session_id)?;
    let pool = state.pool.clone();
    tauri::async_runtime::spawn_blocking(
        move || -> Result<InformeMensual, AppError> {
            let mut c = pool.get().map_err(AppError::from)?;
            InformeService::mensual(&mut c, &fecha_inicio, &fecha_fin)
        },
    )
    .await
    .map_err(|e| AppError::Generic(format!("La tarea del informe falló: {e}")).to_payload())?
    .map_err(|e| e.to_payload())
}
