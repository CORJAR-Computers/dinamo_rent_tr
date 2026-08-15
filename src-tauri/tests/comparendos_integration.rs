//! comparendos_integration.rs — Pruebas de integración del servicio de
//! comparendos contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Usa un auto real de la BD (solo lectura) y crea/elimina comparendos
//! temporales en cada test. Verifica CRUD, marcado de pago y totales.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Local};
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::repositories::comparendo::ComparendoDatos;
use dinamo_rent_lib::repositories::renta::RentaDatos;
use dinamo_rent_lib::services::comparendo::ComparendoService;
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

fn datos_comparendo(placa: &str, monto: &str) -> ComparendoDatos {
    let hoy = Local::now().date_naive();
    ComparendoDatos {
        placa: placa.into(),
        fecha_infraccion: hoy.format("%Y-%m-%d").to_string(),
        hora_infraccion: "14:30".into(),
        monto: monto.into(),
        numero_comparendo: None,
        id_renta: None,
        id_cliente: None,
        estado: "Pendiente".into(),
        observaciones: Some("Exceso de velocidad".into()),
    }
}

#[test]
#[serial]
fn comparendo_crud_y_marcar_pagado() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let mut datos = datos_comparendo(&placa, "580000");
    let creado = ComparendoService::crear(&mut conn, cfg, datos.clone()).expect("crear comparendo");
    let id = creado.id;
    assert_eq!(creado.placa, placa);
    assert_eq!(creado.monto, "580000.00", "monto normalizado con 2 decimales");
    assert_eq!(creado.estado, "Pendiente");
    assert_eq!(creado.hora_infraccion, "14:30", "hora recortada a HH:MM");
    assert!(!creado.vehiculo.is_empty(), "JOIN con autos para la UI");

    // Obtener
    let obtenido = ComparendoService::obtener(&mut conn, id).expect("obtener");
    assert_eq!(obtenido.fecha_infraccion, datos.fecha_infraccion);

    // Actualizar
    datos.monto = "650000".into();
    datos.observaciones = Some("Foto-detección".into());
    let actualizado =
        ComparendoService::actualizar(&mut conn, cfg, id, datos.clone()).expect("actualizar");
    assert_eq!(actualizado.monto, "650000.00");
    assert_eq!(actualizado.observaciones.as_deref(), Some("Foto-detección"));

    // Marcar pagado
    let pagado = ComparendoService::marcar_pagado(&mut conn, id).expect("marcar pagado");
    assert_eq!(pagado.estado, "Pagado");
    // Idempotente
    let otra = ComparendoService::marcar_pagado(&mut conn, id).expect("marcar pagado 2");
    assert_eq!(otra.estado, "Pagado");

    // Historial por placa
    let historial = ComparendoService::listar(&mut conn, None, Some(&placa), None).expect("historial");
    assert!(historial.iter().any(|c| c.id == id));

    // Eliminar
    ComparendoService::eliminar(&mut conn, id).expect("eliminar");
    assert!(ComparendoService::obtener(&mut conn, id).is_err(), "eliminado");
}

#[test]
#[serial]
fn comparendo_validaciones() {
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
    let sin_auto = datos_comparendo("ZZZ999", "100000");
    let err = ComparendoService::crear(&mut conn, cfg, sin_auto).expect_err("placa inexistente");
    assert_eq!(err.kind(), "business");

    // Placa vacía → validation
    let mut sin_placa = datos_comparendo(&placa, "100000");
    sin_placa.placa = "".into();
    let err = ComparendoService::crear(&mut conn, cfg, sin_placa).expect_err("placa vacía");
    assert_eq!(err.kind(), "validation");

    // Hora inválida → validation
    let mut hora = datos_comparendo(&placa, "100000");
    hora.hora_infraccion = "25:99".into();
    let err = ComparendoService::crear(&mut conn, cfg, hora).expect_err("hora inválida");
    assert_eq!(err.kind(), "validation");

    // Fecha inválida → validation
    let mut fecha = datos_comparendo(&placa, "100000");
    fecha.fecha_infraccion = "no-es-fecha".into();
    let err = ComparendoService::crear(&mut conn, cfg, fecha).expect_err("fecha inválida");
    assert_eq!(err.kind(), "validation");

    // Monto cero → validation
    let cero = datos_comparendo(&placa, "0");
    let err = ComparendoService::crear(&mut conn, cfg, cero).expect_err("monto cero");
    assert_eq!(err.kind(), "validation");

    // Monto inválido → validation
    let inv = datos_comparendo(&placa, "abc");
    let err = ComparendoService::crear(&mut conn, cfg, inv).expect_err("monto inválido");
    assert_eq!(err.kind(), "validation");

    // Estado inválido → validation
    let mut est = datos_comparendo(&placa, "100000");
    est.estado = "Apelado".into();
    let err = ComparendoService::crear(&mut conn, cfg, est).expect_err("estado inválido");
    assert_eq!(err.kind(), "validation");

    // XSS en observaciones → validation
    let mut xss = datos_comparendo(&placa, "100000");
    xss.observaciones = Some("<script>alert(1)</script>".into());
    let err = ComparendoService::crear(&mut conn, cfg, xss).expect_err("xss");
    assert_eq!(err.kind(), "validation");
}

