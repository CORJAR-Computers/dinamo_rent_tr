//! services/backup.rs — Backups automáticos de la base de datos (Fase 8 de
//! `PLAN_IMPLEMENTACION_TAURI.md` §4.8/§2.6, puerto de `services/backup_service.py`).
//!
//! Copia de seguridad del `.fdb` en los horarios de `[backup] schedule_times`
//! de `config.ini` (default `09:00, 13:00, 19:00, 23:00`) con rotación a
//! `max_copies` copias (default 10). Corre en un hilo de fondo mientras la
//! app está abierta, igual que el Agente SIMIT.
//!
//! # Estrategia de backup (doble, según el plan)
//! 1. **`gbak`** (`resources/firebird/gbak.exe`, mismo kit que `fbclient.dll`):
//!    backup nativo Firebird, consistente. Es la vía primaria.
//! 2. **Fallback a copia del `.fdb`** (`fs::copy`): el plan (§4.8 y la tabla de
//!    riesgos) prevé el fallback cuando `gbak` no está disponible o está
//!    bloqueado. En producción con la app corriendo, el motor Embedded abre el
//!    `.fdb` en exclusiva por proceso, así que `gbak` (proceso aparte) suele
//!    fallar con el archivo en uso y **el fallback es el camino operativo** —
//!    mismo tradeoff que documenta `DEPLOYMENT_CLIENTES.md` §4.1 para la copia
//!    manual en caliente. Los backups programados solo son consistentes si se
//!    restauran con la app detenida.
//!
//! # Rotación
//! Tras cada backup se conservan las `max_copies` más recientes
//! (`Backup_Dinamo_<YYYYMMDD_HHMMSS>.fbk`; el timestamp del nombre ordena
//! cronológicamente) y se eliminan las excedentes. `max_copies = 0` = conservar
//! todas (rotación desactivada).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, Timelike};

use crate::core::config::AppConfig;
use crate::core::error::AppError;

/// Prefijo de los archivos de backup (coincide con el plan: `Backup_Dinamo_<ts>.fbk`)
const PREFIJO_BACKUP: &str = "Backup_Dinamo_";

/// Nombre de archivo de un backup: `Backup_Dinamo_<YYYYMMDD_HHMMSS>.fbk`
fn nombre_backup(ahora: &DateTime<Local>) -> String {
    format!("{PREFIJO_BACKUP}{}.fbk", ahora.format("%Y%m%d_%H%M%S"))
}

/// Directorio de backups: `backup.directory` (absoluto) o `data_dir/<dir>`
pub fn dir_backups(cfg: &AppConfig) -> PathBuf {
    let dir = PathBuf::from(&cfg.backup_directory);
    if dir.is_absolute() {
        dir
    } else {
        cfg.data_dir.join(dir)
    }
}

/// gbak.exe del kit Firebird empaquetado (dev: `resources/firebird`; prod:
/// el bundle extrae los recursos en `resource_dir`).
fn encontrar_gbak(cfg: &AppConfig) -> PathBuf {
    cfg.resource_dir.join("firebird").join("gbak.exe")
}

/// Lista los backups existentes (`Backup_Dinamo_*.fbk`), ordenados del más
/// viejo al más nuevo (el timestamp del nombre es cronológico).
pub fn listar_backups(cfg: &AppConfig) -> Vec<PathBuf> {
    let dir = dir_backups(cfg);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut archivos: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let nombre = p.file_name().map(|n| n.to_string_lossy().into_owned());
            nombre
                .as_deref()
                .is_some_and(|n| n.starts_with(PREFIJO_BACKUP) && n.ends_with(".fbk"))
        })
        .collect();
    archivos.sort();
    archivos
}

/// Rotación: conserva las `max_copies` más recientes y elimina las excedentes.
/// Devuelve cuántas copias se borraron. `max_copies = 0` = conservar todas.
pub fn rotar(cfg: &AppConfig) -> Result<usize, AppError> {
    let max = cfg.backup_max_copies;
    if max == 0 {
        return Ok(0);
    }
    let archivos = listar_backups(cfg);
    let excedentes = archivos.len().saturating_sub(max);
    for p in archivos.iter().take(excedentes) {
        if let Err(e) = std::fs::remove_file(p) {
            log::warn!("Backup: no se pudo eliminar la copia vieja {}: {e}", p.display());
        }
    }
    Ok(excedentes)
}

