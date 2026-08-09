//! usuarios_integration.rs — Pruebas de integración del servicio de usuarios
//! contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Los tests crean usuarios temporales con usernames únicos y los eliminan al
//! final. Ningún test crea administradores para que la protección del último
//! admin sea determinista (la BD de dev solo tiene al admin del seed).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::{self, LoginAttemptTracker};
use dinamo_rent_lib::repositories::usuario::UsuarioRepository;
use dinamo_rent_lib::services::auth::AuthService;
use dinamo_rent_lib::services::usuario::{
    UsuarioDatos, UsuarioDatosActualizar, UsuarioService,
};
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

/// Sufijo único por ejecución (evita colisiones entre tests paralelos)
fn uniq() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos().to_string())
        .unwrap_or_else(|_| "x".into())
}

fn datos_usuario(username: &str, rol: &str) -> UsuarioDatos {
    UsuarioDatos {
        username: username.into(),
        password: "TempPass1!".into(),
        nombre: "Usuario Temporal".into(),
        rol: rol.into(),
        email: Some("temp@test.co".into()),
        activo: true,
        debe_cambiar_password: true,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CRUD
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn usuario_crud_roundtrip() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");
    let suf = uniq();
    let username = format!("u{}", &suf[..suf.len().min(10)]);

    // Crear
    let creado = UsuarioService::crear(&mut conn, cfg, "admin", datos_usuario(&username, "Operador"))
        .expect("crear usuario");
    let id = creado.id;
    assert_eq!(creado.username, username);
    assert_eq!(creado.rol.as_deref(), Some("Operador"));
    assert!(creado.activo);
    assert!(creado.debe_cambiar_password, "cambio obligatorio al crear");
    assert_eq!(creado.email.as_deref(), Some("temp@test.co"));

    // Obtener
    let obtenido = UsuarioService::obtener(&mut conn, id).expect("obtener usuario");
    assert_eq!(obtenido.username, username);

    // Búsqueda
    let encontrados =
        UsuarioService::listar(&mut conn, Some(&username[..username.len() - 2]))
            .expect("buscar usuario");
    assert!(encontrados.iter().any(|u| u.id == id));

    // Duplicado (insensible a mayúsculas)
    let mut dup = datos_usuario(&username.to_uppercase(), "Operador");
    dup.username = username.to_uppercase();
    let err = UsuarioService::crear(&mut conn, cfg, "admin", dup).expect_err("username duplicado");
    assert_eq!(err.kind(), "duplicate");

    // Actualizar
    let upd = UsuarioDatosActualizar {
        nombre: "Nombre Cambiado".into(),
        rol: "Supervisor".into(),
        email: Some("nuevo@test.co".into()),
        activo: false,
    };
    let actualizado = UsuarioService::actualizar(&mut conn, cfg, "admin", id, upd)
        .expect("actualizar usuario");
    assert_eq!(actualizado.rol.as_deref(), Some("Supervisor"));
    assert!(!actualizado.activo);
    assert_eq!(actualizado.email.as_deref(), Some("nuevo@test.co"));

    // Forzar cambio de contraseña → flag activo + hash verifica la nueva
    let cambio = UsuarioService::forzar_cambio_password(&mut conn, "admin", id, "NuevaPass2!")
        .expect("forzar cambio");
    assert!(cambio.cambio_forzado);
    assert!(cambio.usuario.debe_cambiar_password);
    let auth = UsuarioRepository::obtener_para_autenticacion(&mut conn, &username)
        .expect("obtener auth")
        .expect("existe");
    assert_eq!(
        security::verify_password(&auth.password, "NuevaPass2!"),
        security::VerifyResult::Valid
    );

    // Eliminar
    UsuarioService::eliminar(&mut conn, "admin", id).expect("eliminar usuario");
    assert!(UsuarioService::obtener(&mut conn, id).is_err(), "usuario eliminado");
}

#[test]
#[serial]
fn usuario_validaciones() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");
    let suf = uniq();
    let username = format!("v{}", &suf[..suf.len().min(10)]);

    // Contraseña débil
    let mut debil = datos_usuario(&username, "Operador");
    debil.password = "corta1!".into();
    let err = UsuarioService::crear(&mut conn, cfg, "admin", debil)
        .expect_err("password débil rechazada");
    assert_eq!(err.kind(), "validation");

    // Rol inválido
    let mut mal_rol = datos_usuario(&username, "Cajero");
    mal_rol.password = "TempPass1!".into();
    let err = UsuarioService::crear(&mut conn, cfg, "admin", mal_rol)
        .expect_err("rol inválido rechazado");
    assert_eq!(err.kind(), "validation");

    // Email inválido
    let mut mal_email = datos_usuario(&username, "Operador");
    mal_email.email = Some("correo-sin-arroba".into());
    let err = UsuarioService::crear(&mut conn, cfg, "admin", mal_email)
        .expect_err("email inválido rechazado");
    assert_eq!(err.kind(), "validation");

    // Username con espacios
    let mut espacios = datos_usuario(&username, "Operador");
    espacios.username = format!("{username} con espacio");
    let err = UsuarioService::crear(&mut conn, cfg, "admin", espacios)
        .expect_err("username con espacio rechazado");
    assert_eq!(err.kind(), "validation");
}

