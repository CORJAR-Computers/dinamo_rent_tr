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
        origen: None,
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
    assert_eq!(actualizado.observaciones.as_deref(), Some("FOTO-DETECCIÓN"));

    // Marcar pagado
    let pagado = ComparendoService::marcar_pagado(&mut conn, id).expect("marcar pagado");
    assert_eq!(pagado.estado, "Pagado");
    // Idempotente
    let otra = ComparendoService::marcar_pagado(&mut conn, id).expect("marcar pagado 2");
    assert_eq!(otra.estado, "Pagado");

    // Historial por placa
    let historial =
        ComparendoService::listar(&mut conn, None, Some(&placa), None, false).expect("historial");
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

#[test]
#[serial]
fn comparendo_origen_simit_y_ultimo_visto() {
    // Procedencia persistente («nuevos vs ya registrados»): el Agente inserta
    // con origen SIMIT (deja ultimo_visto_simit = ahora); un registro manual
    // queda 'Manual' con ultimo_visto NULL; marcar_visto_simit_por_id converge
    // un Manual a SIMIT y toca la confirmación; id_existente deduplica y
    // devuelve el id del registro existente.
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

    // 1) Manual (default) → origen 'Manual', ultimo_visto_simit NULL
    let manual = ComparendoService::crear(&mut conn, cfg, datos_comparendo(&placa, "111000"))
        .expect("crear manual");
    assert_eq!(manual.origen, "Manual", "registro manual sin origen SIMIT");
    assert!(
        manual.ultimo_visto_simit.is_none(),
        "manual sin confirmación SIMIT"
    );

    // 2) El Agente inserta con origen SIMIT y luego toca la confirmación
    //    (misma secuencia que sincronizar: insertar + marcar_visto_simit_por_id)
    let mut datos = datos_comparendo(&placa, "222000");
    datos.numero_comparendo = Some("TEST-ORIGEN-SIMIT-001".into());
    datos.origen = Some("SIMIT".into());
    let id_simit = ComparendoRepository::insertar(&mut conn, &datos).expect("insertar SIMIT");
    ComparendoRepository::marcar_visto_simit_por_id(&mut conn, id_simit).expect("marcar visto");
    let simit = ComparendoService::obtener(&mut conn, id_simit).expect("obtener SIMIT");
    assert_eq!(simit.origen, "SIMIT", "el Agente marca la procedencia");
    assert!(
        simit.ultimo_visto_simit.is_some(),
        "la confirmación queda registrada"
    );

    // 3) Dedup por número → devuelve el id del registro existente
    let id_dup = ComparendoRepository::id_existente(
        &mut conn,
        Some("TEST-ORIGEN-SIMIT-001"),
        &placa,
        &datos.fecha_infraccion,
        "222000.00",
    )
    .expect("id_existente");
    assert_eq!(id_dup, Some(id_simit), "mismo número → mismo id");

    // 4) Un comparendo manual que el SIMIT reporta converge a SIMIT al tocarlo
    ComparendoRepository::marcar_visto_simit_por_id(&mut conn, manual.id).expect("marcar visto");
    let convergido = ComparendoService::obtener(&mut conn, manual.id).expect("obtener convergido");
    assert_eq!(convergido.origen, "SIMIT", "confirmado por SIMIT ya no es manual");
    assert!(
        convergido.ultimo_visto_simit.is_some(),
        "toca la confirmación al re-verlo"
    );

    // Limpieza
    ComparendoService::eliminar(&mut conn, id_simit).expect("eliminar simit");
    ComparendoService::eliminar(&mut conn, manual.id).expect("eliminar manual");
}

#[test]
#[serial]
fn comparendo_no_confirmados_simit() {
    // Filtro «el SIMIT dejó de confirmar»: entran los de origen SIMIT con
    // ultimo_visto_simit nulo o anterior al corte; salen los recién
    // confirmados y los manuales (el SIMIT nunca los confirma, es lo esperado).
    use dinamo_rent_lib::repositories::comparendo::ComparendoRepository;
    use dinamo_rent_lib::services::comparendo::DIAS_SIN_CONFIRMAR_SIMIT;
    use rsfbclient::Execute;

    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = auto_real(&state) else {
        panic!(
            "BD de dev sin autos — se requiere flota real. Siembra la BD dev \
             (Handsoff §6.3: importar_autos_clientes.py con scripts/fixtures y --commit)"
        );
    };

    // a) SIMIT sin confirmar (nunca visto) → ENTRA
    let mut a = datos_comparendo(&placa, "331000");
    a.numero_comparendo = Some("TEST-NO-CONF-1".into());
    a.origen = Some("SIMIT".into());
    let id_sin_confirmar = ComparendoRepository::insertar(&mut conn, &a).expect("insertar a");

    // b) SIMIT confirmado hace mucho (10 días) → ENTRA
    let mut b = datos_comparendo(&placa, "332000");
    b.numero_comparendo = Some("TEST-NO-CONF-2".into());
    b.origen = Some("SIMIT".into());
    let id_viejo = ComparendoRepository::insertar(&mut conn, &b).expect("insertar b");
    conn.execute(
        "UPDATE comparendos SET ultimo_visto_simit = CURRENT_TIMESTAMP - 10 \
         WHERE id = ?",
        (id_viejo,),
    )
    .expect("envejecer confirmación");

    // c) SIMIT recién confirmado (ahora) → NO entra
    let mut c = datos_comparendo(&placa, "333000");
    c.numero_comparendo = Some("TEST-NO-CONF-3".into());
    c.origen = Some("SIMIT".into());
    let id_reciente = ComparendoRepository::insertar(&mut conn, &c).expect("insertar c");
    ComparendoRepository::marcar_visto_simit_por_id(&mut conn, id_reciente)
        .expect("confirmar reciente");

    // d) Manual (nunca lo confirma el SIMIT) → NO entra
    let manual =
        ComparendoService::crear(&mut conn, cfg, datos_comparendo(&placa, "334000"))
            .expect("crear manual");

    let ids: Vec<i64> = ComparendoService::listar(&mut conn, None, None, None, true)
        .expect("listar no confirmados")
        .into_iter()
        .map(|c| c.id)
        .collect();

    assert!(
        ids.contains(&id_sin_confirmar),
        "SIMIT nunca visto → entra (ids: {ids:?})"
    );
    assert!(ids.contains(&id_viejo), "confirmado hace {DIAS_SIN_CONFIRMAR_SIMIT}+ días → entra");
    assert!(
        !ids.contains(&id_reciente),
        "recién confirmado → no entra (ids: {ids:?})"
    );
    assert!(!ids.contains(&manual.id), "manual → nunca entra");

    // Sin el filtro vuelve todo
    let todos: Vec<i64> = ComparendoService::listar(&mut conn, None, None, None, false)
        .expect("listar todos")
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert!(todos.contains(&id_reciente), "sin filtro incluye el reciente");

    // Limpieza
    ComparendoService::eliminar(&mut conn, id_sin_confirmar).expect("limpiar a");
    ComparendoService::eliminar(&mut conn, id_viejo).expect("limpiar b");
    ComparendoService::eliminar(&mut conn, id_reciente).expect("limpiar c");
    ComparendoService::eliminar(&mut conn, manual.id).expect("limpiar manual");
}