/// Backup vía `gbak` (nativo Firebird, consistente). Falla con `AppError`
/// detallado si gbak no está, la BD no existe o el proceso termina mal.
fn crear_con_gbak(cfg: &AppConfig, destino: &Path) -> Result<(), AppError> {
    let gbak = encontrar_gbak(cfg);
    if !gbak.exists() {
        return Err(AppError::Generic(format!(
            "gbak.exe no encontrado en {}",
            gbak.display()
        )));
    }
    if !cfg.db_path.exists() {
        return Err(AppError::Generic(format!(
            "BD no encontrada: {}",
            cfg.db_path.display()
        )));
    }
    // current_dir = carpeta de Firebird: gbak resuelve fbclient.dll/firebird.msg
    // desde su propio directorio (busca ahí primero), sin depender del PATH.
    let firebird_dir = gbak.parent().unwrap_or(Path::new("."));
    let salida = Command::new(&gbak)
        .current_dir(firebird_dir)
        .args(["-b", "-user", &cfg.db_user, "-password", &cfg.db_password, "-v"])
        .arg(&cfg.db_path)
        .arg(destino)
        .output()
        .map_err(|e| AppError::Generic(format!("no se pudo ejecutar gbak: {e}")))?;
    if !salida.status.success() {
        let stdout = String::from_utf8_lossy(&salida.stdout);
        let stderr = String::from_utf8_lossy(&salida.stderr);
        return Err(AppError::Generic(format!(
            "gbak terminó con {}: {} {}",
            salida.status,
            stdout.trim(),
            stderr.trim()
        )));
    }
    let vacio = std::fs::metadata(destino)
        .map(|m| m.len() == 0)
        .unwrap_or(true);
    if !destino.exists() || vacio {
        return Err(AppError::Generic(
            "gbak terminó OK pero no dejó el archivo .fbk".into(),
        ));
    }
    Ok(())
}

/// Fallback: copia directa del `.fdb` (la vía operativa con la app corriendo,
/// pues el motor Embedded abre la BD en exclusiva por proceso).
fn copiar_fdb(cfg: &AppConfig, destino: &Path) -> Result<(), AppError> {
    if !cfg.db_path.exists() {
        return Err(AppError::Generic(format!(
            "BD no encontrada: {}",
            cfg.db_path.display()
        )));
    }
    std::fs::copy(&cfg.db_path, destino)?;
    Ok(())
}

/// Crea un backup (gbak con fallback a copia del `.fdb`) y aplica la rotación.
/// Devuelve la ruta del backup creado. Lo usan el scheduler automático y
/// (a futuro) el comando manual de backups.
pub fn crear_backup(cfg: &AppConfig) -> Result<PathBuf, AppError> {
    let dir = dir_backups(cfg);
    std::fs::create_dir_all(&dir)?;
    let mut destino = dir.join(nombre_backup(&Local::now()));
    // Colisión en el mismo segundo (p. ej. backup manual + automático): NO
    // sobrescribir el archivo existente — numerar el nombre (_2, _3, …).
    let mut sufijo = 2u32;
    while destino.exists() {
        destino = dir.join(format!(
            "{PREFIJO_BACKUP}{}_{sufijo}.fbk",
            Local::now().format("%Y%m%d_%H%M%S")
        ));
        sufijo += 1;
    }
    match crear_con_gbak(cfg, &destino) {
        Ok(()) => log::info!("Backup: gbak OK → {}", destino.display()),
        Err(e) => {
            log::warn!(
                "Backup: gbak falló ({e}) — fallback a copia del .fdb"
            );
            copiar_fdb(cfg, &destino)?;
        }
    }
    let borrados = rotar(cfg)?;
    if borrados > 0 {
        log::info!("Backup: rotación eliminó {borrados} copia(s) vieja(s)");
    }
    Ok(destino)
}

