//! commands/app.rs — Comandos de la aplicación (ventana / cierre).

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::Manager;

/// Flag: el frontend ya montó el diálogo de confirmación y puede recibir el
/// evento `app-close-requested`.
///
/// Evita bloquear la X de la ventana si el webview aún no ha registrado su
/// listener (primeros ~1-2 s tras abrir la app, o si el frontend falla): el
/// handler de CloseRequested solo previene el cierre cuando este flag es true;
/// si no, el cierre normal sigue adelante (mejor cerrar que quedarse colgado
/// sin diálogo).
pub struct FrontendListo(pub AtomicBool);

/// El frontend confirma que ya escucha el evento de cierre (se invoca desde
/// ConfirmarCierre.onMount).
#[tauri::command]
pub fn app_frontend_lista(state: tauri::State<'_, FrontendListo>) {
    state.0.store(true, Ordering::SeqCst);
}

/// Versión real de la app (Cargo.toml / tauri.conf.json en el build — la misma
/// que firma el updater y nombra los instaladores). Reemplaza el literal
/// v3.2.0 heredado del proyecto anterior en el menú lateral y el login.
/// Genérica sobre el runtime para poder testearla con `tauri::test::MockRuntime`.
#[tauri::command]
pub fn app_version<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> String {
    app.package_info().version.to_string()
}

/// Cierra la aplicación tras la confirmación del diálogo del frontend.
///
/// El cierre normal de la ventana (botón X) se intercepta en `on_window_event`
/// de lib.rs: se previene y se emite `app-close-requested` al frontend, que
/// muestra el diálogo «¿Está seguro de cerrar la aplicación?». Si el usuario
/// pulsa «Sí», el frontend invoca este comando.
///
/// Se usa `destroy()` (no `close()`) para que el cierre NO vuelva a disparar
/// `CloseRequested` y no entre en el bucle prevent→confirmar→close. Si la
/// ventana main no existe (o falla el destroy), se sale del proceso con
/// `app.exit(0)` como fallback.
#[tauri::command]
pub fn confirmar_cierre(app: tauri::AppHandle) {
    let cerrado = app
        .get_webview_window("main")
        .map(|w| w.destroy().is_ok())
        .unwrap_or(false);
    if !cerrado {
        app.exit(0);
    }
}
