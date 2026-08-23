//! autos_clientes_integration.rs — Pruebas de integración de los servicios de
//! vehículos y clientes contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Cada test crea registros temporales con claves únicas y los elimina al final.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::crypto::PiiCipher;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoDatos;
use dinamo_rent_lib::repositories::cliente::ClienteDatos;
use dinamo_rent_lib::services::auto::AutoService;
use dinamo_rent_lib::services::cliente::ClienteService;
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

// ─────────────────────────────────────────────────────────────────────────────
// AUTOS
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn auto_crud_roundtrip() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");
    let placa = format!("T{}", uniq());
    let placa = &placa[..placa.len().min(8)];

    let mut datos = AutoDatos {
        placa: placa.to_lowercase(), // debe normalizarse a mayúsculas
        marca: "TESTMARK".into(),
        modelo: "2024".into(),
        tipo: "Automóvil".into(),
        estado: "Disponible".into(),
        costo_fijo_mensual: "1500000.50".into(),
        kilometraje: 12000.5,
        vencimiento_soat: Some("2027-12-31".into()),
        observaciones: Some("Auto de prueba".into()),
        ..Default::default()
    };

    // Crear
    let creado = AutoService::crear(&mut conn, cfg, datos.clone()).expect("crear auto");
    assert_eq!(creado.placa, placa.to_uppercase(), "placa normalizada a mayúsculas");
    assert_eq!(creado.marca, "TESTMARK");
    assert_eq!(creado.costo_fijo_mensual, "1500000.50", "decimal sin pérdida");
    assert_eq!(creado.estado, "Disponible");

    // Obtener
    let obtenido = AutoService::obtener(&mut conn, placa).expect("obtener auto");
    assert_eq!(obtenido.vencimiento_soat.as_deref(), Some("2027-12-31"));

    // Duplicado → error
    let dup = AutoService::crear(&mut conn, cfg, datos.clone()).expect_err("placa duplicada");
    assert_eq!(dup.kind(), "duplicate");

    // Actualizar
    datos.marca = "TESTMARK2".into();
    datos.estado = "Mantenimiento".into();
    let actualizado =
        AutoService::actualizar(&mut conn, cfg, placa, datos).expect("actualizar auto");
    assert_eq!(actualizado.marca, "TESTMARK2");
    assert_eq!(actualizado.estado, "Mantenimiento");

    // Alertas: incluye el auto si tiene vencimientos próximos (SOAT lejano → no)
    let alertas = AutoService::alertas_vencimiento(&mut conn, cfg).expect("alertas");
    assert!(alertas.iter().all(|a| a.placa != placa.to_uppercase()));

    // Eliminar
    AutoService::eliminar(&mut conn, "test", placa).expect("eliminar auto");
    assert!(AutoService::obtener(&mut conn, placa).is_err(), "auto eliminado");
}

#[test]
#[serial]
fn auto_listar_y_contar() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");
    let total = AutoService::contar(&mut conn).expect("contar");
    assert!(total > 0, "la BD de dev tiene autos");

    let todos = AutoService::listar(&mut conn, None, None).expect("listar todos");
    assert_eq!(todos.len() as i64, total);

    let por_estado = AutoService::contar_por_estado(&mut conn).expect("por estado");
    let suma: i64 = por_estado.iter().map(|(_, c)| c).sum();
    assert_eq!(suma, total);
}

// ─────────────────────────────────────────────────────────────────────────────
// CLIENTES
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn cliente_crud_con_pii() {
    let state = dev_state();
    let cfg = &state.config;
    let cipher = PiiCipher::new(&cfg.db_encryption_key);
    let mut conn = state.pool.get().expect("conn");
    let suf = uniq();
    let no_doc = format!("DOC{}", &suf[..suf.len().min(8)]);

    let mut datos = ClienteDatos {
        tipo_doc: Some("Cédula".into()),
        no_doc: Some(no_doc.clone()),
        nombres: "Cliente".into(),
        apellidos: Some("Prueba".into()),
        celular: Some("3101234567".into()),
        email: Some("prueba@test.co".into()),
        ciudad: Some("Barranquilla".into()),
        estado: "Activo".into(),
        ..Default::default()
    };

    // Crear (PII se cifra en BD)
    let creado = ClienteService::crear(&mut conn, cfg, &cipher, datos.clone()).expect("crear cliente");
    let id = creado.cliente.id;
    assert_eq!(creado.cliente.nombre_completo, "CLIENTE PRUEBA", "nombres en mayúsculas");
    assert_eq!(creado.cliente.celular.as_deref(), Some("3101234567"), "PII descifrada");
    assert_eq!(creado.cliente.email.as_deref(), Some("prueba@test.co"));

    // Obtener → descifrado
    let obtenido = ClienteService::obtener(&mut conn, &cipher, id).expect("obtener cliente");
    assert_eq!(obtenido.cliente.celular.as_deref(), Some("3101234567"));
    assert!(!obtenido.pii_oculto);

    // Duplicado de documento → error
    let mut dup = datos.clone();
    dup.nombres = "Otro".into();
    let err = ClienteService::crear(&mut conn, cfg, &cipher, dup).expect_err("doc duplicado");
    assert_eq!(err.kind(), "duplicate");

    // Actualizar
    datos.celular = Some("3009998877".into());
    datos.estado = "VIP".into();
    let actualizado =
        ClienteService::actualizar(&mut conn, cfg, &cipher, id, datos).expect("actualizar cliente");
    assert_eq!(actualizado.cliente.celular.as_deref(), Some("3009998877"));
    assert_eq!(actualizado.cliente.estado, "VIP");

    // Buscar por documento
    let encontrados = ClienteService::listar(&mut conn, &cipher, Some(&no_doc), None)
        .expect("buscar por documento");
    assert!(encontrados.iter().any(|c| c.cliente.id == id));

    // Eliminar
    ClienteService::eliminar(&mut conn, "test", id).expect("eliminar cliente");
    assert!(ClienteService::obtener(&mut conn, &cipher, id).is_err(), "cliente eliminado");
}

#[test]
#[serial]
fn cliente_listar_y_contar() {
    let state = dev_state();
    let cfg = &state.config;
    let cipher = PiiCipher::new(&cfg.db_encryption_key);
    let mut conn = state.pool.get().expect("conn");

    let total = ClienteService::contar(&mut conn).expect("contar");
    assert!(total > 0, "la BD de dev tiene clientes");

    let todos = ClienteService::listar(&mut conn, &cipher, None, None).expect("listar");
    assert_eq!(todos.len() as i64, total);

    // Recientes
    let recientes = ClienteService::recientes(&mut conn, &cipher, 5).expect("recientes");
    assert!(recientes.len() <= 5);
    assert!(!recientes.is_empty());
}