#[test]
#[serial]
fn persistencia_ultimo_resultado_simit() {
    // El filtro «Solo nuevos» sobrevive al reinicio: el resultado de la última
    // sincronización se persiste como JSON (una fila, upsert) y se restaura.
    // Round-trip completo: persistir → cargar → re-persistir (upsert).
    use dinamo_rent_lib::services::simit::{
        cargar_ultimo_resultado, persistir_ultimo_resultado, ErrorPlacaSimit, MetricasSimit,
        RegistroSimit, ResultadoSincronizacion,
    };
    use rsfbclient::{Execute, Queryable};

    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    conn.execute("DELETE FROM agente_simit_ultimo_resultado", ())
        .expect("limpiar estado inicial");

    let resultado = ResultadoSincronizacion {
        sincronizado_en: "2026-08-17T10:30:00-05:00".into(),
        placas_consultadas: 2,
        placas_con_error: 0,
        encontrados: 2,
        insertados: 1,
        duplicados: 1,
        total_pendiente: "900000.00".into(),
        registros: vec![RegistroSimit {
            numero: Some("TEST-0022".into()),
            placa: "ABC123".into(),
            fecha_infraccion: "2026-08-01".into(),
            hora_infraccion: "14:30".into(),
            monto: "580000.00".into(),
            estado: "Pendiente".into(),
            organismo: "Policía de Tránsito".into(),
            codigo_infraccion: "C24".into(),
            descripcion: "Exceso de velocidad".into(),
            es_comparendo: true,
            nuevo: true,
            id: Some(42),
        }],
        errores: vec![ErrorPlacaSimit {
            placa: "ZZZ111".into(),
            error: "timeout".into(),
        }],
        reporte_html: Some("C:\\tmp\\reporte.html".into()),
        metricas: MetricasSimit {
            tiempo_total_ms: 1200,
            ..Default::default()
        },
    };

    // Round-trip: persistir → cargar → misma información
    persistir_ultimo_resultado(&mut conn, &resultado).expect("persistir");
    let cargado = cargar_ultimo_resultado(&mut conn)
        .expect("cargar")
        .expect("debe haber resultado persistido");
    assert_eq!(cargado.sincronizado_en, resultado.sincronizado_en);
    assert_eq!(cargado.insertados, 1);
    assert_eq!(cargado.registros.len(), 1);
    let reg = &cargado.registros[0];
    assert!(reg.nuevo, "el registro nuevo conserva su flag");
    assert_eq!(reg.id, Some(42), "el id del comparendo sobrevive (filtro «Solo nuevos»)");
    assert_eq!(reg.numero.as_deref(), Some("TEST-0022"));
    assert_eq!(cargado.errores[0].placa, "ZZZ111");
    assert_eq!(cargado.reporte_html.as_deref(), Some("C:\\tmp\\reporte.html"));

    // Upsert: una segunda corrida reemplaza (sigue siendo una sola fila)
    let mut segunda = resultado.clone();
    segunda.sincronizado_en = "2026-08-17T12:30:00-05:00".into();
    segunda.insertados = 3;
    persistir_ultimo_resultado(&mut conn, &segunda).expect("persistir 2");

    let filas: Option<(i64,)> = conn
        .query_first("SELECT COUNT(*) FROM agente_simit_ultimo_resultado", ())
        .expect("contar filas");
    assert_eq!(filas.map(|r| r.0), Some(1), "el upsert mantiene una sola fila");

    let cargado2 = cargar_ultimo_resultado(&mut conn)
        .expect("cargar 2")
        .expect("debe seguir existiendo");
    assert_eq!(cargado2.sincronizado_en, "2026-08-17T12:30:00-05:00");
    assert_eq!(cargado2.insertados, 3);

    // Limpieza
    conn.execute("DELETE FROM agente_simit_ultimo_resultado", ())
        .expect("limpiar fin");
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

    let lista =
        ComparendoService::listar(&mut conn, None, Some(&placa), None, false).expect("listar");

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
    RentaService::eliminar(&mut conn, renta.id, "test").expect("limpiar renta");
}
