//! mantenimiento_integration.rs — Pruebas de integración del servicio de
//! mantenimiento contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Usa un auto real de la BD (solo lectura) y crea/elimina mantenimientos
//! temporales en cada test. Verifica además la sincronización de
//! `autos.proximo_aceite` y las alertas por kilometraje.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::Local;
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::repositories::mantenimiento::MantenimientoDatos;
use dinamo_rent_lib::services::mantenimiento::MantenimientoService;
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

/// Auto real de la BD de dev (lectura) — o None si no hay autos
fn auto_real(state: &AppState) -> Option<String> {
    let mut conn = state.pool.get().expect("conn");
    let autos = AutoRepository::obtener_todos(&mut conn).expect("autos");
    autos.first().map(|a| a.placa.clone())
}

fn datos_mantenimiento(placa: &str, tipo: &str, costo: &str) -> MantenimientoDatos {
    let hoy = Local::now().date_naive();
    MantenimientoDatos {
        placa: placa.into(),
        tipo: tipo.into(),
        fecha: hoy.format("%Y-%m-%d").to_string(),
        descripcion: Some("Mantenimiento de prueba".into()),
        observaciones: None,
        costo: costo.into(),
        km_proximo_cambio_aceite: None,
    }
}

#[test]
#[serial]
fn mantenimiento_crud_roundtrip() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let mut datos = datos_mantenimiento(&placa, "Frenos", "350000");
    let creado = MantenimientoService::crear(&mut conn, cfg, datos.clone()).expect("crear mantenimiento");
    let id = creado.id;
    assert_eq!(creado.placa, placa);
    assert_eq!(creado.tipo, "FRENOS");
    assert_eq!(creado.costo, "350000.00", "costo normalizado con 2 decimales");
    assert_eq!(creado.total, "350000.00", "total = costo");
    assert!(!creado.vehiculo.is_empty(), "JOIN con autos para la UI");

    // Obtener
    let obtenido = MantenimientoService::obtener(&mut conn, id).expect("obtener");
    assert_eq!(obtenido.fecha, datos.fecha);

    // Actualizar
    datos.tipo = "Llantas".into();
    datos.costo = "400000".into();
    let actualizado =
        MantenimientoService::actualizar(&mut conn, cfg, id, datos.clone()).expect("actualizar");
    assert_eq!(actualizado.tipo, "LLANTAS");
    assert_eq!(actualizado.costo, "400000.00");

    // Historial por placa
    let historial =
        MantenimientoService::listar(&mut conn, None, Some(&placa), None).expect("historial");
    assert!(historial.iter().any(|m| m.id == id));

    // Eliminar
    MantenimientoService::eliminar(&mut conn, id, "test").expect("eliminar");
    assert!(MantenimientoService::obtener(&mut conn, id).is_err(), "eliminado");
}

#[test]
#[serial]
fn mantenimiento_validaciones() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    // Placa inexistente → business (no existe el vehículo)
    let sin_auto = datos_mantenimiento("ZZZ999", "Frenos", "100000");
    let err = MantenimientoService::crear(&mut conn, cfg, sin_auto).expect_err("placa inexistente");
    assert_eq!(err.kind(), "business");

    // Tipo inválido → validation
    let mal_tipo = datos_mantenimiento(&placa, "Inventado", "100000");
    let err = MantenimientoService::crear(&mut conn, cfg, mal_tipo).expect_err("tipo inválido");
    assert_eq!(err.kind(), "validation");

    // Costo cero → validation
    let cero = datos_mantenimiento(&placa, "Frenos", "0");
    let err = MantenimientoService::crear(&mut conn, cfg, cero).expect_err("costo cero");
    assert_eq!(err.kind(), "validation");

    // Costo inválido → validation
    let inv = datos_mantenimiento(&placa, "Frenos", "abc");
    let err = MantenimientoService::crear(&mut conn, cfg, inv).expect_err("costo inválido");
    assert_eq!(err.kind(), "validation");

    // Fecha inválida → validation
    let mut fecha = datos_mantenimiento(&placa, "Frenos", "100000");
    fecha.fecha = "no-es-fecha".into();
    let err = MantenimientoService::crear(&mut conn, cfg, fecha).expect_err("fecha inválida");
    assert_eq!(err.kind(), "validation");

    // XSS en la descripción → validation
    let mut xss = datos_mantenimiento(&placa, "Frenos", "100000");
    xss.descripcion = Some("<script>alert(1)</script>".into());
    let err = MantenimientoService::crear(&mut conn, cfg, xss).expect_err("xss");
    assert_eq!(err.kind(), "validation");
}

