//! borrado_rbac_integration.rs — RBAC de los comandos de borrado contra la BD
//! de dev: eliminar_renta / eliminar_auto / eliminar_cliente /
//! eliminar_comparendo / eliminar_gasto / eliminar_reserva /
//! eliminar_mantenimiento.
//!
//! Verifica que `require_eliminacion` (roles_con_eliminar de config.ini —
//! por defecto Administrador y Supervisor) rechace a Operador y acepte a los
//! roles configurados. El resto de los comandos de estos módulos siguen
//! requiriendo solo sesión activa.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serial_test::serial;

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
        sessions: Mutex::new(SessionStore::new(3600)),
        login_tracker: Mutex::new(LoginAttemptTracker::new(5, 1800, 300, 10)),
        config: cfg.clone(),
        pii_key: Mutex::new(cfg.db_encryption_key.clone()),
    }
}

/// RBAC: eliminar registros (los 7 comandos `eliminar_*`) solo para los roles
/// de `roles_con_eliminar` (config.ini — por defecto Admin y Supervisor).
#[test]
#[serial]
fn eliminar_requiere_roles_con_eliminar() {
    let state = dev_state();

    // Operador → denegado con kind "permission"
    {
        let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let token = sessions.create(1, "operador", "Operador", "Op", false);
        drop(sessions);
        let err = dinamo_rent_lib::commands::require_eliminacion(&state, &token)
            .expect_err("Operador no puede eliminar registros");
        assert_eq!(err.kind, "permission");
    }

    // Supervisor y Administrador → permitidos (default de roles_con_eliminar)
    {
        let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let token_sup = sessions.create(2, "supervisor", "Supervisor", "Sup", false);
        let token_admin = sessions.create(3, "admin", "Administrador", "Adm", false);
        drop(sessions);
        assert!(
            dinamo_rent_lib::commands::require_eliminacion(&state, &token_sup).is_ok(),
            "Supervisor tiene rol de borrado"
        );
        assert!(
            dinamo_rent_lib::commands::require_eliminacion(&state, &token_admin).is_ok(),
            "Administrador tiene rol de borrado"
        );
    }

    // Sin sesión → session_expired
    {
        let err = dinamo_rent_lib::commands::require_eliminacion(&state, "no-existe")
            .expect_err("sin sesión no se elimina");
        assert_eq!(err.kind, "session_expired");
    }
}
