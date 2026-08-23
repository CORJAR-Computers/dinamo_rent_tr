//! reservas_integration.rs — Pruebas de integración del servicio de reservas
//! contra el .fdb de desarrollo (data/dinamo_rent_v3.fdb).
//!
//! Usa un cliente y un auto reales de la BD (solo lectura) y crea/elimina
//! reservas temporales en cada test.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{Duration, Local};
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::rbac::SessionStore;
use dinamo_rent_lib::core::security::LoginAttemptTracker;
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::repositories::cliente::ClienteRepository;
use dinamo_rent_lib::repositories::reserva::ReservaDatos;
use dinamo_rent_lib::services::reserva::ReservaService;
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

/// Cliente real de la BD de dev (lectura)
fn cliente_real(state: &AppState) -> (i64, String) {
    let mut conn = state.pool.get().expect("conn");
    let clientes = ClienteRepository::obtener_todos(&mut conn).expect("clientes");
    let c = clientes.first().expect("la BD de dev tiene clientes");
    (c.id, c.nombre_completo.clone())
}

/// Auto real de la BD de dev (lectura)
fn auto_real(state: &AppState) -> (String, String) {
    let mut conn = state.pool.get().expect("conn");
    let autos = AutoRepository::obtener_todos(&mut conn).expect("autos");
    let a = autos.first().expect("la BD de dev tiene autos");
    (a.placa.clone(), a.tipo.clone())
}

fn datos_reserva(nombre: &str, dias: i64) -> ReservaDatos {
    let hoy = Local::now().date_naive();
    ReservaDatos {
        id_cliente: None,
        nombre_cliente: nombre.into(),
        nacionalidad: Some("Colombiana".into()),
        categoria_vehiculo: Some("Automóvil".into()),
        placa_asignada: None,
        fecha_recogida: (hoy + Duration::days(7)).format("%Y-%m-%d").to_string(),
        hora_recogida: Some("09:00".into()),
        ubicacion_recogida: Some("Oficina principal".into()),
        fecha_retorno: (hoy + Duration::days(7 + dias)).format("%Y-%m-%d").to_string(),
        hora_retorno: Some("18:00".into()),
        ubicacion_retorno: Some("Oficina principal".into()),
        dias_calculados: dias,
        horas_extras: 2,
        valor_dia: "150000".into(),
        valor_hora_adic: "20000".into(),
        abono: "100000".into(),
        total: "0".into(),
        observaciones: Some("Reserva de prueba".into()),
        estado: "Confirmada".into(),
    }
}

#[test]
#[serial]
fn reserva_crud_roundtrip() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");
    let (id_cliente, nombre_cliente) = cliente_real(&state);
    let (placa, categoria) = auto_real(&state);

    let mut datos = datos_reserva(&nombre_cliente, 3);
    datos.id_cliente = Some(id_cliente);
    datos.categoria_vehiculo = Some(categoria.clone());
    datos.placa_asignada = Some(placa.clone());
    // El servicio autocompleta el nombre desde la tabla clientes
    datos.nombre_cliente = "NOMBRE EQUIVOCADO".into();

    // Crear
    let creada = ReservaService::crear(&mut conn, cfg, datos.clone()).expect("crear reserva");
    let id = creada.id;
    assert_eq!(creada.nombre_cliente, nombre_cliente, "nombre autocompletado del cliente");
    assert_eq!(creada.dias_calculados, 3);
    // total = 3 × 150000 + 2 × 20000 = 490000
    assert_eq!(creada.total, "490000.00", "total recalculado por el backend");
    assert_eq!(creada.placa_asignada.as_deref(), Some(placa.as_str()));
    assert_eq!(creada.estado, "Confirmada");

    // Obtener
    let obtenida = ReservaService::obtener(&mut conn, id).expect("obtener reserva");
    assert_eq!(obtenida.fecha_recogida, datos.fecha_recogida);
    assert_eq!(obtenida.hora_recogida.as_deref(), Some("09:00"));

    // Actualizar (abono > total → debe fallar; primero uno válido)
    datos.abono = "200000".into();
    let actualizada = ReservaService::actualizar(&mut conn, cfg, id, datos.clone())
        .expect("actualizar reserva");
    assert_eq!(actualizada.abono, "200000.00");

    // Eliminar
    ReservaService::eliminar(&mut conn, "test", id).expect("eliminar reserva");
    assert!(ReservaService::obtener(&mut conn, id).is_err(), "reserva eliminada");
}

