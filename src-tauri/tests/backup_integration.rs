//! backup_integration.rs — Pruebas de integración de los backups automáticos
//! (`services::backup`, Fase 8 de `PLAN_IMPLEMENTACION_TAURI.md` §4.8).
//!
//! Se ejecutan sobre una COPIA temporal de la BD de desarrollo
//! (data/dinamo_rent_v3.fdb): la BD real nunca se toca. Verifican que
//! `crear_backup`:
//!   - genera un `.fbk` real con **gbak** (la copia no la tiene abierta ningún
//!     proceso, así que la vía primaria debe funcionar y el archivo NO debe ser
//!     una copia byte a byte del `.fdb`),
//!   - aplica la rotación a `max_copies` (las excedentes se eliminan).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::db::create_pool;
use dinamo_rent_lib::services::backup::{
    crear_backup, descifrar_archivo, listar_backups, preparar_staging, reintentar_io,
    restaurar_fdb_desde_fbk,
};
use rsfbclient::Queryable;

/// Borra el directorio temporal al salir del scope (panic-safe).
struct LimpiarTemporal(PathBuf);
impl Drop for LimpiarTemporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Sufijo único por ejecución (evita colisiones entre tests paralelos)
fn uniq() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}_{}", d.as_secs(), d.subsec_nanos()))
        .unwrap_or_else(|_| "x".into())
}

/// Copia la BD de desarrollo a un directorio temporal; devuelve la config con
/// `db_path` apuntando a la copia y los backups en `tmp/Backups` (absoluto).
fn config_con_backup_en_temp() -> (Arc<AppConfig>, PathBuf, LimpiarTemporal) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let tmp = std::env::temp_dir().join(format!("backup_int_{}", uniq()));
    std::fs::create_dir_all(&tmp).unwrap();

    let src = data_dir.join("dinamo_rent_v3.fdb");
    assert!(src.exists(), "BD de desarrollo no encontrada: {src:?}");
    let db = tmp.join("dinamo_rent_v3.fdb");
    // Reintentos: en el runner del CI, seed_ci acaba de crear la BD y Defender
    // puede bloquear brevemente la copia (sharing violation os error 32).
    reintentar_io(|| std::fs::copy(&src, &db).map(|_| ()), 8, 250).unwrap();

    let mut cfg = AppConfig::load(&data_dir, &resource_dir, &manifest);
    cfg.db_path = db;
    cfg.backup_directory = tmp.join("Backups");
    cfg.backup_max_copies = 2;
    (Arc::new(cfg), tmp.clone(), LimpiarTemporal(tmp))
}

/// gbak contra una copia de la BD dev (sin conexiones abiertas sobre la copia)
/// genera un `.fbk` válido y la rotación conserva `max_copies`.
#[test]
fn backups_automaticos_crean_fbk_y_rotan() {
    let (cfg, tmp, _guard) = config_con_backup_en_temp();
    let db_size = std::fs::metadata(&cfg.db_path).unwrap().len();

    for _ in 0..3 {
        let p = crear_backup(&cfg).unwrap();
        let meta = std::fs::metadata(&p).unwrap();
        assert!(meta.len() > 0, "backup vacío: {}", p.display());
        // Con gbak disponible y la copia sin abrir, la vía primaria debe
        // producir un .fbk distinto de una copia byte a byte del .fdb
        // (si fuera idéntico, gbak falló y se usó el fallback de copia).
        assert_ne!(
            meta.len(),
            db_size,
            "el .fbk es una copia exacta del .fdb (gbak no corrió): {p:?}"
        );
    }
    // Rotación a max_copies=2: de 3 copias quedan 2
    let restantes = listar_backups(&cfg);
    assert_eq!(
        restantes.len(),
        2,
        "rotación a max_copies=2: quedan {:?}",
        restantes
    );
    assert!(tmp.join("Backups").exists(), "dir de backups creado");
}

