//! renta_cierre_actualiza_km_auto_test.rs
//!
//! Verifica que al cerrar una renta con km_final, el kilometraje del auto
//! (tabla `autos.kilometraje`) se actualiza correctamente.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Local};
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::repositories::renta::{RentaCierreDatos, RentaDatos};
use dinamo_rent_lib::services::renta::RentaService;
use dinamo_rent_lib::services::AppState;

fn dev_state() -> AppState {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let cfg = Arc::new(AppConfig::load(&data_dir, &resource_dir, &manifest));
    let pool = dinamo_rent_lib::core::db::create_pool(&cfg).expect("pool embedded");
    AppState {
        pool,
        sessions: Arc::new(Mutex::new(SessionStore::new(3600))),
        login_tracker: Mutex::new(LoginAttemptTracker::new(5, 1800, 300, 10)),
        config: cfg.clone(),
        pii_key: Mutex::new(cfg.db_encryption_key.clone()),
    }
}

fn auto_real(state: &AppState) -> Option<String> {
    let mut conn = state.pool.get().expect("conn");
    let autos = AutoRepository::obtener_todos(&mut conn).expect("autos");
    autos.first().map(|a| a.placa.clone())
}

/// Cierra una renta con km_final y verifica que `autos.kilometraje` se actualiza.
#[test]
#[serial]
fn cierre_renta_actualiza_kilometraje_auto() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!("BD de dev sin autos — se requiere flota real.");
    };

    // ── 1. Leer kilometraje actual del auto ──
    let auto_antes =
        AutoRepository::obtener_por_placa(&mut conn, &placa).expect("obtener auto").unwrap();
    let km_antes = auto_antes.kilometraje;

    // ── 2. Crear una renta con km_salida fijo ──
    let hoy = Local::now().date_naive();
    let datos = RentaDatos {
        placa: Some(placa.clone()),
        id_cliente: None,
        nombre_cliente: "Test KM Auto".into(),
        no_licencia: None,
        nacionalidad: None,
        fecha_recogida: hoy.format("%Y-%m-%d").to_string(),
        hora_recogida: Some("09:00".into()),
        ubicacion_recogida: None,
        fecha_retorno: (hoy + Duration::days(1)).format("%Y-%m-%d").to_string(),
        hora_retorno: Some("18:00".into()),
        ubicacion_retorno: None,
        dias_calculados: 1,
        horas_extras: 0,
        valor_dia: "100000".into(),
        valor_hora_extra: "10000".into(),
        valor_dia_extra: "0".into(),
        costo_lavado: "0".into(),
        costo_silla: "0".into(),
        costo_retorno: "0".into(),
        costo_domicilio: "0".into(),
        costo_cables: "0".into(),
        costo_inversor: "0".into(),
        valor_gasolina: "0".into(),
        descuento: "0".into(),
        subtotal: String::new(),
        impuestos: String::new(),
        cobra_iva: false,
        tiene_comision: false,
        cobrar_horas_extra: true,
        comision: "0".into(),
        valor_neto: String::new(),
        total: String::new(),
        abono: "0".into(),
        saldo_pendiente: String::new(),
        observaciones: None,
        km_salida: "42000".into(),
        tanque_salida: Some("Lleno".into()),
        id_reserva: None,
    };
    let creada = RentaService::crear(&mut conn, cfg, datos).expect("crear renta");
    let id = creada.id;
    assert_eq!(creada.estado, "Activo");

    // ── 3. Cerrar con km_final = 45000 ──
    let km_final_esperado: f64 = 45000.0;
    let cierre = RentaCierreDatos {
        fecha_devolucion_real: Some(hoy.format("%Y-%m-%d").to_string()),
        hora_devolucion_real: Some("18:00".into()),
        km_final: Some(km_final_esperado.to_string()),
        tanque_final: Some("Lleno".into()),
        dias_calculados: Some(1),
        horas_extras: Some(0),
        valor_dia: Some("100000".into()),
        valor_hora_extra: Some("10000".into()),
        descuento: Some("0".into()),
        observaciones: Some("Cierre con actualización de km".into()),
    };
    let cerrada = RentaService::cerrar(&mut conn, cfg, id, "test", cierre).expect("cerrar");
    assert_eq!(cerrada.estado, "Cerrada");

    // ── 4. Verificar que autos.kilometraje se actualizó ──
    let auto_despues =
        AutoRepository::obtener_por_placa(&mut conn, &placa).expect("obtener auto tras cierre").unwrap();
    assert_eq!(
        auto_despues.kilometraje, km_final_esperado,
        "autos.kilometraje debe actualizarse al km_final del cierre"
    );
    assert_ne!(
        auto_despues.kilometraje, km_antes,
        "el kilometraje del auto debe cambiar respecto al valor anterior"
    );

    // ── 5. El auto debe estar Disponible ──
    assert_eq!(auto_despues.estado, "Disponible");

    // ── Limpieza ──
    RentaService::eliminar(&mut conn, id, "test").expect("eliminar");
}

