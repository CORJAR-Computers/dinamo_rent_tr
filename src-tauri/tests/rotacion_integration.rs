//! rotacion_integration.rs — Pruebas de integración de la rotación de la clave
//! PII (`services::rotacion::rotar_clave_pii`, el motor del bin
//! `rotate_pii_key` de SECURITY.md §2.1).
//!
//! Se ejecutan sobre una COPIA temporal de la BD de desarrollo
//! (data/dinamo_rent_v3.fdb): la BD real nunca se toca (una rotación re-cifra
//! TODA la tabla clientes). Verifican que la rotación:
//!   - re-cifra los datos PII con la clave nueva (descifrables con ella),
//!   - registra el evento `PII_KEY_ROTATED` en la tabla `auditoria` con
//!     usuario `sistema` e ip `local`,
//!   - NO expone la clave en el mensaje del evento,
//!   - aborta sin escribir nada si la clave vieja no descifra algún token.

use std::path::PathBuf;
use std::sync::Arc;

use rsfbclient::{Execute, Queryable};
use serial_test::serial;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::crypto::PiiCipher;
use dinamo_rent_lib::core::db::{create_pool, PooledConnection};
use dinamo_rent_lib::repositories::cliente::ClienteDatos;
use dinamo_rent_lib::services::cliente::ClienteService;
use dinamo_rent_lib::services::rotacion::rotar_clave_pii;

/// Clave nueva de prueba (base64 de 32 bytes — formato Fernet/válido).
const NUEVA_CLAVE_TEST: &str = "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUE=";

/// Borra el .fdb temporal al salir del scope (panic-safe).
struct LimpiarTemporal(PathBuf);
impl Drop for LimpiarTemporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Copia la BD de desarrollo a un archivo temporal (devuelve ruta + guard).
fn copia_bd_dev() -> (PathBuf, LimpiarTemporal) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("../data/dinamo_rent_v3.fdb");
    assert!(src.exists(), "BD de desarrollo no encontrada: {src:?}");
    let tmp = std::env::temp_dir().join(format!(
        "dinamo_rent_rotacion_{}.fdb",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::copy(&src, &tmp).expect("copiar .fdb a temporal");
    (tmp.clone(), LimpiarTemporal(tmp))
}

/// Config de dev apuntando a la copia temporal.
fn config_con_db(path: &std::path::Path) -> Arc<AppConfig> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let mut cfg = AppConfig::load(&data_dir, &resource_dir, &manifest);
    cfg.db_path = path.to_path_buf();
    Arc::new(cfg)
}

/// Sufijo único por ejecución (evita colisiones entre tests paralelos)
fn uniq() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos().to_string())
        .unwrap_or_else(|_| "x".into())
}

fn total_clientes(conn: &mut PooledConnection) -> i64 {
    conn.query_first("SELECT COUNT(*) FROM clientes", ())
        .expect("contar clientes")
        .map(|(c,)| c)
        .unwrap_or(0)
}

/// PII cruda (cifrada) de un cliente: celular y email.
fn leer_pii(conn: &mut PooledConnection, id: i64) -> (Option<String>, Option<String>) {
    conn.query_first("SELECT celular, email FROM clientes WHERE id = ?", (id,))
        .expect("leer PII cruda")
        .unwrap_or((None, None))
}

/// Último evento PII_KEY_ROTATED: (usuario, accion, mensaje, ip).
/// `CAST(mensaje AS VARCHAR(2000))` porque mensaje es BLOB (mismo patrón que
/// repositories/auditoria.rs).
fn ultimo_pii_rotated(conn: &mut PooledConnection) -> Option<(String, String, String, String)> {
    conn.query_first(
        "SELECT usuario, accion, CAST(mensaje AS VARCHAR(2000)), ip FROM auditoria \
         WHERE accion = 'PII_KEY_ROTATED' ORDER BY id DESC",
        (),
    )
    .expect("consultar auditoria")
}

fn contar_pii_rotated(conn: &mut PooledConnection) -> i64 {
    conn.query_first(
        "SELECT COUNT(*) FROM auditoria WHERE accion = 'PII_KEY_ROTATED'",
        (),
    )
    .expect("contar eventos")
    .map(|(c,)| c)
    .unwrap_or(0)
}

