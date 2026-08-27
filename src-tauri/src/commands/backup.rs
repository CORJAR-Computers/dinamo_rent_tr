//! commands/backup.rs — Comandos Tauri de los backups (Fase 8 del plan)
//!
//! `backup_estado`: estado en memoria + configuración + copias en disco
//! (directorio, horarios, última corrida, próxima corrida, cifrado).
//! `backup_ahora`: crea un backup manual en un hilo de bloqueo (gbak/copia +
//! rotación, y cifrado si está habilitado) sin congelar la UI del webview.
//!
//! Restringidos a Administrador (`roles_con_usuarios` de config.ini): los
//! backups contienen TODA la base de datos (y la contraseña de cifrado es
//! sensible), así que no se exponen a Operadores ni Supervisores.

use std::sync::Arc;

use crate::core::error::{AppError, ErrorPayload};
use crate::services::backup::{EstadoBackup, EstadoBackupManaged, InfoBackup};
use crate::services::AppState;
use tauri::Manager;
use tauri::State;

use super::require_usuario_admin;

type Cmd<T> = Result<T, ErrorPayload>;

/// Estado de los backups gestionado por Tauri (inicializado en setup de lib.rs)
fn estado_backup<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Arc<EstadoBackup>, ErrorPayload> {
    app.try_state::<EstadoBackupManaged>()
        .map(|s| s.0.clone())
        .ok_or_else(|| {
            AppError::Generic("El servicio de backups no está inicializado.".into()).to_payload()
        })
}

/// Estado actual de los backups (config + última corrida + copias en disco).
/// Se consulta al abrir la página de Backups y tras crear un backup manual.
#[tauri::command]
pub fn backup_estado(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<InfoBackup> {
    require_usuario_admin(&state, &session_id)?;
    let estado = estado_backup(&app)?;
    Ok(estado.info(&state.config))
}

/// Crea un backup manual ahora (asíncrono: corre en `spawn_blocking` porque
/// gbak y el copiado del `.fdb` son bloqueantes). Devuelve el estado
/// refrescado para que la UI no tenga que hacer una segunda llamada.
///
/// Si el backup programado ya está corriendo, devuelve error (claim atómico
/// en `run_backup`): no se solapan.
#[tauri::command]
pub async fn backup_ahora<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    session_id: String,
) -> Cmd<InfoBackup> {
    require_usuario_admin(&state, &session_id)?;
    let cfg = state.config.clone();
    let cfg_hilo = cfg.clone();
    let estado = estado_backup(&app)?;
    let estado_hilo = estado.clone();

    tauri::async_runtime::spawn_blocking(move || {
        crate::services::backup::run_backup(&cfg_hilo, &estado_hilo)
    })
    .await
    .map_err(|e| AppError::Generic(format!("La tarea de backup falló: {e}")).to_payload())?
    .map_err(|e| e.to_payload())?;

    Ok(estado.info(&cfg))
}

/// Restaura la base de datos desde un backup del panel. Flujo (Fase 8 del
/// plan, `PLAN_IMPLEMENTACION_TAURI.md` §4.8):
///
/// 1. Valida que el archivo esté dentro del directorio de backups (anti-
///    traversal) y que exista.
/// 2. Prepara un **staging** en claro: si el backup está cifrado (magic
///    DRENC-01) lo descifra con `password` (requerida); si está plano lo
///    copia. El backup original NUNCA se modifica.
/// 3. Relanza la app con `--restaurar-backup=<staging>` y la cierra. En el
///    arranque, antes de abrir el pool, `restaurar_en_arranque` hace
///    `gbak -r` a un temporal y lo renombra sobre el `.fdb` (swap atómico:
///    la BD actual queda intacta si gbak falla). La app debe terminar para
///    soltar el lock exclusivo del motor Embedded.
///
/// Solo Administrador: la restauración reemplaza TODA la base de datos.
#[tauri::command]
pub async fn backup_restaurar<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<'_, AppState>,
    session_id: String,
    archivo: String,
    password: Option<String>,
) -> Cmd<InfoBackup> {
    require_usuario_admin(&state, &session_id)?;
    let cfg = state.config.clone();
    let estado = estado_backup(&app)?;
    let nombre = archivo.trim().to_string();
    if nombre.is_empty() {
        return Err(AppError::Business("Selecciona un backup para restaurar.".into()).to_payload());
    }

    // La preparación del staging y el relanzamiento son bloqueantes → hilo.
    let cfg_hilo = cfg.clone();
    let estado_hilo = estado.clone();
    let app_hilo = app.clone();
    let resultado = tauri::async_runtime::spawn_blocking(move || {
        // 1) Ruta dentro del directorio de backups (defensa en profundidad:
        //    el comando solo debe operar sobre copias del panel).
        let dir = crate::services::backup::dir_backups(&cfg_hilo);
        let ruta = dir.join(&nombre);
        if !ruta.exists() {
            return Err(AppError::Generic(format!("El backup no existe: {nombre}")).to_payload());
        }
        let dir_canon = dir.canonicalize().unwrap_or(dir);
        let ruta_canon = ruta.canonicalize().unwrap_or(ruta);
        if !ruta_canon.starts_with(&dir_canon) {
            return Err(AppError::Generic(
                "El backup está fuera del directorio de backups.".into(),
            )
            .to_payload());
        }

        // 2) Staging (descifra si aplica). Fallos aquí (contraseña incorrecta,
        //    archivo dañado) quedan registrados para el panel.
        let staging = match crate::services::backup::preparar_staging(
            &cfg_hilo,
            &ruta_canon,
            password.as_deref(),
        ) {
            Ok(s) => s,
            Err(e) => {
                estado_hilo.registrar_resultado_restauracion(&nombre, Err(e.to_string()));
                return Err(e.to_payload());
            }
        };

        // 3) Relanzar con el flag de restauración. Si el relanzamiento falla,
        //    se limpia el staging y se registra el error (la app sigue viva).
        if let Err(e) = crate::services::backup::lanzar_reinicio_con_restauracion(&staging) {
            let _ = std::fs::remove_file(&staging);
            estado_hilo.registrar_resultado_restauracion(&nombre, Err(e.to_string()));
            return Err(e.to_payload());
        }
        let info = estado_hilo.info(&cfg_hilo);
        // 4) Cerrar la app tras dar tiempo a que la respuesta llegue a la UI
        //    (la UI muestra «restaurando» antes de que el proceso termine).
        let handle = app_hilo.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            handle.exit(0);
        });
        Ok(info)
    })
    .await
    .map_err(|e| AppError::Generic(format!("La tarea de restauración falló: {e}")).to_payload())?;

    resultado
}
