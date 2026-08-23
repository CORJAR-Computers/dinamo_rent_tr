//! permisos_comandos_integration.rs — RBAC de los comandos sensibles del
//! inventario de permisos (PERMISOS.md): `get_pii_status` (roles_con_usuarios,
//! solo Administrador) y `simit_sync_now` (roles_con_eliminar, por defecto
//! Administrador y Supervisor) contra la BD de desarrollo.
//!
//! Se invocan los **comandos completos** con el mismo contrato que el runtime
//! real (mock de Tauri con `tauri::test::mock_builder` + `State<AppState>` +
//! `AppHandle`), como `tema_comandos_integration.rs`: un Operador con sesión
//! válida debe recibir `kind: "permission"` en ambos, y un token inexistente
//! `session_expired`. El guard corta ANTES de tocar la BD (get_pii_status) o
//! el portal SIMIT (simit_sync_now), así que el test no requiere red ni el
//! estado del agente gestionado.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serial_test::serial;
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};

use dinamo_rent_lib::commands::pii::get_pii_status;
use dinamo_rent_lib::commands::simit::simit_sync_now;
use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::services::AppState;

fn dev_state() -> AppState {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let cfg = Arc::new(AppConfig::load(&data_dir, &resource_dir, &manifest));
    let pool = dinamo_rent_lib::core::db::create_pool(&cfg).expect("pool embedded");
    AppState {
        pool,
        sessions: std::sync::Arc::new(Mutex::new(SessionStore::new(3600))),
        login_tracker: Mutex::new(LoginAttemptTracker::new(5, 1800, 300, 10)),
        config: cfg.clone(),
        pii_key: Mutex::new(cfg.db_encryption_key.clone()),
    }
}

/// App de Tauri mock manejando el AppState de dev. La `State` se obtiene en el
/// test con `app.state::<AppState>()` (requiere `use tauri::Manager`).
fn app_mock(state: AppState) -> tauri::App<tauri::test::MockRuntime> {
    mock_builder()
        .manage(state)
        .build(mock_context(noop_assets()))
        .expect("mock app")
}

/// Crea una sesión para el rol dado y devuelve su token
fn sesion_rol(state: &AppState, user_id: i64, username: &str, rol: &str) -> String {
    let mut sessions = state.sessions.lock().unwrap();
    sessions.create(user_id, username, rol, "Usuario RBAC", false)
}

/// `get_pii_status` con sesión de Operador → `permission` (estado PII solo
/// Administrador según `roles_con_usuarios`).
#[test]
#[serial]
fn get_pii_status_deniega_operador() {
    let state = dev_state();
    let app = app_mock(state);
    let st = app.state::<AppState>();

    // Operador con sesión válida → permission (antes de tocar la BD)
    let sid = sesion_rol(st.inner(), 1, "operador", "Operador");
    let err = get_pii_status(st.clone(), sid).expect_err("Operador no ve el estado PII");
    assert_eq!(err.kind, "permission");

    // Sin sesión → session_expired
    let err = get_pii_status(st.clone(), "token-inexistente".into())
        .expect_err("sin sesión no se consulta el estado PII");
    assert_eq!(err.kind, "session_expired");
}

/// `simit_sync_now` con sesión de Operador → `permission` (sincronizar solo
/// para `roles_con_eliminar`). El guard corta antes de reclamar el agente o
/// contactar el portal SIMIT.
#[test]
#[serial]
fn simit_sync_now_deniega_operador() {
    let state = dev_state();
    let app = app_mock(state);
    let st = app.state::<AppState>();
    let handle = app.handle().clone();

    // Operador con sesión válida → permission (sin red ni agente gestionado)
    let sid = sesion_rol(st.inner(), 2, "operador", "Operador");
    let err = tauri::async_runtime::block_on(simit_sync_now(
        handle.clone(),
        st.clone(),
        sid,
    ))
    .expect_err("Operador no dispara sincronizaciones SIMIT");
    assert_eq!(err.kind, "permission");

    // Sin sesión → session_expired
    let err = tauri::async_runtime::block_on(simit_sync_now(
        handle.clone(),
        st.clone(),
        "token-inexistente".into(),
    ))
    .expect_err("sin sesión no se sincroniza");
    assert_eq!(err.kind, "session_expired");
}
