//! auth_integration.rs — Prueba de integración del flujo de login contra el
//! .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Requiere que la BD de desarrollo exista y que 'admin' tenga la contraseña
//! conocida (ej: tras ejecutar `cargo run --bin dev_reset_admin`).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::services::auth::AuthService;
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

#[test]
fn login_ok_admin() {
    let state = dev_state();
    let result = AuthService::login(&state, "admin", "Admin123!", Some("127.0.0.1"));
    assert!(
        result.is_ok(),
        "login debería funcionar: {:?}",
        result.err()
    );
    let login = result.unwrap();
    assert!(login.success);
    assert_eq!(login.username, "admin");
    assert_eq!(login.rol.as_deref(), Some("Administrador"));
    assert!(!login.session_id.is_empty());

    // La sesión debe validarse
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.validate(&login.session_id).expect("sesión válida");
    assert_eq!(session.username, "admin");
    drop(sessions);

    // Logout
    AuthService::logout(&state, &login.session_id);
}

#[test]
fn login_wrong_password_fails() {
    let state = dev_state();
    let result = AuthService::login(&state, "admin", "contraseña-incorrecta", Some("127.0.0.1"));
    assert!(result.is_err());
}

#[test]
fn login_unknown_user_fails() {
    let state = dev_state();
    let result = AuthService::login(&state, "usuario_inexistente", "x", Some("127.0.0.1"));
    assert!(result.is_err());
}

#[test]
fn login_status_reports_attempts() {
    let state = dev_state();
    let _ = AuthService::login(&state, "admin", "mal1", Some("127.0.0.1"));
    let _ = AuthService::login(&state, "admin", "mal2", Some("127.0.0.1"));
    let status = AuthService::get_login_status(&state, "admin");
    assert!(status.failed_attempts >= 2);
    assert!(status.remaining_attempts <= 3);
}

#[test]
fn account_locks_after_5_failures() {
    let state = dev_state();
    for _ in 0..5 {
        let _ = AuthService::login(&state, "admin", "mal", Some("10.0.0.1"));
    }
    // El 6º intento con credenciales correctas debe estar bloqueado
    let result = AuthService::login(&state, "admin", "Admin123!", Some("10.0.0.1"));
    assert!(
        result.is_err(),
        "la cuenta debe estar bloqueada tras 5 intentos"
    );
    // Desbloquear para no dejar la BD de desarrollo bloqueada
    let _ = AuthService::unlock_account(&state, "admin");
}
