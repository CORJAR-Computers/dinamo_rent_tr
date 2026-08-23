//! seed_ci — Herramienta de DESARROLLO / CI
//!
//! Siembra una BD mínima y determinista en `data/dinamo_rent_v3.fdb` para que
//! los tests de integración (`src-tauri/tests/*.rs`) puedan EJECUTARSE
//! completos en CI, no solo compilarse:
//!
//!   config.ini → create_pool (crea el .fdb) → run_migrations → seed_admin
//!   → admin con contraseña conocida `Admin123!` → 2 autos → 2 clientes.
//!
//! Uso: `cargo run --features dev --bin seed_ci`
//!      `cargo run --features dev --bin seed_ci -- <data_dir>` (para validar
//!       contra un directorio temporal sin tocar la BD dev)
//!
//! Idempotente: si la BD ya existe, aplica solo las migraciones pendientes y
//! reutiliza los autos/clientes ya sembrados (upsert por placa / no_doc), sin
//! borrar datos. El password del admin se reaplica siempre a `Admin123!` para
//! que los tests de login (`auth_integration`) sean deterministas.
//!
//! ⚠️ Solo debug (mismo mecanismo que dev_reset_admin/sync_dev): no se
//! compila en release salvo con `--features dev`.

use std::path::PathBuf;
use std::sync::Arc;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::crypto::PiiCipher;
use dinamo_rent_lib::core::db::create_pool;
use dinamo_rent_lib::core::migrations::run_migrations;
use dinamo_rent_lib::core::security;
use dinamo_rent_lib::repositories::auto::{AutoDatos, AutoRepository};
use dinamo_rent_lib::repositories::cliente::{ClienteDatos, ClienteRepository};
use dinamo_rent_lib::repositories::usuario::UsuarioRepository;
use dinamo_rent_lib::services::auto::AutoService;
use dinamo_rent_lib::services::cliente::ClienteService;

