//! rentas_integration.rs — Pruebas de integración del servicio de rentas
//! contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Usa un auto y un cliente reales de la BD (solo lectura) y crea/elimina
//! rentas temporales en cada test. Verifica el flujo completo: crear con
//! totales, editar, cerrar con recálculo, pagos (abono/saldo), inspecciones,
//! cancelación y validaciones.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{Datelike, Duration, Local};
use rsfbclient::{Execute, Queryable};
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::repositories::cliente::ClienteRepository;
use dinamo_rent_lib::repositories::renta::{
    InspeccionDatos, PagoDatos, RentaCierreDatos, RentaDatos,
};
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

/// Cliente real de la BD de dev (lectura) — o None si no hay clientes
fn cliente_real(state: &AppState) -> Option<i64> {
    let mut conn = state.pool.get().expect("conn");
    let clientes = ClienteRepository::obtener_todos(&mut conn).expect("clientes");
    clientes.first().map(|c| c.id)
}

fn datos_renta(placa: &str, id_cliente: Option<i64>) -> RentaDatos {
    let hoy = Local::now().date_naive();
    RentaDatos {
        placa: Some(placa.into()),
        id_cliente,
        nombre_cliente: "Cliente de Prueba Renta".into(),
        no_licencia: None,
        nacionalidad: None,
        fecha_recogida: hoy.format("%Y-%m-%d").to_string(),
        hora_recogida: Some("09:00".into()),
        ubicacion_recogida: None,
        fecha_retorno: (hoy + Duration::days(3)).format("%Y-%m-%d").to_string(),
        hora_retorno: Some("18:00".into()),
        ubicacion_retorno: None,
        dias_calculados: 3,
        horas_extras: 0,
        valor_dia: "150000".into(),
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
        cobra_iva: true,
        total: String::new(),
        abono: "0".into(),
        saldo_pendiente: String::new(),
        observaciones: None,
        km_salida: "42000".into(),
        tanque_salida: Some("Lleno".into()),
        id_reserva: None,
    }
}

fn datos_pago(monto: &str) -> PagoDatos {
    PagoDatos {
        monto: monto.into(),
        metodo_pago: "Efectivo".into(),
        concepto: "Abono renta".into(),
        observaciones: None,
    }
}

fn datos_inspeccion(tipo: &str, km: &str) -> InspeccionDatos {
    InspeccionDatos {
        tipo: tipo.into(),
        kilometraje: km.into(),
        nivel_gasolina: "Lleno".into(),
        limpieza: Some("Limpio".into()),
        tiene_repuesto: true,
        tiene_gato_cruceta: true,
        tiene_kit_carretera: true,
        tiene_documentos: true,
        danos_carroceria: None,
        observaciones: None,
    }
}