#[test]
#[serial]
fn mantenimiento_sincroniza_proximo_aceite_y_alertas() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    // El auto real puede tener km real; guardamos el valor previo de proximo_aceite
    let auto = AutoRepository::obtener_por_placa(&mut conn, &placa).expect("auto").expect("existe");
    let previo = auto.proximo_aceite;

    // Registrar un cambio de aceite con km próximo → sincroniza autos.proximo_aceite
    let mut datos = datos_mantenimiento(&placa, "Cambio Aceite", "200000");
    datos.km_proximo_cambio_aceite = Some(50_000);
    let creado = MantenimientoService::crear(&mut conn, cfg, datos).expect("crear cambio de aceite");
    let id = creado.id;

    let auto2 = AutoRepository::obtener_por_placa(&mut conn, &placa).expect("auto2").expect("existe");
    assert_eq!(auto2.proximo_aceite, Some(50_000), "proximo_aceite sincronizado");

    // Alertas por km: con km 50.000 > km_alert_aceite (500), la alerta puede existir
    // o no según el kilometraje real; lo importante es que la función no falle
    // y que un cambio de aceite programado aparezca si está dentro del margen.
    let km_actual = auto2.kilometraje as i64;
    let alertas = MantenimientoService::alertas_km(&mut conn, cfg).expect("alertas km");
    let esperado = 50_000 - km_actual <= cfg.km_alert_aceite;
    assert_eq!(
        alertas.iter().any(|a| a.placa == placa && a.tipo == "Cambio de aceite"),
        esperado,
        "la alerta de aceite aparece solo si el km está dentro del margen"
    );

    // Limpieza: eliminar mantenimiento (recalcula proximo_aceite desde el
    // historial restante) y restaurar el valor previo sin importar si era None.
    MantenimientoService::eliminar(&mut conn, id, "test").expect("eliminar");
    AutoRepository::actualizar_proximo_aceite(&mut conn, &placa, previo).expect("restaurar");
}

#[test]
#[serial]
fn mantenimiento_totales_y_contar() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let c1 = MantenimientoService::crear(
        &mut conn,
        cfg,
        datos_mantenimiento(&placa, "Frenos", "150000"),
    )
    .expect("crear m1");
    let c2 = MantenimientoService::crear(
        &mut conn,
        cfg,
        datos_mantenimiento(&placa, "Llantas", "90000"),
    )
    .expect("crear m2");

    let totales = MantenimientoService::totales(&mut conn).expect("totales");
    let monto_total: rust_decimal::Decimal = totales.total_general.parse().expect("numérico");
    assert!(monto_total >= rust_decimal::Decimal::from(240_000), "total >= 240000");
    assert!(
        totales.por_placa.iter().any(|t| t.clave == placa),
        "la placa aparece en los totales por placa"
    );
    assert!(
        totales.por_tipo.iter().any(|t| t.clave == "FRENOS"),
        "el tipo aparece en los totales por tipo"
    );

    let total = MantenimientoService::contar(&mut conn).expect("contar");
    assert!(total >= 2);

    let recientes = MantenimientoService::recientes(&mut conn, 50).expect("recientes");
    assert!(recientes.iter().any(|m| m.id == c1.id || m.id == c2.id));

    MantenimientoService::eliminar(&mut conn, c1.id, "test").expect("eliminar m1");
    MantenimientoService::eliminar(&mut conn, c2.id, "test").expect("eliminar m2");
}