// ─────────────────────────────────────────────────────────────────────────────
// Protecciones
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn usuario_no_elimina_la_propia_cuenta() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");
    let suf = uniq();
    let username = format!("p{}", &suf[..suf.len().min(10)]);

    let creado =
        UsuarioService::crear(&mut conn, cfg, "admin", datos_usuario(&username, "Operador"))
            .expect("crear usuario");
    let id = creado.id;

    // Mismo username que el actor → rechazado
    let err = UsuarioService::eliminar(&mut conn, &username, id).expect_err("auto-eliminación");
    assert_eq!(err.kind(), "business");

    // Otro actor (admin) sí puede
    UsuarioService::eliminar(&mut conn, "admin", id).expect("eliminar con otro actor");
    assert!(UsuarioService::obtener(&mut conn, id).is_err());
}

#[test]
#[serial]
fn usuario_protege_ultimo_admin() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");	// Precondición: la BD de dev solo tiene el admin del seed
	let admins = UsuarioRepository::contar_admins(&mut conn).expect("contar admins");
	assert!(admins >= 1);
	let admin = UsuarioRepository::obtener_por_username(&mut conn, "admin")
		.expect("query")
		.expect("el admin del seed existe");
	if admins > 1 || admin.rol.as_deref() != Some("Administrador") {
		eprintln!("SKIP: hay {admins} admins activos; la protección de último admin no aplica.");
		return;
	}

	// Eliminar al admin → Business
	let err = UsuarioService::eliminar(&mut conn, "operador", admin.id)
		.expect_err("no se elimina al admin");
	assert_eq!(err.kind(), "business");

	// Despromover al admin → Business
	let upd = UsuarioDatosActualizar {
		nombre: "Admin".into(),
		rol: "Operador".into(),
		email: None,
		activo: true,
	};
	let err = UsuarioService::actualizar(&mut conn, cfg, "admin", admin.id, upd)
		.expect_err("no se despromueve al admin");
	assert_eq!(err.kind(), "business");

	// Desactivar al admin → Business
	let upd = UsuarioDatosActualizar {
		nombre: "Admin".into(),
		rol: "Administrador".into(),
		email: None,
		activo: false,
	};
	let err = UsuarioService::actualizar(&mut conn, cfg, "admin", admin.id, upd)
		.expect_err("no se desactiva al admin");
	assert_eq!(err.kind(), "business");

	// El admin sigue intacto
	let admin2 = UsuarioService::obtener(&mut conn, admin.id).expect("admin sigue existiendo");
	assert_eq!(admin2.username, "admin");
	assert!(admin2.activo);
}

// ─────────────────────────────────────────────────────────────────────────────
// Preferencia de tema (columna tema, migración 0005)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn usuario_tema_roundtrip() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");
    let suf = uniq();
    let username = format!("t{}", &suf[..suf.len().min(10)]);

    let creado =
        UsuarioService::crear(&mut conn, cfg, "admin", datos_usuario(&username, "Operador"))
            .expect("crear usuario");
    let id = creado.id;

    // Sin configurar → NULL
    assert_eq!(
        UsuarioRepository::obtener_tema(&mut conn, id).expect("tema inicial"),
        None,
        "un usuario nuevo no tiene tema configurado"
    );

    // Guardar y leer cada valor válido
    for tema in ["light", "dark", "auto"] {
        UsuarioRepository::guardar_tema(&mut conn, id, tema).expect("guardar tema");
        assert_eq!(
            UsuarioRepository::obtener_tema(&mut conn, id).expect("leer tema"),
            Some(tema.to_string()),
            "roundtrip de {tema}"
        );
    }

    // Cleanup
    UsuarioService::eliminar(&mut conn, "admin", id).expect("eliminar");
}

// ─────────────────────────────────────────────────────────────────────────────
// Desbloqueo
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn usuario_desbloqueo_resetea_intentos() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");
    let suf = uniq();
    let username = format!("d{}", &suf[..suf.len().min(10)]);

    let creado =
        UsuarioService::crear(&mut conn, cfg, "admin", datos_usuario(&username, "Operador"))
            .expect("crear usuario");
    let id = creado.id;	// Simular una cuenta bloqueada: intentos fallidos en BD + bloqueo en el tracker
	UsuarioRepository::persistir_intentos(&mut conn, &username, 5).expect("persistir intentos");
	{
		let mut tracker = state.login_tracker.lock().unwrap();
		for _ in 0..5 {
			tracker.record_failed_attempt(&username, None);
		}
		tracker.lock_account(&username);
		assert!(tracker.is_locked(&username), "cuenta bloqueada en el tracker");
	}

	// Desbloqueo vía AuthService (mismo camino que el comando)
	let fue_bloqueada = AuthService::unlock_account(&state, &username).expect("desbloquear");
	assert!(fue_bloqueada, "el tracker la tenía bloqueada");

	// Los intentos quedan en 0 en BD y el tracker ya no la bloquea
	let u = UsuarioService::obtener(&mut conn, id).expect("obtener usuario");
	assert_eq!(u.intentos_fallidos, 0);
	assert!(!state.login_tracker.lock().unwrap().is_locked(&username));

    // Cleanup
    UsuarioService::eliminar(&mut conn, "admin", id).expect("eliminar");
}