#[test]
#[serial]
fn renta_crud_cierre_pagos_inspecciones() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };
    let id_cliente = cliente_real(&state);

    // ── Crear ──
    let mut datos = datos_renta(&placa, id_cliente);
    let creada = RentaService::crear(&mut conn, cfg, datos.clone()).expect("crear renta");
    let id = creada.id;
    assert_eq!(creada.estado, "Activo");
    assert_eq!(creada.placa.as_deref(), Some(placa.as_str()));
    assert!(!creada.vehiculo.is_empty(), "JOIN con autos para la UI");
    // 3 días × 150.000 = 450.000 + 19% IVA = 535.500
    assert_eq!(creada.total, "535500.00", "total = subtotal + IVA");
    assert_eq!(creada.subtotal, "450000.00");
    assert_eq!(creada.impuestos, "85500.00");
    assert_eq!(creada.saldo_pendiente, "535500.00");

    // ── Obtener ──
    let obtenida = RentaService::obtener(&mut conn, id).expect("obtener");
    assert_eq!(obtenida.id, id);

    // ── Inspección de salida ──
    let ins = RentaService::registrar_inspeccion(&mut conn, id, datos_inspeccion("Salida", "42000"))
        .expect("inspección salida");
    assert_eq!(ins.tipo, "Salida");

    // ── Pago parcial ──
    let pago = RentaService::registrar_pago(&mut conn, id, "tester", datos_pago("200000"))
        .expect("pago");
    assert_eq!(pago.monto, "200000.00");
    let tras_pago = RentaService::obtener(&mut conn, id).expect("tras pago");
    assert_eq!(tras_pago.abono, "200000.00");
    assert_eq!(tras_pago.saldo_pendiente, "335500.00");

    // ── Editar (días cambian → totales se recalculan) ──
    datos.dias_calculados = 4;
    datos.fecha_retorno = (Local::now().date_naive() + Duration::days(4))
        .format("%Y-%m-%d")
        .to_string();
    let editada = RentaService::actualizar(&mut conn, cfg, id, datos).expect("editar");
    // 4 días × 150.000 = 600.000 + 114.000 IVA = 714.000; abono 200.000 → saldo 514.000
    assert_eq!(editada.total, "714000.00");
    assert_eq!(editada.abono, "200000.00", "el abono se conserva al editar");
    assert_eq!(editada.saldo_pendiente, "514000.00");

    // ── Cerrar con devolución real ──
    let hoy = Local::now().date_naive();
    let cierre = RentaCierreDatos {
        fecha_devolucion_real: Some(hoy.format("%Y-%m-%d").to_string()),
        hora_devolucion_real: Some("17:30".into()),
        km_final: Some("43100".into()),
        tanque_final: Some("3/4".into()),
        dias_calculados: Some(4),
        horas_extras: Some(0),
        valor_dia: Some("150000".into()),
        valor_hora_extra: Some("10000".into()),
        descuento: Some("10000".into()),
        observaciones: Some("Devolución en buen estado".into()),
    };
    let cerrada = RentaService::cerrar(&mut conn, cfg, id, cierre).expect("cerrar");
    assert_eq!(cerrada.estado, "Cerrada");
    assert_eq!(cerrada.fecha_devolucion_real.as_deref(), Some(hoy.format("%Y-%m-%d").to_string().as_str()));
    // 4×150.000 − 10.000 descuento = 590.000 + 112.100 IVA = 702.100; saldo = 702.100 − 200.000
    assert_eq!(cerrada.total, "702100.00");
    assert_eq!(cerrada.saldo_pendiente, "502100.00");

    // No se puede cerrar dos veces
    let err = RentaService::cerrar(&mut conn, cfg, id, RentaCierreDatos::default()).expect_err("doble cierre");
    assert_eq!(err.kind(), "business");

    // ── Limpieza: eliminar (pagos e inspecciones caen en cascada) ──
    RentaService::eliminar(&mut conn, id).expect("eliminar");
    assert!(RentaService::obtener(&mut conn, id).is_err(), "eliminada");
}

#[test]
#[serial]
fn renta_montos_en_blanco_ok() {
    // Regresión del error real de producción: SQLCODE -303 "conversion error
    // from string ''" al enlazar campos monetarios vacíos a CAST(? AS DECIMAL).
    // El formulario arranca valorDia/valorHoraExtra en '' y los costos extra
    // suelen quedar en blanco; el backend debe normalizarlos a "0.00".
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let mut datos = datos_renta(&placa, None);
    // Escenario real: tarifas/costos extra y abono en blanco ('' como envía la UI)
    datos.valor_hora_extra = String::new();
    datos.valor_dia_extra = String::new();
    datos.costo_lavado = String::new();
    datos.costo_silla = String::new();
    datos.costo_retorno = String::new();
    datos.costo_domicilio = String::new();
    datos.costo_cables = String::new();
    datos.costo_inversor = String::new();
    datos.descuento = String::new();
    datos.abono = String::new();

    let creada =
        RentaService::crear(&mut conn, cfg, datos.clone()).expect("crear con montos en blanco");
    // 3 días × 150.000 + IVA 19% = 535.500; abono en blanco → 0.00
    assert_eq!(creada.total, "535500.00");
    assert_eq!(creada.abono, "0.00", "abono en blanco → 0.00, no -303");
    assert_eq!(creada.saldo_pendiente, "535500.00");

    // Editar dejando la tarifa en blanco también debe funcionar (antes: -303)
    datos.valor_dia = String::new();
    datos.valor_hora_extra = String::new();
    let editada = RentaService::actualizar(&mut conn, cfg, creada.id, datos)
        .expect("editar con montos en blanco");
    assert_eq!(editada.total, "0.00", "sin tarifa → total 0, no -303");

    // Cierre con ajustes en blanco (vacío = mantener el valor actual)
    let cierre = RentaCierreDatos {
        fecha_devolucion_real: Some(Local::now().date_naive().format("%Y-%m-%d").to_string()),
        hora_devolucion_real: Some("18:00".into()),
        km_final: Some("43000".into()),
        tanque_final: Some("Lleno".into()),
        dias_calculados: Some(3),
        horas_extras: Some(0),
        valor_dia: Some(String::new()),
        valor_hora_extra: Some(String::new()),
        descuento: Some(String::new()),
        observaciones: None,
    };
    let cerrada =
        RentaService::cerrar(&mut conn, cfg, creada.id, cierre).expect("cerrar con ajustes en blanco");
    assert_eq!(cerrada.estado, "Cerrada");
    assert_eq!(cerrada.total, "0.00");

    // ── Limpieza ──
    RentaService::eliminar(&mut conn, creada.id).expect("eliminar");
}

