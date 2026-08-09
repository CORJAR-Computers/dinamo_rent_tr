//! gastos_integration.rs — Pruebas de integración del servicio de gastos
//! contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Usa un auto real de la BD (solo lectura) y crea/elimina gastos temporales
//! en cada test.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Local;
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::repositories::gasto::GastoDatos;
use dinamo_rent_lib::services::gasto::GastoService;
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

/// Auto real de la BD de dev (lectura) — o None si no hay autos
fn auto_real(state: &AppState) -> Option<String> {
    let mut conn = state.pool.get().expect("conn");
    let autos = AutoRepository::obtener_todos(&mut conn).expect("autos");
    autos.first().map(|a| a.placa.clone())
}

fn datos_gasto(desc: &str) -> GastoDatos {
    let hoy = Local::now().date_naive();
    GastoDatos {
        placa: None,
        fecha: hoy.format("%Y-%m-%d").to_string(),
        categoria: "Combustible".into(),
        descripcion: desc.into(),
        monto: "120000".into(),
        comprobante: Some("F-0001".into()),
    }
}

#[test]
#[serial]
fn gasto_crud_roundtrip() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let mut datos = datos_gasto("Gasto roundtrip");
    if let Some(placa) = auto_real(&state) {
        datos.placa = Some(placa.clone());
    }

    // Crear (actor registrado en la columna usuario)
    let creado = GastoService::crear(&mut conn, cfg, "tester", datos.clone()).expect("crear gasto");
    let id = creado.id;
    assert_eq!(creado.descripcion, "Gasto roundtrip");
    assert_eq!(creado.categoria, "Combustible");
    assert_eq!(creado.monto, "120000.00", "monto normalizado con 2 decimales");
    assert_eq!(creado.usuario.as_deref(), Some("tester"), "actor registrado");
    if let Some(placa) = &datos.placa {
        assert_eq!(creado.placa.as_deref(), Some(placa.as_str()));
    }

    // Obtener
    let obtenido = GastoService::obtener(&mut conn, id).expect("obtener gasto");
    assert_eq!(obtenido.fecha, datos.fecha);

    // Actualizar
    datos.monto = "150000".into();
    datos.descripcion = "Gasto actualizado".into();
    let actualizado = GastoService::actualizar(&mut conn, cfg, id, datos.clone())
        .expect("actualizar gasto");
    assert_eq!(actualizado.monto, "150000.00");
    assert_eq!(actualizado.descripcion, "Gasto actualizado");

    // Eliminar
    GastoService::eliminar(&mut conn, id).expect("eliminar gasto");
    assert!(GastoService::obtener(&mut conn, id).is_err(), "gasto eliminado");
}

#[test]
#[serial]
fn gasto_validaciones() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    // Descripción vacía → validation
    let sin_desc = datos_gasto("   ");
    let err = GastoService::crear(&mut conn, cfg, "tester", sin_desc).expect_err("descripción vacía");
    assert_eq!(err.kind(), "validation");

    // Monto cero → validation
    let mut cero = datos_gasto("Monto cero");
    cero.monto = "0".into();
    let err = GastoService::crear(&mut conn, cfg, "tester", cero).expect_err("monto cero");
    assert_eq!(err.kind(), "validation");

    // Monto inválido → validation
    let mut inv = datos_gasto("Monto inválido");
    inv.monto = "abc".into();
    let err = GastoService::crear(&mut conn, cfg, "tester", inv).expect_err("monto inválido");
    assert_eq!(err.kind(), "validation");

    // Fecha inválida → validation
    let mut fecha = datos_gasto("Fecha inválida");
    fecha.fecha = "no-es-fecha".into();
    let err = GastoService::crear(&mut conn, cfg, "tester", fecha).expect_err("fecha inválida");
    assert_eq!(err.kind(), "validation");

    // Categoría fuera de la lista de config → validation
    let mut cat = datos_gasto("Categoría inventada");
    cat.categoria = "Inventada".into();
    let err = GastoService::crear(&mut conn, cfg, "tester", cat).expect_err("categoría inválida");
    assert_eq!(err.kind(), "validation");

    // XSS en la descripción → validation
    let xss = datos_gasto("<script>alert(1)</script>");
    let err = GastoService::crear(&mut conn, cfg, "tester", xss).expect_err("xss");
    assert_eq!(err.kind(), "validation");
}

#[test]
#[serial]
fn gasto_totales_y_contar() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    // Dos gastos: 120000 + 80000 = 200000
    let mut g1 = datos_gasto("Gasto totales 1");
    g1.monto = "120000".into();
    g1.categoria = "Combustible".into();
    let creado1 = GastoService::crear(&mut conn, cfg, "tester", g1).expect("crear g1");
    let id1 = creado1.id;

    let mut g2 = datos_gasto("Gasto totales 2");
    g2.monto = "80000".into();
    g2.categoria = "Lavado".into();
    let creado2 = GastoService::crear(&mut conn, cfg, "tester", g2).expect("crear g2");
    let id2 = creado2.id;

    // Totales
    let totales = GastoService::totales(&mut conn).expect("totales");
    // El total general incluye los gastos preexistentes de la BD; al menos suma los nuestros
    let monto_total: rust_decimal::Decimal = totales.total_general.parse().expect("total numérico");
    assert!(monto_total >= rust_decimal::Decimal::from(200_000), "total >= 200000");

    let comb = totales
        .por_categoria
        .iter()
        .find(|t| t.clave == "Combustible")
        .map(|t| t.total.parse::<rust_decimal::Decimal>().unwrap_or_default());
    assert!(comb.is_some_and(|m| m >= rust_decimal::Decimal::from(120_000)));

    // Contar
    let total = GastoService::contar(&mut conn).expect("contar");
    assert!(total >= 2);

    // Recientes incluye los nuestros
    let recientes = GastoService::recientes(&mut conn, 50).expect("recientes");
    assert!(recientes.iter().any(|g| g.id == id1 || g.id == id2));

    // Limpieza
    GastoService::eliminar(&mut conn, id1).expect("eliminar g1");
    GastoService::eliminar(&mut conn, id2).expect("eliminar g2");
}

#[test]
#[serial]
fn gasto_por_placa() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        eprintln!("Sin autos en la BD de dev — test de placa omitido");
        return;
    };

    let mut datos = datos_gasto("Gasto placa ABC");
    datos.placa = Some(placa.clone());
    let creado = GastoService::crear(&mut conn, cfg, "tester", datos).expect("crear gasto con placa");
    let id = creado.id;
    assert_eq!(creado.placa.as_deref(), Some(placa.as_str()));

    // Listar por placa devuelve el gasto
    let lista = GastoService::listar(&mut conn, None, Some(&placa), None).expect("listar por placa");
    assert!(
        lista.iter().any(|g| g.id == id),
        "el gasto aparece filtrando por placa"
    );

    // Totales por placa incluyen la placa
    let totales = GastoService::totales(&mut conn).expect("totales");
    assert!(
        totales.por_placa.iter().any(|t| t.clave == placa),
        "la placa aparece en los totales por placa"
    );

    // Limpieza
    GastoService::eliminar(&mut conn, id).expect("eliminar");
}
