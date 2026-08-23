//! tema_comandos_integration.rs — Pruebas de integración de los comandos Tauri
//! `obtener_tema` / `guardar_tema` (commands/auth.rs) contra el .fdb de desarrollo.
//!
//! Los comandos reciben `tauri::State<AppState>`, así que se construye una app
//! mock con `tauri::test::mock_builder()` (feature `test` de tauri, ver
//! Cargo.toml dev-dependencies) para invocarlos con el mismo contrato que el
//! runtime real: validación de sesión + validación de valores + persistencia.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serial_test::serial;
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};

use dinamo_rent_lib::commands::auth::{guardar_tema, obtener_tema};
use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::services::usuario::{UsuarioDatos, UsuarioService};
use dinamo_rent_lib::services::AppState;

/// Guard RAII minimalista: ejecuta la clausura al salir del scope, incluso si
/// un `assert!` falla (panic-safe). Garantiza que el usuario temporal siempre
/// se elimina de la BD de dev aunque el test falle a mitad de camino.
struct AlSalir<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for AlSalir<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

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

/// Sufijo único por ejecución (evita colisiones entre tests paralelos)
fn uniq() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos().to_string())
        .unwrap_or_else(|_| "x".into())
}

fn datos_usuario(username: &str) -> UsuarioDatos {
    UsuarioDatos {
        username: username.into(),
        password: "TempPass1!".into(),
        nombre: "Usuario Tema".into(),
        rol: "Operador".into(),
        email: None,
        activo: true,
        debe_cambiar_password: true,
    }
}

/// Crea la sesión de un usuario real y devuelve su token
fn crear_sesion(state: &AppState, user_id: i64, username: &str) -> String {
    let mut sessions = state.sessions.lock().unwrap();
    sessions.create(user_id, username, "Operador", "Usuario Tema", false)
}

#[test]
#[serial]
fn tema_comandos_roundtrip_y_validacion() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    // Usuario temporal para tener un user_id real en la sesión
    let suf = uniq();
    let username = format!("w{}", &suf[..suf.len().min(10)]);
    let creado = UsuarioService::crear(&mut conn, cfg, "admin", datos_usuario(&username))
        .expect("crear usuario");
    let id = creado.id;
    // Cleanup garantizado aunque falle un assert (no deja residuos en la BD)
    let _limpieza = AlSalir(Some(move || {
        let _ = UsuarioService::eliminar(&mut conn, "admin", id);
    }));
    let sid = crear_sesion(&state, id, &username);

    let app = app_mock(state);
    let st = app.state::<AppState>();	// ── 1) Sin configurar → None ──
	let tema = obtener_tema(st.clone(), sid.clone()).expect("obtener_tema inicial");
	assert_eq!(tema, None, "un usuario nuevo no tiene tema");

	// ── 2) Guardar y leer cada valor válido ──
	for v in ["light", "dark", "auto"] {
		guardar_tema(st.clone(), sid.clone(), v.into()).expect("guardar_tema válido");
		let leido = obtener_tema(st.clone(), sid.clone()).expect("obtener_tema tras guardar");
		assert_eq!(leido.as_deref(), Some(v), "roundtrip de {v}");
	}

	// ── 3) Valores inválidos → error validation y NO modifican la BD ──
	let antes = obtener_tema(st.clone(), sid.clone()).expect("tema actual");
	for invalido in ["neon", "DARK", "auto ", " light", "", "auto-light", "oscuro"] {
		let err = guardar_tema(st.clone(), sid.clone(), invalido.into())
			.expect_err("tema inválido debe rechazarse");
		assert_eq!(err.kind, "validation", "kind para {invalido:?}");
		let despues = obtener_tema(st.clone(), sid.clone()).expect("tema tras rechazo");
		assert_eq!(
			despues, antes,
			"un tema inválido no debe alterar el valor persistido ({invalido:?})"
		);
	}

	// ── 4) Sesión inexistente → session_expired en ambos comandos ──
	let err = obtener_tema(st.clone(), "token-inexistente".into())
		.expect_err("obtener_tema sin sesión");
	assert_eq!(err.kind, "session_expired");
	let err = guardar_tema(st.clone(), "token-inexistente".into(), "light".into())
		.expect_err("guardar_tema sin sesión");
	assert_eq!(err.kind, "session_expired");
}

#[test]
#[serial]
fn tema_comandos_aislan_por_usuario() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    // Dos usuarios con sesiones propias
    let suf = uniq();
    let u1 = format!("x{}", &suf[..suf.len().min(9)]);
    let u2 = format!("y{}", &suf[..suf.len().min(9)]);
    let id1 = UsuarioService::crear(&mut conn, cfg, "admin", datos_usuario(&u1))
        .expect("crear u1")
        .id;
    let id2 = UsuarioService::crear(&mut conn, cfg, "admin", datos_usuario(&u2))
        .expect("crear u2")
        .id;
    // Cleanup garantizado de ambos aunque falle un assert
    let _limpieza = AlSalir(Some(move || {
        let _ = UsuarioService::eliminar(&mut conn, "admin", id1);
        let _ = UsuarioService::eliminar(&mut conn, "admin", id2);
    }));
    let sid1 = crear_sesion(&state, id1, &u1);
    let sid2 = crear_sesion(&state, id2, &u2);

    let app = app_mock(state);
    let st = app.state::<AppState>();	// u1 guarda 'dark'; u2 no ve su preferencia
	guardar_tema(st.clone(), sid1.clone(), "dark".into()).expect("u1 guarda dark");
	let t1 = obtener_tema(st.clone(), sid1.clone()).expect("tema u1");
	let t2 = obtener_tema(st.clone(), sid2.clone()).expect("tema u2");
	assert_eq!(t1.as_deref(), Some("dark"));
	assert_eq!(t2, None, "la preferencia de u1 no debe filtrarse a u2");

	// u2 guarda 'auto' → no altera la de u1
	guardar_tema(st.clone(), sid2, "auto".into()).expect("u2 guarda auto");
	let t1 = obtener_tema(st, sid1).expect("tema u1 de nuevo");
	assert_eq!(t1.as_deref(), Some("dark"), "u1 sigue en dark");
}