#[test]
#[serial]
fn renta_no_contrato_secuencial_independiente_del_id() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    // Crea dos rentas seguidas: el no_contrato debe ser estrictamente
    // secuencial (+1) dentro del AÑO actual (secuencia por año, no global).
    let r1 = RentaService::crear(&mut conn, cfg, datos_renta(&placa, None)).expect("crear r1");
    let r2 = RentaService::crear(&mut conn, cfg, datos_renta(&placa, None)).expect("crear r2");

    let anio_actual = Local::now().year() as i64;
    assert!(r1.no_contrato > 0, "no_contrato asignado");
    assert_eq!(r1.anio_contrato, anio_actual, "año del contrato = año de creación");
    assert_eq!(r2.anio_contrato, anio_actual, "ambas rentas en el mismo año");
    assert_eq!(r2.no_contrato, r1.no_contrato + 1, "secuencia +1 dentro del año");
    assert_ne!(
        r1.no_contrato, r1.id,
        "no_contrato independiente del id (secuencia propia)"
    );

    // La persistencia también lo conserva (vuelve a leer desde la BD)
    let re_leida = RentaService::obtener(&mut conn, r1.id).expect("releer r1");
    assert_eq!(re_leida.no_contrato, r1.no_contrato);
    assert_eq!(re_leida.anio_contrato, anio_actual);

    RentaService::eliminar(&mut conn, r1.id).expect("limpieza r1");
    RentaService::eliminar(&mut conn, r2.id).expect("limpieza r2");
}

#[test]
#[serial]
fn renta_crear_con_abono_inicial() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    // Abono inicial 200.000 sobre total 535.500 → saldo 335.500
    let mut datos = datos_renta(&placa, None);
    datos.abono = "200000".into();
    let creada = RentaService::crear(&mut conn, cfg, datos).expect("crear con abono");
    assert_eq!(creada.abono, "200000.00");
    assert_eq!(creada.total, "535500.00");
    assert_eq!(creada.saldo_pendiente, "335500.00", "saldo = total − abono inicial");

    RentaService::eliminar(&mut conn, creada.id).expect("limpieza");
}

#[test]
#[serial]
fn renta_validaciones_y_cancelacion() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    // Placa inexistente → business
    let sin_auto = datos_renta("ZZZ999", None);
    let err = RentaService::crear(&mut conn, cfg, sin_auto).expect_err("placa inexistente");
    assert_eq!(err.kind(), "business");

    // Fechas invertidas → validation
    let mut inv = datos_renta(&placa, None);
    let hoy = Local::now().date_naive();
    inv.fecha_recogida = (hoy + Duration::days(5)).format("%Y-%m-%d").to_string();
    inv.fecha_retorno = hoy.format("%Y-%m-%d").to_string();
    let err = RentaService::crear(&mut conn, cfg, inv).expect_err("fechas invertidas");
    assert_eq!(err.kind(), "validation");

    // Hora inválida → validation
    let mut hora = datos_renta(&placa, None);
    hora.hora_recogida = Some("25:99".into());
    let err = RentaService::crear(&mut conn, cfg, hora).expect_err("hora inválida");
    assert_eq!(err.kind(), "validation");

    // XSS en nombre → validation
    let mut xss = datos_renta(&placa, None);
    xss.nombre_cliente = "<script>alert(1)</script>".into();
    let err = RentaService::crear(&mut conn, cfg, xss).expect_err("xss");
    assert_eq!(err.kind(), "validation");

    // Valor del día inválido → validation
    let mut monto = datos_renta(&placa, None);
    monto.valor_dia = "abc".into();
    let err = RentaService::crear(&mut conn, cfg, monto).expect_err("monto inválido");
    assert_eq!(err.kind(), "validation");

    // ── Cancelar ──
    let creada = RentaService::crear(&mut conn, cfg, datos_renta(&placa, None)).expect("crear");
    let cancelada = RentaService::cancelar(&mut conn, creada.id).expect("cancelar");
    assert!(cancelada.cancelada);
    assert_eq!(cancelada.renta.estado, "Cancelada");
    // Doble cancelación → ok con cancelada=false
    let otra = RentaService::cancelar(&mut conn, creada.id).expect("doble cancelar");
    assert!(!otra.cancelada);

    // No se puede cerrar una cancelada
    let err = RentaService::cerrar(&mut conn, cfg, creada.id, RentaCierreDatos::default())
        .expect_err("cerrar cancelada");
    assert_eq!(err.kind(), "business");

    // Pagos en rentas no activas → business
    let err = RentaService::registrar_pago(&mut conn, creada.id, "tester", datos_pago("10000"))
        .expect_err("pago en cancelada");
    assert_eq!(err.kind(), "business");

    RentaService::eliminar(&mut conn, creada.id).expect("limpieza");
}