/// La rotación re-cifra TODA la tabla con la clave nueva, registra el evento
/// `PII_KEY_ROTATED` (usuario `sistema`, ip `local`) y NO expone la clave en
/// el mensaje.
#[test]
#[serial]
fn rotacion_registra_pii_key_rotated_sin_exponer_la_clave() {
    let (path, _limpieza) = copia_bd_dev();
    let cfg = config_con_db(&path);
    let old_key = cfg.db_encryption_key.clone();
    assert!(
        !old_key.trim().is_empty(),
        "la BD dev tiene clave PII configurada"
    );
    let pool = create_pool(&cfg).expect("pool copia dev");
    let mut conn = pool.get().expect("conn");

    // Cliente temporal (su PII queda cifrada con la clave vieja al crearse)
    let suf = uniq();
    let cliente = ClienteService::crear(
        &mut conn,
        &cfg,
        &PiiCipher::new(&old_key),
        "test",
        ClienteDatos {
            tipo_doc: Some("Cédula".into()),
            no_doc: Some(format!("ROT{}", &suf[..suf.len().min(8)])),
            nombres: "Cliente".into(),
            apellidos: Some("Rotación".into()),
            celular: Some("3101112233".into()),
            email: Some("rotacion@test.co".into()),
            ciudad: Some("Barranquilla".into()),
            estado: "Activo".into(),
            ..Default::default()
        },
    )
    .expect("crear cliente temporal");
    let id = cliente.cliente.id;

    // Rotar la clave de TODA la copia
    let resultado =
        rotar_clave_pii(&mut conn, &old_key, NUEVA_CLAVE_TEST).expect("la rotación debe funcionar");
    assert_eq!(resultado.clientes as i64, total_clientes(&mut conn));
    assert!(resultado.clientes > 0, "la copia tiene clientes");
    assert!(
        resultado.aes_v1 > 0,
        "la copia tiene tokens v1: re-cifrados (aes_v1={})",
        resultado.aes_v1
    );

    // 1) El evento existe con usuario/ip correctos
    let (usuario, accion, mensaje, ip) =
        ultimo_pii_rotated(&mut conn).expect("debe existir el evento PII_KEY_ROTATED");
    assert_eq!(usuario, "sistema");
    assert_eq!(accion, "PII_KEY_ROTATED");
    assert_eq!(ip, "local");

    // 2) El mensaje NO expone las claves (ni la nueva ni la vieja)
    assert!(
        !mensaje.contains(NUEVA_CLAVE_TEST),
        "el mensaje de auditoría no debe contener la clave nueva: {mensaje}"
    );
    assert!(
        !mensaje.contains(old_key.trim()),
        "el mensaje de auditoría no debe contener la clave vieja: {mensaje}"
    );

    // 3) Los datos re-cifrados descifran con la clave NUEVA y no con la vieja
    let (celular, email) = leer_pii(&mut conn, id);
    let nuevo = PiiCipher::new(NUEVA_CLAVE_TEST);
    let viejo = PiiCipher::new(&old_key);
    assert_eq!(
        nuevo.decrypt(celular.as_deref().unwrap()).unwrap(),
        "3101112233",
        "celular descifra con la clave nueva"
    );
    assert_eq!(
        nuevo.decrypt(email.as_deref().unwrap()).unwrap(),
        "rotacion@test.co",
        "email descifra con la clave nueva"
    );
    assert!(
        viejo.decrypt(celular.as_deref().unwrap()).is_err(),
        "la clave vieja ya NO debe descifrar tras la rotación"
    );

    // Limpieza (la copia se borra sola con el guard)
    conn.execute("DELETE FROM clientes WHERE id = ?", (id,))
        .expect("limpiar cliente temporal");
}

/// Una clave vieja incorrecta debe ABORTAR la rotación sin escribir nada:
/// sin evento de auditoría y con los datos intactos (descifran con la clave
/// real).
#[test]
#[serial]
fn rotacion_aborta_sin_escribir_si_la_clave_vieja_no_descifra() {
    let (path, _limpieza) = copia_bd_dev();
    let cfg = config_con_db(&path);
    let old_key = cfg.db_encryption_key.clone();
    let pool = create_pool(&cfg).expect("pool copia dev");
    let mut conn = pool.get().expect("conn");

    let eventos_antes = contar_pii_rotated(&mut conn);

    let err = rotar_clave_pii(&mut conn, "clave-incorrecta", NUEVA_CLAVE_TEST)
        .expect_err("una clave vieja incorrecta debe fallar");
    let msg = format!("{err}");
    assert!(
        msg.contains("clave"),
        "el error debe señalar la clave: {msg}"
    );

    // Nada se escribió: no hay evento nuevo…
    assert_eq!(
        contar_pii_rotated(&mut conn),
        eventos_antes,
        "la rotación fallida no debe registrar eventos"
    );
    // …y los PII siguen descifrando con la clave real.
    let rows: Vec<(Option<String>,)> = conn
        .query("SELECT celular FROM clientes WHERE celular IS NOT NULL", ())
        .expect("leer celulares");
    assert!(!rows.is_empty(), "la copia tiene clientes con celular");
    let cipher = PiiCipher::new(&old_key);
    let descifrados = rows
        .iter()
        .filter(|(c,)| {
            c.as_deref()
                .map(|v| cipher.decrypt(v).is_ok())
                .unwrap_or(false)
        })
        .count();
    assert!(
        descifrados == rows.len(),
        "todos los celulares deben seguir descifrando con la clave real ({descifrados}/{}). El aborto debe ser limpio",
        rows.len()
    );
}
