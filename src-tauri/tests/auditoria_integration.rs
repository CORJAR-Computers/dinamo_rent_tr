//! auditoria_integration.rs — Pruebas de integración del servicio de auditoría
//! contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Los tests insertan eventos temporales de auditoría y los eliminan al final
//! para no ensuciar el log real.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serial_test::serial;

use dinamo_rent_lib::core::audit::log_audit;
use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use rsfbclient::{Execute, Queryable};

use dinamo_rent_lib::repositories::auditoria::AuditoriaFiltros;
use dinamo_rent_lib::services::auditoria::AuditoriaService;
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

/// Inserta un evento temporal y devuelve su id
fn insertar_evento(state: &AppState, usuario: &str, accion: &str, mensaje: &str) -> i64 {
    let mut conn = state.pool.get().expect("conn");
    log_audit(&mut conn, usuario, accion, mensaje, "127.0.0.1").expect("log_audit");
    // id del evento recién insertado (último)
    let row: Option<(i64,)> = conn.query_first("SELECT MAX(id) FROM auditoria", ()).expect("max id");
    row.map(|(id,)| id).unwrap_or(0)
}

fn eliminar_eventos(state: &AppState, ids: &[i64]) {
    if ids.is_empty() {
        return;
    }
    let mut conn = state.pool.get().expect("conn");
    for id in ids {
        let _ = conn.execute("DELETE FROM auditoria WHERE id = ?", (*id,));
    }
}

#[test]
#[serial]
fn auditoria_listar_y_filtrar() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    // Insertar eventos temporales con marca clara
    let id1 = insertar_evento(&state, "testaudit", "LOGIN OK", "usuario=testaudit");
    let id2 = insertar_evento(&state, "testaudit", "LOGIN FALLIDO", "usuario=testaudit, intentos=1");
    let id3 = insertar_evento(&state, "otrouser", "USUARIO CREADO", "username=prueba");

    // Sin filtros → los incluye (paginación grande)
    let r = AuditoriaService::listar(&mut conn, AuditoriaFiltros::default(), Some(1), Some(500))
        .expect("listar");
    assert!(r.total >= 3);
    assert!(
        r.eventos.iter().any(|e| e.id == id1 || e.id == id2 || e.id == id3),
        "los eventos temporales aparecen en la lista"
    );

    // Filtro por usuario exacto
    let f = AuditoriaFiltros {
        usuario: Some("testaudit".into()),
        ..Default::default()
    };
    let r = AuditoriaService::listar(&mut conn, f, Some(1), Some(100)).expect("filtro usuario");
    assert!(r.eventos.iter().any(|e| e.id == id1 || e.id == id2));
    assert!(
        r.eventos.iter().all(|e| e.id != id3),
        "el filtro por usuario excluye otros usuarios"
    );

    // Filtro por acción exacta
    let f = AuditoriaFiltros {
        accion: Some("LOGIN FALLIDO".into()),
        ..Default::default()
    };
    let r = AuditoriaService::listar(&mut conn, f, Some(1), Some(100)).expect("filtro accion");
    assert!(r.eventos.iter().any(|e| e.id == id2));
    assert!(
        r.eventos.iter().all(|e| e.accion == "LOGIN FALLIDO"),
        "todas las acciones coinciden"
    );

    // Búsqueda libre
    let f = AuditoriaFiltros {
        busqueda: Some("prueba".into()),
        ..Default::default()
    };
    let r = AuditoriaService::listar(&mut conn, f, Some(1), Some(100)).expect("busqueda");
    assert!(r.eventos.iter().any(|e| e.id == id3));

    // Paginación: primera página de 2 → no puede contener a la vez los 3 si total > 2
    let r = AuditoriaService::listar(
        &mut conn,
        AuditoriaFiltros {
            usuario: Some("testaudit".into()),
            ..Default::default()
        },
        Some(1),
        Some(1),
    )
    .expect("paginación");
    assert!(r.eventos.len() <= 1);
    assert_eq!(r.pagina, 1);
    assert!(r.total >= 2);

    // Limpieza
    eliminar_eventos(&state, &[id1, id2, id3]);
}

#[test]
#[serial]
fn auditoria_filtro_fechas_invalidas() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    // Fecha inválida → validation
    let f = AuditoriaFiltros {
        fecha_desde: Some("no-es-fecha".into()),
        ..Default::default()
    };
    let err = AuditoriaService::listar(&mut conn, f, Some(1), Some(50)).expect_err("fecha inválida");
    assert_eq!(err.kind(), "validation");

    // Desde > Hasta → validation
    let f = AuditoriaFiltros {
        fecha_desde: Some("2026-12-31".into()),
        fecha_hasta: Some("2026-01-01".into()),
        ..Default::default()
    };
    let err = AuditoriaService::listar(&mut conn, f, Some(1), Some(50)).expect_err("rango invertido");
    assert_eq!(err.kind(), "validation");

    // Rango válido no falla (aunque devuelva 0 del pasado remoto)
    let f = AuditoriaFiltros {
        fecha_desde: Some("2000-01-01".into()),
        fecha_hasta: Some("2000-01-02".into()),
        ..Default::default()
    };
    let r = AuditoriaService::listar(&mut conn, f, Some(1), Some(50)).expect("rango válido");
    assert_eq!(r.eventos.len(), 0);
}

#[test]
#[serial]
fn auditoria_acciones_y_usuarios() {
    let state = dev_state();
    let mut conn = state.pool.get().expect("conn");

    let id1 = insertar_evento(&state, "testacciones", "LOGIN OK", "x");
    let id2 = insertar_evento(&state, "testacciones2", "LOGIN OK", "x");
    // LOGIN FALLIDO se inserta aquí (no se asume historia previa en la BD):
    // así el test es autosuficiente y pasa también contra una BD fresca
    // sembrada por seed_ci en CI.
    let id3 = insertar_evento(&state, "testacciones", "LOGIN FALLIDO", "x");

    let acciones = AuditoriaService::acciones(&mut conn).expect("acciones");
    assert!(acciones.contains(&"LOGIN OK".to_string()));
    assert!(acciones.contains(&"LOGIN FALLIDO".to_string()));

    let usuarios = AuditoriaService::usuarios(&mut conn).expect("usuarios");
    assert!(usuarios.contains(&"testacciones".to_string()));
    assert!(usuarios.contains(&"testacciones2".to_string()));

    eliminar_eventos(&state, &[id1, id2, id3]);
}