#[test]
#[serial]
fn renta_pago_supera_saldo() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let creada = RentaService::crear(&mut conn, cfg, datos_renta(&placa, None)).expect("crear");
    // El total es 535.500; un pago mayor debe rechazarse
    let err = RentaService::registrar_pago(&mut conn, creada.id, "tester", datos_pago("999999999"))
        .expect_err("pago excede saldo");
    assert_eq!(err.kind(), "business");

    // Pago inválido (cero) → validation
    let err = RentaService::registrar_pago(&mut conn, creada.id, "tester", datos_pago("0"))
        .expect_err("pago cero");
    assert_eq!(err.kind(), "validation");

    // Monto mal formado → validation
    let mut mal = datos_pago("abc");
    mal.concepto = "Abono".into();
    let err = RentaService::registrar_pago(&mut conn, creada.id, "tester", mal)
        .expect_err("pago inválido");
    assert_eq!(err.kind(), "validation");

    RentaService::eliminar(&mut conn, creada.id).expect("limpieza");
}

#[test]
#[serial]
fn renta_cierre_con_fecha_devolucion_invalida() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let creada = RentaService::crear(&mut conn, cfg, datos_renta(&placa, None)).expect("crear");

    // Fecha de devolución mal formada → validation en el cierre
    let cierre_mal = RentaCierreDatos {
        fecha_devolucion_real: Some("no-es-fecha".into()),
        ..Default::default()
    };
    let err = RentaService::cerrar(&mut conn, cfg, creada.id, cierre_mal).expect_err("fecha inválida");
    assert_eq!(err.kind(), "validation");

    // Hora de devolución inválida → validation
    let cierre_hora = RentaCierreDatos {
        hora_devolucion_real: Some("25:99".into()),
        ..Default::default()
    };
    let err = RentaService::cerrar(&mut conn, cfg, creada.id, cierre_hora).expect_err("hora inválida");
    assert_eq!(err.kind(), "validation");

    RentaService::eliminar(&mut conn, creada.id).expect("limpieza");
}

/// Dos autos reales de la BD de dev en estado Disponible (lectura) — o None
/// si no hay al menos dos. El test restaura sus estados al final.
fn dos_autos(state: &AppState) -> Option<(String, String)> {
    let mut conn = state.pool.get().expect("conn");
    let autos = AutoRepository::obtener_todos(&mut conn).expect("autos");
    let placas: Vec<String> = autos
        .iter()
        .filter(|a| a.estado == "Disponible")
        .map(|a| a.placa.clone())
        .collect();
    if placas.len() >= 2 {
        Some((placas[0].clone(), placas[1].clone()))
    } else {
        None
    }
}

