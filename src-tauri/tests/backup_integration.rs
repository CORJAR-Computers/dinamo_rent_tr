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

use std::path::PathBuf;
use std::sync::Arc;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::services::backup::{crear_backup, listar_backups};

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
    std::fs::copy(&src, &db).unwrap();

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
