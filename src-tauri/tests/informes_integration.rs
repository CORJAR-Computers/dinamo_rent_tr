//! informes_integration.rs — Pruebas de integración del servicio de informes
//! contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Verifica el balance mensual: consistencia de ingresos/egresos, el mes por
//! defecto y la lista de rentas del período.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Local};
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::repositories::renta::RentaDatos;
use dinamo_rent_lib::services::informe::InformeService;
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

#[test]
#[serial]
fn informe_mensual_balance_consistente() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    // Mes por defecto (actual)
    let informe =
        InformeService::mensual(&mut conn, "2026-08-01", "2026-08-31").expect("informe mes actual");
    assert_eq!(informe.fecha_inicio, "2026-08-01", "fecha de inicio");
    assert_eq!(informe.fecha_fin, "2026-08-31", "fecha fin");

    // Balance = total_ingresos − total_egresos (consistencia interna)
    let ingresos: rust_decimal::Decimal = informe.total_ingresos.parse().expect("numérico");
    let egresos: rust_decimal::Decimal = informe.total_egresos.parse().expect("numérico");
    let balance: rust_decimal::Decimal = informe.balance.parse().expect("numérico");
    assert_eq!(balance, ingresos - egresos, "balance = ingresos − egresos");

    // Netos: ingresos_netos = total_ingresos − total_comisiones;
    // balance_neto = balance − total_comisiones (consistencia interna)
    let comisiones: rust_decimal::Decimal = informe
        .total_comisiones
        .parse()
        .expect("comisiones numérico");
    let ingresos_netos: rust_decimal::Decimal =
        informe.ingresos_netos.parse().expect("netos numérico");
    let balance_neto: rust_decimal::Decimal =
        informe.balance_neto.parse().expect("balance neto numérico");
    assert_eq!(
        ingresos_netos,
        ingresos - comisiones,
        "ingresos netos = ingresos − comisiones"
    );
    assert_eq!(
        balance_neto,
        balance - comisiones,
        "balance neto = balance − comisiones"
    );

    // Los totales parciales suman el total
    let pagos: rust_decimal::Decimal = informe.ingresos_pagos.parse().expect("numérico");
    let reservas: rust_decimal::Decimal = informe.ingresos_reservas.parse().expect("numérico");
    assert_eq!(
        ingresos,
        pagos + reservas,
        "total ingresos = pagos + reservas"
    );

    // La estructura de rentas del mes es válida
    for r in &informe.rentas {
        assert!(r.id > 0);
        assert!(!r.nombre_cliente.is_empty());
    }
}

#[test]
#[serial]
fn informe_utilidad_por_vehiculo() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    let informe = InformeService::mensual(&mut conn, "2026-08-01", "2026-08-31").expect("informe");

    // La utilidad de cada vehículo = ingresos − costos (consistencia interna)
    for v in &informe.utilidad_por_vehiculo {
        assert!(!v.placa.is_empty(), "toda fila tiene placa");
        let ingresos: rust_decimal::Decimal = v.ingresos.parse().expect("ingresos numérico");
        let costos: rust_decimal::Decimal = v.costos.parse().expect("costos numérico");
        let utilidad: rust_decimal::Decimal = v.utilidad.parse().expect("utilidad numérico");
        assert_eq!(
            utilidad,
            ingresos - costos,
            "utilidad = ingresos − costos ({})",
            v.placa
        );
    }

    // Ordenadas de mayor a menor utilidad
    let utilidades: Vec<rust_decimal::Decimal> = informe
        .utilidad_por_vehiculo
        .iter()
        .map(|v| v.utilidad.parse().expect("numérico"))
        .collect();
    assert!(
        utilidades.windows(2).all(|w| w[0] >= w[1]),
        "orden descendente"
    );
}

#[test]
#[serial]
fn informe_mensual_especifico() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    // Un mes con datos mínimos esperados (2026-08 tiene tablas de prueba)
    let informe =
        InformeService::mensual(&mut conn, "2026-08-01", "2026-08-31").expect("informe 2026-08");
    assert_eq!(informe.fecha_inicio, "2026-08-01");
    assert_eq!(informe.fecha_fin, "2026-08-31");

    // Los montos devueltos son cadenas numéricas válidas
    for m in [
        &informe.ingresos_pagos,
        &informe.ingresos_reservas,
        &informe.egresos_gastos,
        &informe.egresos_mantenimiento,
        &informe.egresos_comparendos,
        &informe.total_ingresos,
        &informe.total_egresos,
        &informe.balance,
        &informe.total_comisiones,
        &informe.ingresos_netos,
        &informe.balance_neto,
    ] {
        assert!(
            m.parse::<rust_decimal::Decimal>().is_ok(),
            "monto numérico: {m}"
        );
    }
    // Gastos por categoría: claves no vacías
    for (cat, total) in &informe.gastos_por_categoria {
        assert!(!cat.is_empty());
        assert!(total.parse::<rust_decimal::Decimal>().is_ok());
    }
}

