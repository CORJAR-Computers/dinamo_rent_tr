//! verificar_instalacion_limpia — Herramienta de DESARROLLO / validación E2E
//!
//! Simula el arranque de la app en un EQUIPO LIMPIO (sin BD y sin el directorio
//! `migrations/` en disco — el bundle solo empaqueta `resources/firebird` y
//! `CARGO_MANIFEST_DIR` apunta a la máquina de build) sin abrir la GUI de
//! Tauri, replicando la secuencia exacta del setup de producción (lib.rs):
//! `AppConfig::load → create_pool → run_migrations → seed_admin` + login real.
//!
//! Uso:
//!   cargo run --features dev --bin verificar_instalacion_limpia
//!   cargo run --features dev --bin verificar_instalacion_limpia -- <data_dir>
//!
//! Con un `<data_dir>` se verifica un directorio ya creado por la app (p.ej.
//! `%APPDATA%\com.corjar.dinamorent` tras un smoke test del release).
//!
//! ⚠️ Solo debug (mismo mecanismo que dev_reset_admin/sync_dev): no se compila
//! en release salvo con `--features dev`.

use std::path::PathBuf;
use std::sync::Mutex;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::db::create_pool;
use dinamo_rent_lib::core::migrations::{run_migrations, MIGRACIONES_EMBEDIDAS};
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::usuario::UsuarioRepository;
use dinamo_rent_lib::services::auth::AuthService;
use dinamo_rent_lib::services::AppState;
use rsfbclient::Queryable;

/// Logger mínimo a stderr (el bin no arranca Tauri, así que `log::` no tiene
/// receptor por defecto). Muestra el progreso de las migraciones.
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        eprintln!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

/// Implementación real (sólo debug).
#[cfg(debug_assertions)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = log::set_boxed_logger(Box::new(StderrLogger));
    log::set_max_level(log::LevelFilter::Info);

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resource_dir = manifest.join("resources");

    // 1) Directorio de datos limpio (simula %APPDATA%\com.corjar.dinamorent vacío)
    let data_dir: PathBuf = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => std::env::temp_dir().join(format!(
            "dinamo_instalacion_limpia_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        )),
    };
    std::fs::create_dir_all(&data_dir)?;
    let fdb = data_dir.join("dinamo_rent_v3.fdb");
    let ya_existia = fdb.exists();
    println!("== Simulando equipo limpio ==");
    println!("  data_dir     : {}", data_dir.display());
    println!("  BD existía   : {ya_existia}");
    if ya_existia {
        println!("  (modo verificación de un dir ya creado por la app)");
    }

    // 2) Config como en producción (genera config.ini si falta)
    let cfg = std::sync::Arc::new(AppConfig::load(&data_dir, &resource_dir, &manifest));
    let config_ini = data_dir.join("config.ini");
    if !ya_existia {
        assert!(
            config_ini.exists(),
            "config.ini debe generarse en el primer arranque"
        );
    }
    println!("  config.ini   : {}", config_ini.display());

    // 3) create_pool: debe CREAR la BD si el archivo no existe (el fix del
    //    release v1.0.0 — antes la app se colgaba esperando una BD inexistente).
    let pool = create_pool(&cfg)?;
    if !ya_existia {
        assert!(fdb.exists(), "create_pool debe haber creado el .fdb");
        println!("✓ BD creada por create_pool: {}", fdb.display());
    } else {
        println!("✓ BD existente abierta: {}", fdb.display());
    }

    // 4) Migraciones con un directorio INEXISTENTE (equipo limpio: el bundle no
    //    incluye migrations/ y CARGO_MANIFEST_DIR es la ruta de la máquina de
    //    build) → el runner debe usar las MIGRACIONES_EMBEDIDAS.
    let dir_inexistente = std::env::temp_dir().join("dinamo_migrations_NO_EXISTE_EN_LIMPIO");
    let _ = std::fs::remove_dir_all(&dir_inexistente);
    assert!(!dir_inexistente.exists(), "precondición: dir de migraciones ausente");
    run_migrations(&pool, &dir_inexistente)?;
    println!(
        "✓ Migraciones aplicadas (fallback de las {} embebidas en el binario)",
        MIGRACIONES_EMBEDIDAS.len()
    );

    // 5) Todas las versiones embebidas quedan registradas en schema_migrations
    let mut conn = pool.get()?;
    let aplicadas: Vec<String> = conn
        .query("SELECT version FROM schema_migrations", ())
        .map(|rows: Vec<(String,)>| rows.into_iter().map(|r| r.0).collect())?;
    let aplicadas_set: std::collections::HashSet<String> = aplicadas.iter().cloned().collect();
    for (name, _) in MIGRACIONES_EMBEDIDAS.iter().copied() {
        assert!(
            aplicadas_set.contains(name),
            "falta registrar {name}; aplicadas: {aplicadas:?}"
        );
    }
    println!("✓ schema_migrations: {} versiones registradas", aplicadas.len());

    // 6) seed_admin: crea el admin por defecto en una BD vacía (solo si no hay usuarios)
    if UsuarioRepository::contar(&mut conn)? == 0 {
        dinamo_rent_lib::seed_admin(&pool)?;
    }
    let n_admin = UsuarioRepository::contar(&mut conn)?;
    assert_eq!(n_admin, 1, "debe existir exactamente 1 usuario tras el seed");
    let admin = UsuarioRepository::obtener_para_autenticacion(&mut conn, "admin")?;
    assert!(admin.is_some(), "el usuario 'admin' debe existir");
    println!("✓ Usuario 'admin' sembrado (contraseña por defecto admin123)");

    // 7) Login real con el admin sembrado (valida hash Argon2 + sesión)
    let state = AppState {
        pool: pool.clone(),
        sessions: Mutex::new(SessionStore::new(3600)),
        login_tracker: Mutex::new(LoginAttemptTracker::new(5, 1800, 300, 10)),
        config: cfg.clone(),
        pii_key: Mutex::new(cfg.db_encryption_key.clone()),
    };
    let login = AuthService::login(&state, "admin", "admin123", Some("local"))?;
    assert!(
        !login.session_id.is_empty(),
        "el login debe devolver una sesión"
    );
    println!(
        "✓ Login admin/admin123 OK (sesión {}…)",
        login.session_id.get(..8).unwrap_or(&login.session_id)
    );

    // 8) Segundo arranque (reinicio): idempotente, sin errores ni versiones nuevas
    run_migrations(&pool, &dir_inexistente)?;
    let n2: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM schema_migrations", ())?;
    assert_eq!(
        n2.map(|(c,)| c).unwrap_or(0) as usize,
        aplicadas.len(),
        "el segundo arranque no debe añadir versiones"
    );
    println!(
        "✓ Segundo arranque idempotente ({} versiones, sin cambios)",
        aplicadas.len()
    );

    println!("\n✅ INSTALACIÓN LIMPIA VALIDADA DE PUNTA A PUNTA");
    Ok(())
}

/// Stub de release (defense-in-depth, igual que dev_reset_admin)
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("verificar_instalacion_limpia solo está disponible en builds de debug.");
    eprintln!("Compila con: cargo run --features dev --bin verificar_instalacion_limpia");
    std::process::exit(1);
}