#[test]
#[serial]
fn renta_cambiar_auto_sin_cerrar() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some((placa_a, placa_b)) = dos_autos(&state) else {
        panic!(
            "BD de dev sin 2 autos Disponibles — se requiere flota real. Siembra \
             la BD dev (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures \
             y --commit)"
        );
    };

    let creada = RentaService::crear(&mut conn, cfg, datos_renta(&placa_a, None)).expect("crear");
    let id = creada.id;
    assert_eq!(creada.placa.as_deref(), Some(placa_a.as_str()));

    // ── Cambio de vehículo sin cerrar ──
    let cambiada = RentaService::cambiar_auto(&mut conn, id, &placa_b, "tester").expect("cambiar auto");
    assert_eq!(cambiada.placa.as_deref(), Some(placa_b.as_str()), "placa nueva");
    assert_eq!(cambiada.estado, "Activo", "la renta sigue activa");

    // El auto anterior queda Disponible y el nuevo Rentado
    let estado_a: Option<(String,)> =
        conn.query_first("SELECT estado FROM autos WHERE placa = ?", (placa_a.clone(),))
            .expect("estado a");
    assert_eq!(estado_a.map(|r| r.0).as_deref(), Some("Disponible"));
    let estado_b: Option<(String,)> =
        conn.query_first("SELECT estado FROM autos WHERE placa = ?", (placa_b.clone(),))
            .expect("estado b");
    assert_eq!(estado_b.map(|r| r.0).as_deref(), Some("Rentado"));

    // ── Mismo vehículo → no-op ──
    let misma = RentaService::cambiar_auto(&mut conn, id, &placa_b, "tester").expect("mismo auto");
    assert_eq!(misma.placa.as_deref(), Some(placa_b.as_str()));

    // ── Vehículo no disponible → business ──
    // Marcar B en Mantenimiento, volver a A (disponible) e intentar cambiar a B
    conn.execute(
        "UPDATE autos SET estado = 'Mantenimiento' WHERE placa = ?",
        (placa_b.clone(),),
    )
    .expect("marcar mantenimiento");
    RentaService::cambiar_auto(&mut conn, id, &placa_a, "tester").expect("volver a a");
    // La marca Mantenimiento de B se conserva (no se sobrescribe al liberarlo)
    let estado_b: Option<(String,)> =
        conn.query_first("SELECT estado FROM autos WHERE placa = ?", (placa_b.clone(),))
            .expect("estado b mantenimiento");
    assert_eq!(estado_b.map(|r| r.0).as_deref(), Some("Mantenimiento"));
    let err = RentaService::cambiar_auto(&mut conn, id, &placa_b, "tester").expect_err("no disponible");
    assert_eq!(err.kind(), "business");

    // ── Placa inexistente → business ──
    let err = RentaService::cambiar_auto(&mut conn, id, "ZZZ999", "tester").expect_err("placa inexistente");
    assert_eq!(err.kind(), "business");

    // ── Limpieza: restaurar estados y eliminar la renta ──
    conn.execute(
        "UPDATE autos SET estado = 'Disponible' WHERE placa = ?",
        (placa_b.clone(),),
    )
    .expect("restaurar b");
    conn.execute(
        "UPDATE autos SET estado = 'Disponible' WHERE placa = ?",
        (placa_a.clone(),),
    )
    .expect("restaurar a");
    RentaService::eliminar(&mut conn, id).expect("limpieza");
}

#[test]
#[serial]
fn renta_cierre_calcula_dias_horas_automatico() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    // Renta de 2 días: recogida 2026-01-01 10:00 → retorno 2026-01-03 10:00
    let mut base = datos_renta(&placa, None);
    base.fecha_recogida = "2026-01-01".into();
    base.hora_recogida = Some("10:00".into());
    base.fecha_retorno = "2026-01-03".into();
    base.hora_retorno = Some("10:00".into());
    base.dias_calculados = 2;

    // ── Excedente 6 h (> 3 h) → día completo ──
    let creada = RentaService::crear(&mut conn, cfg, base.clone()).expect("crear");
    let cierre = RentaCierreDatos {
        fecha_devolucion_real: Some("2026-01-03".into()),
        hora_devolucion_real: Some("16:00".into()),
        dias_calculados: None,
        horas_extras: None,
        ..Default::default()
    };
    let cerrada = RentaService::cerrar(&mut conn, cfg, creada.id, cierre).expect("cerrar tarde");
    assert_eq!(cerrada.dias_calculados, 3, "6 h de excedente → día completo");
    assert_eq!(cerrada.horas_extras, 0);
    RentaService::eliminar(&mut conn, creada.id).expect("limpieza");

    // ── Excedente 2 h (≤ 3 h) → 2 horas extras ──
    let creada2 = RentaService::crear(&mut conn, cfg, base).expect("crear2");
    let cierre2 = RentaCierreDatos {
        fecha_devolucion_real: Some("2026-01-03".into()),
        hora_devolucion_real: Some("12:00".into()),
        dias_calculados: None,
        horas_extras: None,
        ..Default::default()
    };
    let cerrada2 = RentaService::cerrar(&mut conn, cfg, creada2.id, cierre2).expect("cerrar puntual");
    assert_eq!(cerrada2.dias_calculados, 2);
    assert_eq!(cerrada2.horas_extras, 2, "2 h de excedente → horas extras");
    RentaService::eliminar(&mut conn, creada2.id).expect("limpieza2");
}