#[test]
#[serial]
fn comparendo_totales() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let c1 = ComparendoService::crear(&mut conn, cfg, datos_comparendo(&placa, "150000"))
        .expect("crear c1");
    let c2 = ComparendoService::crear(&mut conn, cfg, datos_comparendo(&placa, "90000"))
        .expect("crear c2");
    ComparendoService::marcar_pagado(&mut conn, c1.id).expect("pagar c1");

    let totales = ComparendoService::totales(&mut conn).expect("totales");
    let monto_total: rust_decimal::Decimal = totales.total_general.parse().expect("numérico");
    assert!(monto_total >= rust_decimal::Decimal::from(240_000), "total >= 240000");
    assert!(
        totales.por_placa.iter().any(|t| t.clave == placa),
        "la placa aparece en los totales por placa"
    );
    assert!(
        totales.por_estado.iter().any(|t| t.clave == "Pendiente"),
        "el estado aparece en los totales por estado"
    );

    let total = ComparendoService::contar(&mut conn).expect("contar");
    assert!(total >= 2);

    ComparendoService::eliminar(&mut conn, c1.id).expect("eliminar c1");
    ComparendoService::eliminar(&mut conn, c2.id).expect("eliminar c2");
}

#[test]
#[serial]
fn comparendo_numero_oficial_y_dedup() {
    // Verifica el número oficial (fuente SIMIT): round-trip del campo y los
    // métodos de deduplicación que usa el Agente SIMIT.
    use dinamo_rent_lib::repositories::comparendo::ComparendoRepository;

    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let mut datos = datos_comparendo(&placa, "320000");
    datos.numero_comparendo = Some("TEST-250010000000999".into());
    let creado = ComparendoService::crear(&mut conn, cfg, datos.clone()).expect("crear");
    assert_eq!(
        creado.numero_comparendo.as_deref(),
        Some("TEST-250010000000999"),
        "el número viaja hasta la fila serializable"
    );

    // Dedup por número (el caso real del agente: mismo número → no re-insertar)
    assert!(
        ComparendoRepository::existe_por_numero(&mut conn, "TEST-250010000000999").expect("num"),
        "el número ya existe en la BD"
    );
    assert!(
        !ComparendoRepository::existe_por_numero(&mut conn, "TEST-999").expect("num 2"),
        "número distinto no existe"
    );

    // Dedup por placa + fecha + monto (fallback sin número)
    assert!(
        ComparendoRepository::existe_duplicado(
            &mut conn,
            &placa,
            &datos.fecha_infraccion,
            "320000.00"
        )
        .expect("dup"),
        "misma placa/fecha/monto → duplicado"
    );
    assert!(
        !ComparendoRepository::existe_duplicado(
            &mut conn,
            &placa,
            &datos.fecha_infraccion,
            "1.00"
        )
        .expect("dup 2"),
        "monto distinto → no duplicado"
    );

    // El número se conserva al editar
    datos.monto = "340000".into();
    let actualizado =
        ComparendoService::actualizar(&mut conn, cfg, creado.id, datos).expect("actualizar");
    assert_eq!(
        actualizado.numero_comparendo.as_deref(),
        Some("TEST-250010000000999"),
        "el número no se pierde al actualizar"
    );

    // Sincronización de estado: el SIMIT reporta pagado → la BD converge
    // (solo toca registros pendientes con ese número).
    ComparendoRepository::marcar_pagado_por_numero(&mut conn, "TEST-250010000000999")
        .expect("marcar por número");
    let obtenido = ComparendoService::obtener(&mut conn, creado.id).expect("obtener");
    assert_eq!(obtenido.estado, "Pagado", "estado sincronizado desde el número");

    ComparendoService::eliminar(&mut conn, creado.id).expect("eliminar");
    assert!(
        !ComparendoRepository::existe_por_numero(&mut conn, "TEST-250010000000999")
            .expect("num after delete"),
        "soft-delete excluye el número (dedup solo sobre activos)"
    );
}