#[test]
#[serial]
fn informe_comisiones_y_balance_neto() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let Some(placa) = (|| {
        let autos = AutoRepository::obtener_todos(&mut conn).ok()?;
        autos.first().map(|a| a.placa.clone())
    })() else {
        panic!("BD de dev sin autos — se requiere flota real.");
    };

    let hoy = Local::now().date_naive();
    let dia = hoy.format("%Y-%m-%d").to_string();

    // Baseline del día antes de crear la renta con comisión
    let antes = InformeService::mensual(&mut conn, &dia, &dia).expect("informe antes");
    let comisiones_antes: rust_decimal::Decimal = antes.total_comisiones.parse().expect("numérico");
    let ingresos_netos_antes: rust_decimal::Decimal =
        antes.ingresos_netos.parse().expect("numérico");
    let balance_neto_antes: rust_decimal::Decimal = antes.balance_neto.parse().expect("numérico");

    // Renta de hoy con comisión de 120.000: total 178.500 (1 día × 150.000
    // + 19% IVA) → valor neto 58.500
    let datos = RentaDatos {
        placa: Some(placa.clone()),
        nombre_cliente: "CLIENTE INFORME COMISION".into(),
        fecha_recogida: dia.clone(),
        hora_recogida: Some("09:00".into()),
        fecha_retorno: (hoy + Duration::days(1)).format("%Y-%m-%d").to_string(),
        hora_retorno: Some("18:00".into()),
        dias_calculados: 1,
        horas_extras: 0,
        valor_dia: "150000".into(),
        valor_hora_extra: "10000".into(),
        cobra_iva: true,
        tiene_comision: true,
        comision: "120000".into(),
        ..RentaDatos::default()
    };
    let creada = RentaService::crear(&mut conn, cfg, datos).expect("crear renta con comisión");
    assert_eq!(creada.valor_neto, "58500.00", "neto = 178.500 − 120.000");

    let despues = InformeService::mensual(&mut conn, &dia, &dia).expect("informe después");
    let comisiones_despues: rust_decimal::Decimal =
        despues.total_comisiones.parse().expect("numérico");
    let ingresos_netos_despues: rust_decimal::Decimal =
        despues.ingresos_netos.parse().expect("numérico");
    let balance_neto_despues: rust_decimal::Decimal =
        despues.balance_neto.parse().expect("numérico");

    assert_eq!(
        comisiones_despues - comisiones_antes,
        rust_decimal::Decimal::from(120000),
        "la renta suma su comisión al total del período"
    );
    assert_eq!(
        ingresos_netos_despues - ingresos_netos_antes,
        rust_decimal::Decimal::from(-120000),
        "los ingresos netos caen por la comisión"
    );
    assert_eq!(
        balance_neto_despues - balance_neto_antes,
        rust_decimal::Decimal::from(-120000),
        "el balance neto cae por la comisión"
    );

    // La renta aparece en el detalle con su comisión y valor neto
    let renta = despues
        .rentas
        .iter()
        .find(|r| r.id == creada.id)
        .expect("renta en el informe");
    assert_eq!(renta.comision, "120000.00");
    assert_eq!(renta.valor_neto, "58500.00");

    // La utilidad por vehículo incluye la comisión como costo
    let util = despues
        .utilidad_por_vehiculo
        .iter()
        .find(|v| v.placa == placa)
        .expect("vehículo en utilidad");
    let costos: rust_decimal::Decimal = util.costos.parse().expect("costos numérico");
    assert!(
        costos >= rust_decimal::Decimal::from(120000),
        "los costos del vehículo incluyen la comisión"
    );

    RentaService::eliminar(&mut conn, creada.id, "test").expect("limpieza");
}

/// RBAC: `informe_mensual` solo es accesible para los roles de
/// `roles_con_informes` (config.ini — por defecto solo Administrador).
#[test]
#[serial]
fn informe_requiere_roles_con_informes() {
    let state = dev_state();

    // Operador y Supervisor → denegados con kind "permission"
    {
        let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let token_op = sessions.create(1, "operador", "Operador", "Op", false);
        let token_sup = sessions.create(2, "supervisor", "Supervisor", "Sup", false);
        drop(sessions);
        let err_op = dinamo_rent_lib::commands::require_informes(&state, &token_op)
            .expect_err("Operador no puede consultar informes");
        assert_eq!(err_op.kind, "permission");
        let err_sup = dinamo_rent_lib::commands::require_informes(&state, &token_sup)
            .expect_err("Supervisor no puede consultar informes (solo Admin)");
        assert_eq!(err_sup.kind, "permission");
    }

    // Administrador → permitido (único rol en default de roles_con_informes)
    {
        let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let token_admin = sessions.create(3, "admin", "Administrador", "Adm", false);
        drop(sessions);
        assert!(
            dinamo_rent_lib::commands::require_informes(&state, &token_admin).is_ok(),
            "Administrador tiene rol de informes"
        );
    }

    // Sin sesión → session_expired
    {
        let err = dinamo_rent_lib::commands::require_informes(&state, "no-existe")
            .expect_err("sin sesión no se accede a informes");
        assert_eq!(err.kind, "session_expired");
    }
}