/// Cerrar una renta SIN km_final NO debe cambiar el kilometraje del auto.
#[test]
#[serial]
fn cierre_renta_sin_km_no_modifica_auto() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!("BD de dev sin autos — se requiere flota real.");
    };

    // ── 1. Leer kilometraje actual ──
    let auto_antes =
        AutoRepository::obtener_por_placa(&mut conn, &placa).expect("obtener auto").unwrap();
    let km_antes = auto_antes.kilometraje;

    // ── 2. Crear renta ──
    let hoy = Local::now().date_naive();
    let datos = RentaDatos {
        placa: Some(placa.clone()),
        id_cliente: None,
        nombre_cliente: "Test KM sin final".into(),
        no_licencia: None,
        nacionalidad: None,
        fecha_recogida: hoy.format("%Y-%m-%d").to_string(),
        hora_recogida: Some("09:00".into()),
        ubicacion_recogida: None,
        fecha_retorno: (hoy + Duration::days(1)).format("%Y-%m-%d").to_string(),
        hora_retorno: Some("18:00".into()),
        ubicacion_retorno: None,
        dias_calculados: 1,
        horas_extras: 0,
        valor_dia: "100000".into(),
        valor_hora_extra: "10000".into(),
        valor_dia_extra: "0".into(),
        costo_lavado: "0".into(),
        costo_silla: "0".into(),
        costo_retorno: "0".into(),
        costo_domicilio: "0".into(),
        costo_cables: "0".into(),
        costo_inversor: "0".into(),
        valor_gasolina: "0".into(),
        descuento: "0".into(),
        subtotal: String::new(),
        impuestos: String::new(),
        cobra_iva: false,
        tiene_comision: false,
        cobrar_horas_extra: true,
        comision: "0".into(),
        valor_neto: String::new(),
        total: String::new(),
        abono: "0".into(),
        saldo_pendiente: String::new(),
        observaciones: None,
        km_salida: "42000".into(),
        tanque_salida: Some("Lleno".into()),
        id_reserva: None,
    };
    let creada = RentaService::crear(&mut conn, cfg, datos).expect("crear renta");
    let id = creada.id;

    // ── 3. Cerrar SIN km_final ──
    let cierre = RentaCierreDatos {
        fecha_devolucion_real: Some(hoy.format("%Y-%m-%d").to_string()),
        hora_devolucion_real: Some("18:00".into()),
        km_final: None, // Sin km final
        tanque_final: Some("Lleno".into()),
        dias_calculados: Some(1),
        horas_extras: Some(0),
        valor_dia: Some("100000".into()),
        valor_hora_extra: Some("10000".into()),
        descuento: Some("0".into()),
        observaciones: None,
    };
    RentaService::cerrar(&mut conn, cfg, id, "test", cierre).expect("cerrar");

    // ── 4. Verificar que autos.kilometraje NO cambió ──
    let auto_despues =
        AutoRepository::obtener_por_placa(&mut conn, &placa).expect("obtener auto tras cierre").unwrap();
    assert_eq!(
        auto_despues.kilometraje, km_antes,
        "sin km_final, autos.kilometraje no debe modificar"
    );
    assert_eq!(auto_despues.estado, "Disponible");

    // ── Limpieza ──
    RentaService::eliminar(&mut conn, id, "test").expect("eliminar");
}