/// Decide si el scheduler debe ejecutar un backup en `ahora`:
/// - el minuto actual está en `schedule_minutes`, y
/// - ese (fecha, minuto) aún no se ejecutó (marca `ultimo`).
/// Evita duplicar la corrida si `check_interval_ms` < 60 s y permite volver a
/// ejecutar el horario al día siguiente.
fn debe_ejecutar(
    ahora: NaiveDateTime,
    schedule_minutes: &[u32],
    ultimo: Option<(NaiveDate, u32)>,
) -> bool {
    let minuto = ahora.hour() * 60 + ahora.minute();
    if !schedule_minutes.contains(&minuto) {
        return false;
    }
    ultimo != Some((ahora.date(), minuto))
}

/// Lanza el hilo de fondo de backups programados. Cada `check_interval_ms`
/// (default 60 s) compara la hora local contra `schedule_times`; cuando el
/// minuto coincide se crea el backup (una sola vez por minuto y por día) y se
/// aplica la rotación. Los errores se loguean y el ciclo continúa (reintento
/// en el siguiente horario).
///
/// Con `schedule_times` vacío no hay horarios y el hilo queda en espera
/// (backups automáticos desactivados).
pub fn spawn_scheduler(cfg: Arc<AppConfig>) {
    // Clone para el hilo: la config original se usa para el log de arranque.
    let cfg_hilo = cfg.clone();
    std::thread::spawn(move || {
        // Marca de la última corrida: (fecha, minuto del día). El guard lo
        // mantiene este hilo (único escritor); no se comparte con la UI.
        let mut ultimo: Option<(NaiveDate, u32)> = None;
        loop {
            if !cfg_hilo.backup_schedule_minutes.is_empty() {
                let ahora = Local::now();
                if debe_ejecutar(
                    ahora.naive_local(),
                    &cfg_hilo.backup_schedule_minutes,
                    ultimo,
                ) {
                    match crear_backup(&cfg_hilo) {
                        Ok(path) => {
                            ultimo = Some((ahora.date_naive(), ahora.hour() * 60 + ahora.minute()));
                            log::info!(
                                "Backup automático completado: {} ({} copia(s) en {})",
                                path.display(),
                                listar_backups(&cfg_hilo).len(),
                                dir_backups(&cfg_hilo).display()
                            );
                        }
                        Err(e) => {
                            log::error!("Backup automático falló: {e}");
                            // No se marca ultimo → se reintenta en el siguiente
                            // tick (si el minuto sigue siendo horario).
                        }
                    }
                }
            }
            let intervalo = cfg_hilo.backup_check_interval_ms.max(1000);
            std::thread::sleep(Duration::from_millis(intervalo));
        }
    });
    log::info!(
        "Backups automáticos activos: horarios {:?}, {} copia(s), cada {} ms",
        cfg.backup_schedule_times,
        cfg.backup_max_copies,
        cfg.backup_check_interval_ms
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn uniq() -> String {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("{}_{}", d.as_secs(), d.subsec_nanos()))
            .unwrap_or_else(|_| "x".into())
    }

    /// Config con data_dir temporal y sin firebird (gbak nunca se intenta:
    /// el fallback de copia es determinista y rápido en los tests).
    fn config_prueba() -> AppConfig {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let tmp = std::env::temp_dir().join(format!("backup_test_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = AppConfig::load(&tmp, &manifest.join("resources"), &manifest);
        cfg.resource_dir = tmp.join("sin-firebird");
        cfg
    }

    #[test]
    fn nombre_backup_formato() {
        let ahora = Local::now();
        let nombre = nombre_backup(&ahora);
        assert!(nombre.starts_with(PREFIJO_BACKUP), "{nombre}");
        assert!(nombre.ends_with(".fbk"), "{nombre}");
        // Timestamp YYYYMMDD_HHMMSS (8+1+6 = 15 caracteres) entre prefijo y extensión
        let ts = &nombre[PREFIJO_BACKUP.len()..nombre.len() - 4];
        assert_eq!(ts.len(), 15, "{nombre}");
        assert!(ts.chars().all(|c| c.is_ascii_digit() || c == '_'), "{nombre}");
    }

    #[test]
    fn rotacion_conserva_las_mas_recientes() {
        let tmp = std::env::temp_dir().join(format!("backup_rot_{}", uniq()));
        let dir = tmp.join("Backups");
        fs::create_dir_all(&dir).unwrap();
        let mut cfg = config_prueba();
        cfg.backup_directory = PathBuf::from("Backups");
        cfg.data_dir = tmp.clone();
        cfg.backup_max_copies = 3;
        // 5 copias con timestamps crecientes (el orden del nombre es cronológico)
        for i in 1..=5 {
            fs::write(
                dir.join(format!("{PREFIJO_BACKUP}20260817_0{i}0000.fbk")),
                b"x",
            )
            .unwrap();
        }
        let borrados = rotar(&cfg).unwrap();
        assert_eq!(borrados, 2);
        let restantes = listar_backups(&cfg);
        assert_eq!(restantes.len(), 3);
        assert!(!dir.join(format!("{PREFIJO_BACKUP}20260817_010000.fbk")).exists());
        assert!(!dir.join(format!("{PREFIJO_BACKUP}20260817_020000.fbk")).exists());
        assert!(dir.join(format!("{PREFIJO_BACKUP}20260817_030000.fbk")).exists());
        assert!(dir.join(format!("{PREFIJO_BACKUP}20260817_050000.fbk")).exists());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn rotacion_cero_conserva_todas() {
        let tmp = std::env::temp_dir().join(format!("backup_rot0_{}", uniq()));
        let dir = tmp.join("Backups");
        fs::create_dir_all(&dir).unwrap();
        let mut cfg = config_prueba();
        cfg.backup_directory = PathBuf::from("Backups");
        cfg.data_dir = tmp.clone();
        cfg.backup_max_copies = 0;
        for i in 1..=3 {
            fs::write(
                dir.join(format!("{PREFIJO_BACKUP}20260817_0{i}0000.fbk")),
                b"x",
            )
            .unwrap();
        }
        assert_eq!(rotar(&cfg).unwrap(), 0);
        assert_eq!(listar_backups(&cfg).len(), 3);
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn crear_backup_con_copia_fallback_y_rotacion() {
        let tmp = std::env::temp_dir().join(format!("backup_fb_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = config_prueba();
        cfg.data_dir = tmp.clone();
        cfg.backup_directory = PathBuf::from("Backups");
        cfg.backup_max_copies = 2;
        // .fdb falso: gbak falla (no hay firebird en resource_dir) → copia
        let fdb = tmp.join("dinamo_rent_v3.fdb");
        fs::write(&fdb, b"contenido-fdb-falso").unwrap();
        cfg.db_path = fdb;
        let contenido = b"contenido-fdb-falso";
        for _ in 0..3 {
            let p = crear_backup(&cfg).unwrap();
            assert!(p.exists(), "backup creado: {}", p.display());
            assert_eq!(fs::read(&p).unwrap(), contenido);
        }
        // Rotación a 2: de 3 copias quedan 2
        let restantes = listar_backups(&cfg);
        assert_eq!(restantes.len(), 2);
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn debe_ejecutar_solo_en_horarios_y_una_vez_por_minuto() {
        let minutos = vec![540, 780, 1140, 1380]; // 09:00, 13:00, 19:00, 23:00
        let fecha = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        let t = fecha.and_hms_opt(9, 0, 30).unwrap();
        // En horario y sin marca → ejecuta
        assert!(debe_ejecutar(t, &minutos, None));
        // Mismo (fecha, minuto) ya ejecutado → no repite (incluso con intervalo < 60 s)
        assert!(!debe_ejecutar(t, &minutos, Some((fecha, 540))));
        // Minuto fuera de horario → no ejecuta
        let t2 = fecha.and_hms_opt(9, 1, 0).unwrap();
        assert!(!debe_ejecutar(t2, &minutos, None));
        // Día siguiente, mismo horario → ejecuta (la marca de ayer no bloquea)
        let manana = (fecha + chrono::Duration::days(1)).and_hms_opt(9, 0, 0).unwrap();
        assert!(debe_ejecutar(manana, &minutos, Some((fecha, 540))));
        // Horarios vacíos → nunca ejecuta
        assert!(!debe_ejecutar(t, &[], None));
    }
}