/// Logger mínimo a stderr (el bin no arranca Tauri, así que `log::` no tiene
/// receptor por defecto). Muestra el progreso de las migraciones.
/// Clave PII de desarrollo para la BD sembrada en CI (solo tests, sin datos
/// reales). Se persiste en config.ini si viene vacía: los tests de rotación
/// PII exigen una clave configurada (rotacion_integration.rs).
const SEED_CI_PII_KEY: &str = "seed-ci-dev-key";

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
    let data_dir = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => manifest.join("../data"),
    };
    let resource_dir = manifest.join("resources");
    std::fs::create_dir_all(&data_dir)?;
    let fdb = data_dir.join("dinamo_rent_v3.fdb");
    let ya_existia = fdb.exists();
    println!("== Sembrando BD mínima para tests de integración ==");
    println!("  data_dir    : {}", data_dir.display());
    println!("  BD existía  : {ya_existia}");

    // 1) Config (auto-genera config.ini con defaults) — mismo camino que dev/tests
    let mut cfg = AppConfig::load(&data_dir, &resource_dir, &manifest);
    println!("  config.ini  : {}", data_dir.join("config.ini").display());
    if cfg.db_encryption_key.trim().is_empty() {
        println!("  clave PII vacía → persistir clave de desarrollo (solo CI/tests)");
        cfg.persist_db_encryption_key(SEED_CI_PII_KEY)?;
        // Recargar para que el cipher de los clientes use la clave persistida
        cfg = AppConfig::load(&data_dir, &resource_dir, &manifest);
    }
    let cfg = Arc::new(cfg);

    // 2) create_pool: crea el .fdb si no existe (fix release v1.0.0)
    let pool = create_pool(&cfg)?;
    println!("  BD lista    : {}", fdb.display());

    // 3) Migraciones (usa el dir de disco si existe; fallback a embebidas)
    run_migrations(&pool, &manifest.join("migrations"))?;
    println!("✓ Migraciones aplicadas");

    // 4) Admin por defecto + contraseña conocida para los tests de login
    dinamo_rent_lib::seed_admin(&pool)?;
    let password = "Admin123!";
    let hash = security::hash_password(password)?;
    {
        let mut conn = pool.get()?;
        UsuarioRepository::actualizar_password(&mut conn, "admin", &hash)?;
    }
    println!("✓ Usuario 'admin' con contraseña conocida (no se imprime)");

    // 5) Autos mínimos (upsert por placa) — los tests de rentas/comparendos/
    //    gastos/mantenimiento exigen ≥1 vehículo.
    let autos = [
        AutoDatos {
            placa: "ABC123".into(),
            marca: "Toyota".into(),
            modelo: "Corolla".into(),
            version: Some("SE 2021".into()),
            color: Some("Blanco".into()),
            tipo: "Sedán".into(),
            cilindraje: Some("1800".into()),
            transmision: Some("Automática".into()),
            combustible: Some("Gasolina".into()),
            no_motor: Some("M1A2B3C4".into()),
            no_chasis: Some("C9Z8Y7X6".into()),
            propietario: Some("Dinamo Rent SAS".into()),
            estado: "Disponible".into(),
            costo_fijo_mensual: "2500000.00".into(),
            kilometraje: 45000.0,
            ubicacion: Some("Bodega Central".into()),
            tipo_adquisicion: Some("Compra".into()),
            proximo_aceite: Some(5000),
            proximo_frenos: Some(30000),
            vencimiento_soat: Some("2026-06-01".into()),
            vencimiento_tecnico: Some("2026-03-15".into()),
            vencimiento_extintor: Some("2026-12-20".into()),
            vencimiento_bateria: Some("2026-09-01".into()),
            observaciones: Some("Vehículo de prueba sembrado por seed_ci".into()),
            fecha_ingreso: "2024-01-15".into(),
        },
        AutoDatos {
            placa: "XYZ789".into(),
            marca: "Chevrolet".into(),
            modelo: "Spark".into(),
            version: Some("GT 2020".into()),
            color: Some("Rojo".into()),
            tipo: "Hatchback".into(),
            cilindraje: Some("1000".into()),
            transmision: Some("Manual".into()),
            combustible: Some("Gasolina".into()),
            no_motor: Some("X7Y8Z9A".into()),
            no_chasis: Some("B1C2D3E4".into()),
            propietario: Some("Dinamo Rent SAS".into()),
            estado: "Disponible".into(),
            costo_fijo_mensual: "1800000.00".into(),
            kilometraje: 78000.0,
            ubicacion: Some("Bodega Central".into()),
            tipo_adquisicion: Some("Leasing".into()),
            proximo_aceite: Some(4500),
            proximo_frenos: Some(25000),
            vencimiento_soat: Some("2025-12-01".into()),
            vencimiento_tecnico: Some("2026-08-30".into()),
            vencimiento_extintor: Some("2026-06-15".into()),
            vencimiento_bateria: Some("2026-04-20".into()),
            observaciones: None,
            fecha_ingreso: "2024-03-01".into(),
        },
    ];
    let mut n_autos_insertados = 0;
    let mut conn = pool.get()?;
    for datos in autos {
        if AutoRepository::existe(&mut conn, &datos.placa)? {
            println!("  auto {} ya existe (skip)", datos.placa);
            continue;
        }
        AutoService::crear(&mut conn, &cfg, "seed", datos)?;
        n_autos_insertados += 1;
    }
    drop(conn);
    println!("✓ Autos sembrados: {n_autos_insertados} nuevos");
    let mut conn = pool.get()?;
    let total_autos = AutoRepository::contar(&mut conn)?;
    println!("  total autos en BD: {total_autos}");
    drop(conn);

    // 6) Clientes mínimos (upsert por no_doc) — los tests de rentas exigen ≥1.
    let cipher = PiiCipher::new(&cfg.db_encryption_key);
    let clientes = [
        ClienteDatos {
            tipo_doc: Some("CC".into()),
            no_doc: Some("1001234567".into()),
            nombres: "Juan".into(),
            apellidos: Some("Pérez Rodríguez".into()),
            nombre_completo: "Juan Pérez Rodríguez".into(),
            celular: Some("3001112233".into()),
            celular2: Some("3014445566".into()),
            email: Some("juan.perez@example.com".into()),
            ciudad: Some("Bogotá".into()),
            estado_region: Some("Cundinamarca".into()),
            pais: Some("Colombia".into()),
            nacionalidad: Some("Colombiana".into()),
            dir_residencia: Some("Calle 10 # 5-20".into()),
            dir_temporal: Some("Carrera 7 # 72-41".into()),
            hotel: Some("Hotel Inter".into()),
            habitacion: Some("302".into()),
            no_licencia: Some("LC1234567890".into()),
            tipo_licencia: Some("B1".into()),
            vencimiento_licencia: Some("2027-05-10".into()),
            estado: "Activo".into(),
        },
        ClienteDatos {
            tipo_doc: Some("CE".into()),
            no_doc: Some("E987654321".into()),
            nombres: "María".into(),
            apellidos: Some("Gómez Torres".into()),
            nombre_completo: "María Gómez Torres".into(),
            celular: Some("3107778899".into()),
            celular2: None,
            email: Some("maria.gomez@example.com".into()),
            ciudad: Some("Medellín".into()),
            estado_region: None,
            pais: Some("Colombia".into()),
            nacionalidad: Some("Colombiana".into()),
            dir_residencia: Some("Calle 45 # 12-30".into()),
            dir_temporal: None,
            hotel: None,
            habitacion: None,
            no_licencia: Some("LC9876543210".into()),
            tipo_licencia: Some("B2".into()),
            vencimiento_licencia: Some("2026-11-22".into()),
            estado: "Activo".into(),
        },
    ];
    let mut n_clientes_insertados = 0;
    let mut conn = pool.get()?;
    for datos in clientes {
        let doc = datos.no_doc.clone().unwrap_or_default();
        if ClienteRepository::obtener_por_documento(&mut conn, &doc)?.is_some() {
            println!("  cliente doc {doc} ya existe (skip)");
            continue;
        }
        ClienteService::crear(&mut conn, &cfg, &cipher, "seed", datos)?;
        n_clientes_insertados += 1;
    }
    drop(conn);
    println!("✓ Clientes sembrados: {n_clientes_insertados} nuevos");
    let mut conn = pool.get()?;
    let total_clientes = ClienteRepository::contar(&mut conn)?;
    println!("  total clientes en BD: {total_clientes}");

    assert!(total_autos >= 1, "seed_ci debe dejar ≥1 auto para los tests");
    assert!(total_clientes >= 1, "seed_ci debe dejar ≥1 cliente para los tests");
    println!("\n✅ BD mínima lista para `cargo test --tests`");
    Ok(())
}

/// Stub de release (defense-in-depth, igual que dev_reset_admin)
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("seed_ci solo está disponible en builds de debug.");
    eprintln!("Compila con: cargo run --features dev --bin seed_ci");
    std::process::exit(1);
}