#[test]
#[serial]
fn reserva_validaciones() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    // Fechas invertidas → validation
    let mut inv = datos_reserva("Cliente Inválido", 1);
    inv.fecha_retorno = "2020-01-01".into();
    let err = ReservaService::crear(&mut conn, cfg, inv).expect_err("fechas invertidas");
    assert_eq!(err.kind(), "validation");

    // Abono mayor que el total → validation
    let mut abono = datos_reserva("Cliente Abono", 1);
    abono.abono = "99999999".into();
    let err = ReservaService::crear(&mut conn, cfg, abono).expect_err("abono > total");
    assert_eq!(err.kind(), "validation");

    // Estado inválido → validation
    let mut est = datos_reserva("Cliente Estado", 1);
    est.estado = "Inventada".into();
    let err = ReservaService::crear(&mut conn, cfg, est).expect_err("estado inválido");
    assert_eq!(err.kind(), "validation");

    // Hora inválida → validation
    let mut hora = datos_reserva("Cliente Hora", 1);
    hora.hora_recogida = Some("25:99".into());
    let err = ReservaService::crear(&mut conn, cfg, hora).expect_err("hora inválida");
    assert_eq!(err.kind(), "validation");

    // id_cliente inexistente → Business (FK amigable)
    let mut fk = datos_reserva("Cliente FK", 1);
    fk.id_cliente = Some(999_999_999);
    let err = ReservaService::crear(&mut conn, cfg, fk).expect_err("cliente inexistente");
    assert_eq!(err.kind(), "business");
}

#[test]
#[serial]
fn reserva_cancelar() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    let creada =
        ReservaService::crear(&mut conn, cfg, datos_reserva("Cliente Cancelar", 2))
            .expect("crear reserva");
    let id = creada.id;

    // Cancelar → Cancelada
    let r = ReservaService::cancelar(&mut conn, id).expect("cancelar");
    assert!(r.cancelada);
    assert_eq!(r.reserva.estado, "Cancelada");

    // Cancelar de nuevo → no-op
    let r2 = ReservaService::cancelar(&mut conn, id).expect("cancelar otra vez");
    assert!(!r2.cancelada);

    // Completada no se puede cancelar
    let mut datos = datos_reserva("Cliente Completar", 1);
    datos.estado = "Completada".into();
    let completada = ReservaService::crear(&mut conn, cfg, datos).expect("crear completada");
    let err = ReservaService::cancelar(&mut conn, completada.id).expect_err("no cancelar completada");
    assert_eq!(err.kind(), "business");

    // Limpieza
    ReservaService::eliminar(&mut conn, "test", id).expect("eliminar cancelada");
    ReservaService::eliminar(&mut conn, "test", completada.id).expect("eliminar completada");
}

#[test]
#[serial]
fn reserva_proximas_y_contar() {
    let state = dev_state();
    let cfg = &state.config;
    let mut conn = state.pool.get().expect("conn");

    // Fecha futura → aparece en próximas
    let creada =
        ReservaService::crear(&mut conn, cfg, datos_reserva("Cliente Proximas", 1))
            .expect("crear reserva");
    let id = creada.id;

    let proximas = ReservaService::proximas(&mut conn, 50).expect("proximas");
    assert!(
        proximas.iter().any(|r| r.id == id),
        "la reserva futura aparece en próximas"
    );

    let total = ReservaService::contar(&mut conn).expect("contar");
    assert!(total >= 1);

    let por_estado = ReservaService::contar_por_estado(&mut conn).expect("por estado");
    let suma: i64 = por_estado.iter().map(|(_, c)| c).sum();
    assert_eq!(suma, total);

    // Limpieza
    ReservaService::eliminar(&mut conn, "test", id).expect("eliminar");
}