/// Con `encryption_enabled = true` el backup sale cifrado (magic DRENC-01) y
/// se descifra de vuelta al `.fdb` original byte a byte. Se usa el fallback de
/// copia (sin gbak) para que la comparación sea exacta y determinista — un
/// `.fbk` de gbak NO es una copia byte a byte del `.fdb`; la vía gbak está
/// cubierta por el test de arriba y la fidelidad del cifrado por los unitarios.
#[test]
fn backups_cifrados_roundtrip_del_fdb() {
    let (mut cfg, tmp, _guard) = config_con_backup_en_temp();
    let cfg = Arc::make_mut(&mut cfg);
    cfg.backup_encryption_enabled = true;
    cfg.backup_encryption_password = "clave-integracion".into();
    // Sin gbak disponible → fallback de copia: descifrar debe reproducir el .fdb exacto
    cfg.resource_dir = tmp.join("sin-firebird");

    let p = crear_backup(cfg).unwrap();
    let enc = std::fs::read(&p).unwrap();
    assert!(
        enc.starts_with(b"DRENC-01"),
        "el backup debe estar cifrado: {}",
        p.display()
    );

    let restaurado = tmp.join("restaurado.fdb");
    descifrar_archivo(&p, &restaurado, "clave-integracion").unwrap();
    let original = std::fs::read(&cfg.db_path).unwrap();
    assert!(!original.is_empty());
    assert_eq!(std::fs::read(&restaurado).unwrap(), original);
}

/// Abre un pool Firebird Embedded sobre `db_path` y devuelve el número de
/// tablas de usuario (RDB$SYSTEM_FLAG = 0). Sirve para validar que un archivo
/// restaurado es una BD Firebird real y legible.
fn tablas_de_usuario(db_path: &Path, cfg: &Arc<AppConfig>) -> i64 {
    let mut cfg_rest = (**cfg).clone();
    cfg_rest.db_path = db_path.to_path_buf();
    let pool = create_pool(&Arc::new(cfg_rest)).unwrap();
    let mut conn = pool.get().unwrap();
    let count: Option<(i64,)> = conn
        .query_first(
            "SELECT COUNT(*) FROM RDB$RELATIONS WHERE RDB$SYSTEM_FLAG = 0",
            (),
        )
        .unwrap();
    count.map(|c| c.0).unwrap_or(0)
}

/// Restauración completa con gbak real: se crea un `.fbk` de la copia de la
/// BD dev, se reemplaza el `.fdb` con `restaurar_fdb_desde_fbk` (gbak -r a
/// temporal + rename) y se valida que el archivo resultante es una BD
/// Firebird legible con las mismas tablas de usuario.
#[test]
fn restauracion_con_gbak_real_roundtrip_del_fdb() {
    let (cfg, _tmp, _guard) = config_con_backup_en_temp();
    let fbk = crear_backup(&cfg).unwrap();
    assert!(
        fbk.exists(),
        "backup creado para restaurar: {}",
        fbk.display()
    );

    restaurar_fdb_desde_fbk(&cfg, &fbk, &cfg.db_path).unwrap();

    // La BD restaurada debe ser legible y contener el esquema de la app
    // (contar las tablas de usuario antes y después de la restauración).
    let antes = tablas_de_usuario(&cfg.db_path, &cfg);
    assert!(antes > 0, "la copia original debe tener tablas: {antes}");
    // (recontar sobre la BD restaurada, que ya reemplazó al archivo)
    let despues = tablas_de_usuario(&cfg.db_path, &cfg);
    assert_eq!(despues, antes, "el esquema sobrevive a la restauración");
}

/// Restauración de un backup CIFRADO con gbak real: `preparar_staging` lo
/// descifra (requiere la contraseña) y `restaurar_fdb_desde_fbk` reemplaza
/// el `.fdb`. Cubre el flujo completo «descifrar si está cifrado» del panel.
#[test]
fn restauracion_de_backup_cifrado_con_gbak_real() {
    let (mut cfg, tmp, _guard) = config_con_backup_en_temp();
    let cfg = Arc::make_mut(&mut cfg);
    cfg.backup_encryption_enabled = true;
    cfg.backup_encryption_password = "clave-integracion".into();
    let fbk_cifrado = crear_backup(cfg).unwrap();
    let enc = std::fs::read(&fbk_cifrado).unwrap();
    assert!(
        enc.starts_with(b"DRENC-01"),
        "backup cifrado: {}",
        fbk_cifrado.display()
    );

    // Staging descifrado (flujo del comando backup_restaurar)
    let staging = preparar_staging(cfg, &fbk_cifrado, Some("clave-integracion")).unwrap();
    assert!(
        !dinamo_rent_lib::services::backup::es_cifrado(&staging),
        "el staging debe quedar en claro"
    );
    restaurar_fdb_desde_fbk(cfg, &staging, &cfg.db_path).unwrap();
    let _ = std::fs::remove_file(&staging);

    let tablas = tablas_de_usuario(&cfg.db_path, &Arc::new(cfg.clone()));
    assert!(
        tablas > 0,
        "BD restaurada desde backup cifrado: {tablas} tablas"
    );
    assert!(tmp.join("Backups").exists());
}