/// Renta temporal con rango de fechas dado (para el cruce con comparendos)
fn datos_renta_cruce(placa: &str, recogida: &str, retorno: &str) -> RentaDatos {
    RentaDatos {
        placa: Some(placa.into()),
        id_cliente: None,
        nombre_cliente: "CLIENTE CRUCE TEST".into(),
        no_licencia: None,
        nacionalidad: None,
        fecha_recogida: recogida.into(),
        hora_recogida: Some("09:00".into()),
        ubicacion_recogida: None,
        fecha_retorno: retorno.into(),
        hora_retorno: Some("18:00".into()),
        ubicacion_retorno: None,
        dias_calculados: 5,
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

/// El cruce con rentas responde QUIÉN tenía el vehículo el día de la multa:
/// la renta del mismo vehículo cuyo rango [recogida, retorno] contiene la
/// fecha de la infracción (None si el vehículo no estaba rentado ese día).
#[test]
#[serial]
fn comparendo_cruce_responsable_renta() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    let hoy = Local::now().date_naive();
    // Renta temporal en el pasado (60-55 días atrás): no pisa rentas reales de
    // la BD dev (recientes) ni las de otros tests.
    let recogida = hoy - Duration::days(60);
    let retorno = hoy - Duration::days(55);
    let renta = RentaService::crear(
        &mut conn,
        cfg,
        datos_renta_cruce(
            &placa,
            &recogida.format("%Y-%m-%d").to_string(),
            &retorno.format("%Y-%m-%d").to_string(),
        ),
    )
    .expect("crear renta");
    assert_eq!(renta.nombre_cliente, "CLIENTE CRUCE TEST");

    // Comparendo DENTRO del rango → el responsable es la renta del test
    let mut c1 = datos_comparendo(&placa, "450000");
    c1.fecha_infraccion = (hoy - Duration::days(58)).format("%Y-%m-%d").to_string();
    let c1 = ComparendoService::crear(&mut conn, cfg, c1).expect("crear comparendo dentro");
    // La atribución se PERSISTE en el alta (id_renta/id_cliente resueltos)
    assert_eq!(
        c1.id_renta,
        Some(renta.id),
        "el alta resuelve y guarda la renta del día"
    );
    assert_eq!(c1.id_cliente, renta.id_cliente, "id_cliente heredado de la renta");

    // Comparendo FUERA del rango (30 días después del retorno) → sin responsable
    let mut c2 = datos_comparendo(&placa, "450001");
    c2.fecha_infraccion = (hoy + Duration::days(30)).format("%Y-%m-%d").to_string();
    let c2 = ComparendoService::crear(&mut conn, cfg, c2).expect("crear comparendo fuera");
    assert!(
        c2.id_renta.is_none() && c2.id_cliente.is_none(),
        "fuera del rango → sin renta atribuida"
    );

    let lista = ComparendoService::listar(&mut conn, None, Some(&placa), None).expect("listar");

    let dentro = lista
        .iter()
        .find(|c| c.id == c1.id)
        .expect("comparendo dentro en la lista");
    let resp = dentro
        .responsable
        .as_ref()
        .expect("comparendo dentro del rango → responsable asignado");
    assert_eq!(resp.id_renta, renta.id);
    assert_eq!(resp.nombre_cliente, "CLIENTE CRUCE TEST");
    assert_eq!(resp.no_contrato, renta.no_contrato);
    assert_eq!(resp.anio_contrato, renta.anio_contrato);
    assert_eq!(
        resp.fecha_recogida,
        recogida.format("%Y-%m-%d").to_string(),
        "rango de la renta reportado"
    );

    let fuera = lista
        .iter()
        .find(|c| c.id == c2.id)
        .expect("comparendo fuera en la lista");
    assert!(
        fuera.responsable.is_none(),
        "comparendo fuera del rango → sin responsable (el vehículo no estaba rentado)"
    );

    // Limpieza (comparendos soft-delete; la renta cae limpia)
    ComparendoService::eliminar(&mut conn, c1.id).expect("limpiar c1");
    ComparendoService::eliminar(&mut conn, c2.id).expect("limpiar c2");
    RentaService::eliminar(&mut conn, renta.id).expect("limpiar renta");
}
