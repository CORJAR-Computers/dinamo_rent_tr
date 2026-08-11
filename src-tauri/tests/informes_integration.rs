//! informes_integration.rs — Pruebas de integración del servicio de informes
//! contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Verifica el balance mensual: consistencia de ingresos/egresos, el mes por
//! defecto y la lista de rentas del período.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::services::informe::InformeService;
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

#[test]
#[serial]
fn informe_mensual_balance_consistente() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    // Mes por defecto (actual)
    let informe = InformeService::mensual(&mut conn, "2026-08-01", "2026-08-31").expect("informe mes actual");
    assert_eq!(informe.fecha_inicio, "2026-08-01", "fecha de inicio");
    assert_eq!(informe.fecha_fin, "2026-08-31", "fecha fin");

    // Balance = total_ingresos − total_egresos (consistencia interna)
    let ingresos: rust_decimal::Decimal = informe.total_ingresos.parse().expect("numérico");
    let egresos: rust_decimal::Decimal = informe.total_egresos.parse().expect("numérico");
    let balance: rust_decimal::Decimal = informe.balance.parse().expect("numérico");
    assert_eq!(
        balance,
        ingresos - egresos,
        "balance = ingresos − egresos"
    );

    // Los totales parciales suman el total
    let pagos: rust_decimal::Decimal = informe.ingresos_pagos.parse().expect("numérico");
    let reservas: rust_decimal::Decimal = informe.ingresos_reservas.parse().expect("numérico");
    assert_eq!(ingresos, pagos + reservas, "total ingresos = pagos + reservas");

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
        assert_eq!(utilidad, ingresos - costos, "utilidad = ingresos − costos ({})", v.placa);
    }

    // Ordenadas de mayor a menor utilidad
    let utilidades: Vec<rust_decimal::Decimal> = informe
        .utilidad_por_vehiculo
        .iter()
        .map(|v| v.utilidad.parse().expect("numérico"))
        .collect();
    assert!(utilidades.windows(2).all(|w| w[0] >= w[1]), "orden descendente");
}

#[test]
#[serial]
fn informe_mensual_especifico() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    // Un mes con datos mínimos esperados (2026-08 tiene tablas de prueba)
    let informe = InformeService::mensual(&mut conn, "2026-08-01", "2026-08-31").expect("informe 2026-08");
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
    ] {
        assert!(m.parse::<rust_decimal::Decimal>().is_ok(), "monto numérico: {m}");
    }
    // Gastos por categoría: claves no vacías
    for (cat, total) in &informe.gastos_por_categoria {
        assert!(!cat.is_empty());
        assert!(total.parse::<rust_decimal::Decimal>().is_ok());
    }
}

/// RBAC: `informe_mensual` solo es accesible para los roles de
/// `roles_con_informes` (config.ini — por defecto Administrador y Supervisor).
#[test]
#[serial]
fn informe_requiere_roles_con_informes() {
    let state = dev_state();

    // Operador → denegado con kind "permission"
    {
        let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let token = sessions.create(1, "operador", "Operador", "Op", false);
        drop(sessions);
        let err = dinamo_rent_lib::commands::require_informes(&state, &token)
            .expect_err("Operador no puede consultar informes");
        assert_eq!(err.kind, "permission");
    }

    // Supervisor y Administrador → permitidos (default de roles_con_informes)
    {
        let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let token_sup = sessions.create(2, "supervisor", "Supervisor", "Sup", false);
        let token_admin = sessions.create(3, "admin", "Administrador", "Adm", false);
        drop(sessions);
        assert!(
            dinamo_rent_lib::commands::require_informes(&state, &token_sup).is_ok(),
            "Supervisor tiene rol de informes"
        );
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
