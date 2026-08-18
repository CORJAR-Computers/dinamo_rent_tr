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
//!
//! # Cifrado opcional (`[backup] encryption_enabled` / `encryption_password`)
//! Si el cifrado está activo, el `.fbk` se cifra **antes** de dejarlo con su
//! nombre final (el temporal en claro nunca queda en disco): AES-256-GCM por
//! chunks de 1 MiB con clave derivada de la contraseña vía PBKDF2-SHA256
//! (100 000 iteraciones, salt aleatorio de 16 bytes prefijado al archivo —
//! `PLAN_IMPLEMENTACION_TAURI.md` §2.6/§4.8). Formato en disco:
//!
//! ```text
//! [magic 8B "DRENC-01"][salt 16B] { [nonce 12B][ciphertext][tag 16B] }*
//! ```
//!
//! El nonce es un contador de chunk (clave única por archivo gracias al salt
//! fresco, así que el contador no se reutiliza entre archivos). Un backup
//! cifrado NO es un `.fbk` Firebird: para restaurarlo hay que descifrarlo
//! primero (`descifrar_archivo`).

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use serde::Serialize;
use sha2::Sha256;

use crate::core::config::AppConfig;
use crate::core::error::AppError;

/// Prefijo de los archivos de backup (coincide con el plan: `Backup_Dinamo_<ts>.fbk`)
const PREFIJO_BACKUP: &str = "Backup_Dinamo_";
/// Prefijo de los archivos staging de restauración (temporales en el dir de backups)
const PREFIJO_STAGING: &str = "restore_staging_";
/// Flag de línea de comandos con el que `backup_restaurar` relanza la app para
/// que el swap del `.fdb` ocurra en el arranque, ANTES de abrir el pool (el
/// motor Embedded abre la BD en exclusiva por proceso; la app actual debe
/// terminar para que gbak pueda reemplazar el archivo).
const FLAG_RESTAURAR: &str = "--restaurar-backup=";

// ─── Cifrado opcional (AES-256-GCM por chunks + PBKDF2) ──────────────────────

/// Magic header de un backup cifrado: distingue el `.fbk` cifrado de uno plano
const MAGIC_CIFRADO: &[u8; 8] = b"DRENC-01";
/// Salt PBKDF2 (16 bytes), prefijado al archivo y fresco por cada backup
const PBKDF2_SALT_LEN: usize = 16;
/// Iteraciones PBKDF2-SHA256 (las mismas que `security.hash_iterations`)
const PBKDF2_ITERATIONS: u32 = 100_000;
/// Chunk de cifrado: AES-256-GCM no es un cifrador de flujo; se trocea el
/// archivo en bloques de 1 MiB, cada uno con su nonce y tag (16 B).
const CHUNK_SIZE: usize = 1024 * 1024;
/// Longitud del nonce GCM (96 bits)
const GCM_NONCE_LEN: usize = 12;
/// Longitud del tag de autenticación GCM
const GCM_TAG_LEN: usize = 16;

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
        // Reintentos por la misma carrera de Defender sobre archivos recientes.
        if let Err(e) = reintentar_io(|| std::fs::remove_file(p), 4, 200) {
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
    // Reintentos: Defender puede escanear la BD justo tras escribirla y
    // bloquear el copy (sharing violation transitoria).
    reintentar_io(|| std::fs::copy(&cfg.db_path, destino).map(|_| ()), 8, 250)?;
    Ok(())
}

// ─── Cifrado de archivos (AES-256-GCM por chunks) ────────────────────────────

/// Deriva la clave AES-256 (32 bytes) desde la contraseña con PBKDF2-SHA256.
/// El salt es fresco por archivo → la clave es única por backup (el nonce
/// contador por chunk es seguro porque no se reutiliza entre archivos).
fn derivar_clave_cifrado(password: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut clave = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password, salt, PBKDF2_ITERATIONS, &mut clave);
    clave
}

/// Cifra `origen` (un `.fbk` en claro) a `destino` con AES-256-GCM por chunks
/// de `CHUNK_SIZE`, anteponiendo el magic + el salt PBKDF2 (formato documentado
/// en la cabecera del módulo). Streaming: no carga el archivo completo en
/// memoria. `password` vacía devuelve error (nunca se escribe un archivo
/// "cifrado" con clave vacía).
pub fn cifrar_archivo(origen: &Path, destino: &Path, password: &str) -> Result<(), AppError> {
    if password.is_empty() {
        return Err(AppError::Crypto(
            "Backup: no se puede cifrar con contraseña vacía".into(),
        ));
    }
    let mut entrada = File::open(origen)?;
    let mut salida = File::create(destino)?;
    salida.write_all(MAGIC_CIFRADO)?;
    let mut salt = [0u8; PBKDF2_SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salida.write_all(&salt)?;
    let clave = derivar_clave_cifrado(password.as_bytes(), &salt);
    let cipher = Aes256Gcm::new_from_slice(&clave)
        .map_err(|e| AppError::Crypto(format!("Error creando cifrador AES: {e}")))?;

    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut idx: u64 = 0;
    loop {
        let mut n = 0usize;
        while n < buf.len() {
            match entrada.read(&mut buf[n..]) {
                Ok(0) => break,
                Ok(k) => n += k,
                Err(e) => {
                    return Err(AppError::Generic(format!(
                        "Backup: error leyendo {origen:?}: {e}"
                    )))
                }
            }
        }
        if n == 0 {
            break; // fin del archivo (los backups nunca están vacíos)
        }
        let mut nonce_bytes = [0u8; GCM_NONCE_LEN];
        nonce_bytes[..8].copy_from_slice(&idx.to_be_bytes());
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), &buf[..n])
            .map_err(|e| AppError::Crypto(format!("Error cifrando chunk: {e}")))?;
        salida.write_all(&nonce_bytes)?;
        salida.write_all(&ct)?;
        idx += 1;
        if n < buf.len() {
            break; // último chunk (lectura corta)
        }
    }
    salida.flush()?;
    Ok(())
}

/// Descifra `origen` (backup cifrado con `cifrar_archivo`) a `destino`.
/// Verifica el magic, deriva la clave del salt prefijado y valida el tag GCM
/// de cada chunk (contraseña incorrecta o archivo manipulado → `AppError`).
pub fn descifrar_archivo(origen: &Path, destino: &Path, password: &str) -> Result<(), AppError> {
    // Escritura transaccional: se descifra a un temporal y solo se renombra al
    // destino final si TODO el archivo se descifró bien — un fallo (contraseña
    // incorrecta, manipulación) no deja un destino parcial o vacío.
    let temporal = PathBuf::from(format!("{}.tmp", destino.display()));
    let resultado = (|| -> Result<(), AppError> {
        let mut entrada = File::open(origen)?;
        let mut salida = File::create(&temporal)?;

        let mut magic = [0u8; MAGIC_CIFRADO.len()];
        entrada.read_exact(&mut magic)?;
        if &magic != MAGIC_CIFRADO {
            return Err(AppError::Crypto(
                "No es un backup cifrado (falta el magic DRENC-01)".into(),
            ));
        }
        let mut salt = [0u8; PBKDF2_SALT_LEN];
        entrada.read_exact(&mut salt)?;
        let clave = derivar_clave_cifrado(password.as_bytes(), &salt);
        let cipher = Aes256Gcm::new_from_slice(&clave)
            .map_err(|e| AppError::Crypto(format!("Error creando descifrador AES: {e}")))?;

        let mut primer_chunk = true;
        loop {
            let mut nonce_bytes = [0u8; GCM_NONCE_LEN];
            match entrada.read_exact(&mut nonce_bytes) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    if primer_chunk {
                        return Err(AppError::Crypto(
                            "Backup cifrado vacío o truncado (sin chunks)".into(),
                        ));
                    }
                    break; // fin normal del archivo
                }
                Err(e) => {
                    return Err(AppError::Generic(format!(
                        "Backup: error leyendo {origen:?}: {e}"
                    )))
                }
            }
            primer_chunk = false;
            // Ciphertext = CHUNK_SIZE (chunks completos) o menos (último); + tag
            let mut ct = vec![0u8; CHUNK_SIZE + GCM_TAG_LEN];
            let mut n = 0usize;
            while n < ct.len() {
                match entrada.read(&mut ct[n..]) {
                    Ok(0) => break,
                    Ok(k) => n += k,
                    Err(e) => {
                        return Err(AppError::Generic(format!(
                            "Backup: error leyendo {origen:?}: {e}"
                        )))
                    }
                }
            }
            if n < GCM_TAG_LEN {
                return Err(AppError::Crypto(
                    "Backup cifrado truncado (chunk sin tag completo)".into(),
                ));
            }
            ct.truncate(n);
            let pt = cipher
                .decrypt(Nonce::from_slice(&nonce_bytes), ct.as_slice())
                .map_err(|_| {
                    AppError::Crypto(
                        "No se pudo descifrar el backup (contraseña incorrecta o archivo dañado)"
                            .into(),
                    )
                })?;
            salida.write_all(&pt)?;
        }
        salida.flush()?;
        Ok(())
    })();
    match resultado {
        Ok(()) => {
            std::fs::rename(&temporal, destino)?;
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temporal);
            Err(e)
        }
    }
}

/// Crea un backup (gbak con fallback a copia del `.fdb`), lo cifra si está
/// habilitado y aplica la rotación. Devuelve la ruta del backup creado. Lo
/// usan el scheduler automático y (a futuro) el comando manual de backups.
///
/// El backup se genera en un archivo temporal: si el cifrado está activo se
/// cifra ANTES de dejarlo con su nombre final — nunca queda un `.fbk` en
/// claro con nombre de backup cuando `encryption_enabled = true`. Si el
/// cifrado está habilitado pero la contraseña está vacía, el backup FALLA
/// (misconfiguración: mejor fallar ruidoso que escribir en claro creyendo
/// que está cifrado) y no queda ningún archivo.
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
    let temporal = PathBuf::from(format!("{}.tmp", destino.display()));
    match crear_con_gbak(cfg, &temporal) {
        Ok(()) => log::info!("Backup: gbak OK → {}", destino.display()),
        Err(e) => {
            log::warn!("Backup: gbak falló ({e}) — fallback a copia del .fdb");
            copiar_fdb(cfg, &temporal)?;
        }
    }
    if cfg.backup_encryption_enabled {
        let password = cfg.backup_encryption_password.trim();
        if password.is_empty() {
            let _ = std::fs::remove_file(&temporal);
            return Err(AppError::Crypto(
                "Backup: encryption_enabled = true pero encryption_password está vacía — no se creó el backup".into(),
            ));
        }
        cifrar_archivo(&temporal, &destino, password)?;
        let _ = reintentar_io(|| std::fs::remove_file(&temporal), 8, 250);
        log::info!("Backup: cifrado AES-256-GCM aplicado a {}", destino.display());
    } else {
        // Reintentos: el `.tmp` recién escrito por gbak puede estar siendo
        // escaneado por Defender (sharing violation transitoria en Windows).
        reintentar_io(|| std::fs::rename(&temporal, &destino), 8, 250)?;
    }
    let borrados = rotar(cfg)?;
    if borrados > 0 {
        log::info!("Backup: rotación eliminó {borrados} copia(s) vieja(s)");
    }
    Ok(destino)
}

// ─── Restauración desde un backup ────────────────────────────────────────────

/// Reintenta una operación de archivo ante errores transitorios. En Windows,
/// el antivirus/Defender puede escanear un archivo recién escrito (p. ej. el
/// `.fbk` que acaba de dejar gbak, o la BD que acaba de sembrar seed_ci en el
/// runner del CI) y bloquear brevemente el copy/rename con
/// ERROR_SHARING_VIOLATION (os error 32) — la carrera que hizo fallar de
/// forma intermitente los tests de backups en GitHub Actions. Reintentos
/// cortos y acotados: un error real (permisos, ruta) falla al agotarlos igual
/// que antes; solo se absorbe el destello del escáner.
pub fn reintentar_io<T>(
    mut op: impl FnMut() -> std::io::Result<T>,
    intentos: u32,
    espera_ms: u64,
) -> std::io::Result<T> {
    let mut i = 0;
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) => {
                i += 1;
                if i >= intentos {
                    return Err(e);
                }
                std::thread::sleep(Duration::from_millis(espera_ms));
            }
        }
    }
}

/// Renombra `origen` sobre `destino` con reintentos. Al relanzar la app, el
/// proceso anterior puede tardar un momento en soltar el lock del `.fdb`
/// (motor Embedded exclusivo por proceso): gbak restaura a un temporal y el
/// rename es la ÚLTIMA operación sobre el archivo real — si falla, la BD
/// actual queda intacta y la app arranca con ella.
pub fn renombrar_con_reintentos(
    origen: &Path,
    destino: &Path,
    intentos: u32,
    espera_ms: u64,
) -> Result<(), AppError> {
    let mut i = 0;
    loop {
        match std::fs::rename(origen, destino) {
            Ok(()) => return Ok(()),
            Err(e) => {
                i += 1;
                if i >= intentos {
                    return Err(AppError::Generic(format!(
                        "No se pudo reemplazar la BD {:?}: {e}",
                        destino
                    )));
                }
                log::warn!(
                    "Restauración: reemplazo {:?} → {:?} falló ({e}); reintento {i}/{intentos}",
                    origen,
                    destino
                );
                std::thread::sleep(Duration::from_millis(espera_ms));
            }
        }
    }
}

/// Restaura un backup Firebird (`.fbk` en claro) sobre `db_path` con
/// `gbak -r` a un archivo temporal + rename atómico con reintentos. El `.fdb`
/// actual NO se toca hasta que gbak terminó OK (restauración transaccional):
/// si gbak falla o el rename no logra reemplazar, la BD existente queda
/// intacta y los temporales se limpian.
pub fn restaurar_fdb_desde_fbk(
    cfg: &AppConfig,
    staging_fbk: &Path,
    db_path: &Path,
) -> Result<(), AppError> {
    let gbak = encontrar_gbak(cfg);
    if !gbak.exists() {
        return Err(AppError::Generic(format!(
            "gbak.exe no encontrado en {}",
            gbak.display()
        )));
    }
    if !staging_fbk.exists() {
        return Err(AppError::Generic(format!(
            "El backup a restaurar no existe: {}",
            staging_fbk.display()
        )));
    }
    // gbak -r recrea el destino (NO debe existir) → restauramos a un temporal
    // y renombramos sobre el `.fdb` real al final (swap atómico).
    let destino_tmp = db_path.with_extension("fdb.restore.tmp");
    let _ = std::fs::remove_file(&destino_tmp);
    let firebird_dir = gbak.parent().unwrap_or(Path::new("."));
    let salida = Command::new(&gbak)
        .current_dir(firebird_dir)
        .args(["-r", "-user", &cfg.db_user, "-password", &cfg.db_password, "-v"])
        .arg(staging_fbk)
        .arg(&destino_tmp)
        .output()
        .map_err(|e| AppError::Generic(format!("no se pudo ejecutar gbak: {e}")))?;
    let resultado = (|| -> Result<(), AppError> {
        if !salida.status.success() {
            let stdout = String::from_utf8_lossy(&salida.stdout);
            let stderr = String::from_utf8_lossy(&salida.stderr);
            return Err(AppError::Generic(format!(
                "gbak -r falló ({}) — el backup puede estar dañado: {} {}",
                salida.status,
                stdout.trim(),
                stderr.trim()
            )));
        }
        let vacio = std::fs::metadata(&destino_tmp)
            .map(|m| m.len() == 0)
            .unwrap_or(true);
        if !destino_tmp.exists() || vacio {
            return Err(AppError::Generic(
                "gbak -r terminó OK pero no dejó la BD restaurada".into(),
            ));
        }
        renombrar_con_reintentos(&destino_tmp, db_path, 30, 500)
    })();
    if resultado.is_err() {
        let _ = std::fs::remove_file(&destino_tmp);
    }
    resultado
}

/// Prepara el staging para restaurar: si el backup está cifrado (magic
/// DRENC-01) lo descifra a un `.fbk` temporal (requiere contraseña); si está
/// en claro lo copia. El backup original NUNCA se modifica. Devuelve la ruta
/// del staging, que el arranque consumirá (y borrará) — así el relanzamiento
/// no necesita la contraseña.
pub fn preparar_staging(
    cfg: &AppConfig,
    archivo: &Path,
    password: Option<&str>,
) -> Result<PathBuf, AppError> {
    if !archivo.exists() {
        return Err(AppError::Generic(format!(
            "El backup no existe: {}",
            archivo.display()
        )));
    }
    let dir = dir_backups(cfg);
    let mut staging =
        dir.join(format!("{PREFIJO_STAGING}{}.fbk", Local::now().format("%Y%m%d_%H%M%S")));
    let mut sufijo = 2u32;
    while staging.exists() {
        staging = dir.join(format!(
            "{PREFIJO_STAGING}{}_{sufijo}.fbk",
            Local::now().format("%Y%m%d_%H%M%S")
        ));
        sufijo += 1;
    }
    if es_cifrado(archivo) {
        let pw = password.unwrap_or("").trim();
        if pw.is_empty() {
            return Err(AppError::Crypto(
                "El backup está cifrado: se requiere la contraseña para restaurarlo.".into(),
            ));
        }
        descifrar_archivo(archivo, &staging, pw)?;
    } else {
        std::fs::copy(archivo, &staging)?;
    }
    Ok(staging)
}

/// Extrae el staging del flag `--restaurar-backup=<ruta>` de la línea de
/// comandos (lo detecta `lib.rs` al arrancar, antes de abrir el pool).
pub fn staging_restauracion_desde_args() -> Option<PathBuf> {
    std::env::args()
        .find_map(|a| a.strip_prefix(FLAG_RESTAURAR).map(PathBuf::from))
        .filter(|p| !p.as_os_str().is_empty())
}

/// Modo restauración al arranque: restaura `staging` sobre la BD real y
/// borra el staging (es una copia; el backup original no se toca). El pool
/// todavía NO está abierto, así que gbak puede reemplazar el `.fdb` sin
/// conflictos de lock. Si falla, la BD actual queda intacta.
pub fn restaurar_en_arranque(cfg: &AppConfig, staging: &Path) -> Result<(), AppError> {
    if !staging.exists() {
        return Err(AppError::Generic(format!(
            "Staging de restauración no encontrado: {}",
            staging.display()
        )));
    }
    let resultado = restaurar_fdb_desde_fbk(cfg, staging, &cfg.db_path);
    // El staging siempre se limpia (éxito o fallo): es una copia temporal.
    let _ = std::fs::remove_file(staging);
    resultado
}

/// Relanza la app con `--restaurar-backup=<staging>` en un proceso
/// desacoplado (sin consola en Windows). El nuevo proceso hace el swap en su
/// arranque; la app actual debe terminar para soltar el lock exclusivo del
/// motor Embedded sobre el `.fdb` (ver `restaurar_en_arranque`).
pub fn lanzar_reinicio_con_restauracion(staging: &Path) -> Result<(), AppError> {
    let exe = std::env::current_exe().map_err(|e| {
        AppError::Generic(format!("No se pudo ubicar el ejecutable de la app: {e}"))
    })?;
    let mut cmd = Command::new(&exe);
    cmd.arg(format!("{FLAG_RESTAURAR}{}", staging.display()))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Proceso desacoplado: sin consola y sin que la muerte del padre lo
        // derribe (el hijo debe completar el swap aunque la app se cierre).
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.spawn().map_err(|e| {
        AppError::Generic(format!("No se pudo relanzar la app para restaurar: {e}"))
    })?;
    log::info!("Restauración: app relanzada con {}", staging.display());
    Ok(())
}

// ─── Estado en memoria (panel de la UI) ──────────────────────────────────────

/// Wrapper manejado por Tauri (app.manage) para que los comandos accedan al
/// estado de los backups sin ampliar AppState.
pub struct EstadoBackupManaged(pub Arc<EstadoBackup>);

/// Estado en memoria de los backups (visible para la UI)
#[derive(Default)]
pub struct EstadoBackup {
    interno: Mutex<EstadoBackupInner>,
    /// Evita backups concurrentes (manual + programado)
    pub ejecutando: AtomicBool,
}

#[derive(Default)]
struct EstadoBackupInner {
    ultimo_backup: Option<String>,
    ultimo_resultado: Option<String>,
    ultimo_error: Option<String>,
    /// Última restauración exitosa (origen) y su error (si falló) — el
    /// resultado del swap se registra en el arranque tras `restaurar_en_arranque`.
    ultima_restauracion: Option<String>,
    ultima_restauracion_error: Option<String>,
}

/// Una copia de seguridad en disco (para la tabla del panel)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoCopia {
    pub nombre: String,
    pub tamano_bytes: u64,
    /// Última modificación (RFC3339 local)
    pub modificado: String,
    /// true si empieza por el magic DRENC-01 (backup cifrado)
    pub cifrado: bool,
}

/// Info serializable del estado de backups para la UI
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoBackup {
    pub directorio: String,
    pub max_copies: usize,
    pub horarios: Vec<String>,
    pub cifrado: bool,
    pub ejecutando: bool,
    pub ultimo_backup: Option<String>,
    pub ultimo_resultado: Option<String>,
    pub ultimo_error: Option<String>,
    /// Próxima corrida programada (RFC3339 local), calculada de los horarios
    pub proxima_corrida: Option<String>,
    /// Copias existentes, de la más reciente a la más vieja
    pub copias: Vec<InfoCopia>,
    /// Última restauración exitosa (nombre del backup, si hubo)
    pub ultima_restauracion: Option<String>,
    /// Error de la última restauración (si falló)
    pub ultima_restauracion_error: Option<String>,
}

impl EstadoBackup {
    pub fn esta_ejecutando(&self) -> bool {
        self.ejecutando.load(Ordering::SeqCst)
    }

    /// Intenta tomar la ejecución de forma atómica (CAS). Solo un hilo gana:
    /// evita que el backup programado y el manual corran a la vez.
    pub fn claimar(&self) -> bool {
        self.ejecutando
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Libera la ejecución (siempre tras `claimar`).
    pub fn liberar(&self) {
        self.ejecutando.store(false, Ordering::SeqCst);
    }

    fn registrar_ok(&self, path: &Path) {
        let mut i = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        i.ultimo_backup = Some(path.to_string_lossy().into_owned());
        i.ultimo_resultado = Some("OK".into());
        i.ultimo_error = None;
    }

    fn registrar_error(&self, error: &str) {
        let mut i = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        i.ultimo_error = Some(error.to_string());
    }

    /// Registra el resultado de una restauración: `origen` es el nombre del
    /// backup restaurado. Lo llaman el arranque (tras el swap) y el comando
    /// `backup_restaurar` cuando falla ANTES del reinicio (p. ej. contraseña
    /// incorrecta) para que el panel muestre el motivo.
    pub fn registrar_resultado_restauracion(&self, origen: &str, resultado: Result<(), String>) {
        let mut i = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        match resultado {
            Ok(()) => {
                i.ultima_restauracion = Some(origen.to_string());
                i.ultima_restauracion_error = None;
            }
            Err(e) => {
                i.ultima_restauracion_error = Some(format!("{origen}: {e}"));
            }
        }
    }

    /// Estado completo para la UI: configuración + última corrida + copias.
    pub fn info(&self, cfg: &AppConfig) -> InfoBackup {
        let i = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        InfoBackup {
            directorio: dir_backups(cfg).to_string_lossy().into_owned(),
            max_copies: cfg.backup_max_copies,
            horarios: cfg.backup_schedule_times.clone(),
            cifrado: cfg.backup_encryption_enabled,
            ejecutando: self.esta_ejecutando(),
            ultimo_backup: i.ultimo_backup.clone(),
            ultimo_resultado: i.ultimo_resultado.clone(),
            ultimo_error: i.ultimo_error.clone(),
            proxima_corrida: proxima_corrida_desde(
                Local::now().naive_local(),
                &cfg.backup_schedule_minutes,
            )
            .map(|t| {
                Local.from_local_datetime(&t)
                    .single()
                    .map(|dt| dt.to_rfc3339())
                    // Ambiguidad de DST (no aplica en Colombia): fallback sin offset
                    .unwrap_or_else(|| t.format("%Y-%m-%dT%H:%M:%S").to_string())
            }),
            copias: listar_backups(cfg)
                .iter()
                .rev()
                .map(|p| InfoCopia {
                    nombre: p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    tamano_bytes: std::fs::metadata(p).map(|m| m.len()).unwrap_or(0),
                    modificado: std::fs::metadata(p)
                        .and_then(|m| m.modified())
                        .map(|t: std::time::SystemTime| -> DateTime<Local> { t.into() })
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_default(),
                    cifrado: es_cifrado(p),
                })
                .collect(),
            ultima_restauracion: i.ultima_restauracion.clone(),
            ultima_restauracion_error: i.ultima_restauracion_error.clone(),
        }
    }
}

/// ¿El archivo es un backup cifrado? (empieza por el magic DRENC-01)
pub fn es_cifrado(ruta: &Path) -> bool {
    let mut f = match File::open(ruta) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut magic = [0u8; MAGIC_CIFRADO.len()];
    f.read_exact(&mut magic)
        .map(|_| &magic == MAGIC_CIFRADO)
        .unwrap_or(false)
}

/// Ejecuta un backup (crear + rotación) registrando el resultado en el estado.
/// Es la única entrada compartida por el scheduler y el comando manual
/// `backup_ahora`; el claim evita que corran a la vez.
pub fn run_backup(cfg: &AppConfig, estado: &EstadoBackup) -> Result<PathBuf, AppError> {
    if !estado.claimar() {
        return Err(AppError::Business(
            "Ya hay un backup en curso. Espera a que termine.".into(),
        ));
    }
    let resultado = crear_backup(cfg);
    match &resultado {
        Ok(path) => estado.registrar_ok(path),
        Err(e) => estado.registrar_error(&e.to_string()),
    }
    estado.liberar();
    resultado
}

/// Próximo horario (fecha+hora local) >= ahora, o None si no hay horarios.
/// Si ya pasaron todos los de hoy, devuelve el primero de mañana.
fn proxima_corrida_desde(ahora: NaiveDateTime, minutos: &[u32]) -> Option<NaiveDateTime> {
    let minuto_actual = ahora.hour() * 60 + ahora.minute();
    for &m in minutos {
        if m > minuto_actual {
            return Some(ahora.date().and_hms_opt(m / 60, m % 60, 0).expect("hora válida"));
        }
    }
    minutos.first().map(|&m| {
        (ahora.date() + chrono::Duration::days(1))
            .and_hms_opt(m / 60, m % 60, 0)
            .expect("hora válida")
    })
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
pub fn spawn_scheduler(cfg: Arc<AppConfig>, estado: Arc<EstadoBackup>) {
    // Clones para el hilo: la config original se usa para el log de arranque.
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
                    match run_backup(&cfg_hilo, &estado) {
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

    // ─── Cifrado opcional ────────────────────────────────────────────────────

    /// Escribe un archivo pseudo-aleatorio y devuelve su ruta (panic-safe)
    fn archivo_prueba(dir: &Path, nombre: &str, bytes: usize) -> PathBuf {
        let ruta = dir.join(nombre);
        let mut datos = vec![0u8; bytes];
        rand::thread_rng().fill_bytes(&mut datos);
        fs::write(&ruta, &datos).unwrap();
        ruta
    }

    #[test]
    fn cifrado_roundtrip_multichunk() {
        let tmp = std::env::temp_dir().join(format!("backup_enc_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        // 2.5 MiB: cruza el límite de CHUNK_SIZE → 3 chunks (2 completos + 1 corto)
        let origen = archivo_prueba(&tmp, "original.fbk", CHUNK_SIZE * 2 + CHUNK_SIZE / 2);
        let cifrado = tmp.join("backup.fbk.enc");
        cifrar_archivo(&origen, &cifrado, "clave-secreta").unwrap();

        let enc_bytes = fs::read(&cifrado).unwrap();
        let datos = fs::read(&origen).unwrap();
        // Magic + salt prefijados, y el contenido NO está en claro
        assert!(enc_bytes.starts_with(MAGIC_CIFRADO));
        assert_ne!(enc_bytes, datos);

        // Round-trip: el descifrado reproduce el archivo original byte a byte
        let restaurado = tmp.join("restaurado.fbk");
        descifrar_archivo(&cifrado, &restaurado, "clave-secreta").unwrap();
        assert_eq!(fs::read(&restaurado).unwrap(), datos);
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cifrado_roundtrip_tamano_exacto_de_chunk() {
        let tmp = std::env::temp_dir().join(format!("backup_enc2_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        // Plaintext de exactamente CHUNK_SIZE: 1 chunk completo, fin limpio
        let origen = archivo_prueba(&tmp, "original.fbk", CHUNK_SIZE);
        let cifrado = tmp.join("backup.fbk.enc");
        cifrar_archivo(&origen, &cifrado, "clave").unwrap();
        let restaurado = tmp.join("restaurado.fbk");
        descifrar_archivo(&cifrado, &restaurado, "clave").unwrap();
        assert_eq!(fs::read(&restaurado).unwrap(), fs::read(&origen).unwrap());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cifrado_clave_incorrecta_falla() {
        let tmp = std::env::temp_dir().join(format!("backup_enc3_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let origen = archivo_prueba(&tmp, "original.fbk", 64 * 1024);
        let cifrado = tmp.join("backup.fbk.enc");
        cifrar_archivo(&origen, &cifrado, "clave-a").unwrap();
        let restaurado = tmp.join("restaurado.fbk");
        assert!(descifrar_archivo(&cifrado, &restaurado, "clave-b").is_err());
        assert!(!restaurado.exists(), "no debe quedar salida al fallar");
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cifrado_detecta_manipulacion() {
        let tmp = std::env::temp_dir().join(format!("backup_enc4_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let origen = archivo_prueba(&tmp, "original.fbk", 64 * 1024);
        let cifrado = tmp.join("backup.fbk.enc");
        cifrar_archivo(&origen, &cifrado, "clave").unwrap();

        // Manipular un byte a mitad del archivo → el tag GCM del chunk falla
        let mut bytes = fs::read(&cifrado).unwrap();
        let medio = bytes.len() / 2;
        bytes[medio] ^= 0x01;
        fs::write(&cifrado, &bytes).unwrap();
        let restaurado = tmp.join("restaurado.fbk");
        assert!(descifrar_archivo(&cifrado, &restaurado, "clave").is_err());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn crear_backup_con_cifrado_aplica_magic_y_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("backup_enc5_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = config_prueba();
        cfg.data_dir = tmp.clone();
        cfg.backup_directory = PathBuf::from("Backups");
        cfg.backup_max_copies = 2;
        cfg.backup_encryption_enabled = true;
        cfg.backup_encryption_password = "clave-del-backup".into();
        let fdb = tmp.join("dinamo_rent_v3.fdb");
        fs::write(&fdb, b"contenido-fdb-falso").unwrap();
        cfg.db_path = fdb;

        let p = crear_backup(&cfg).unwrap();
        let enc = fs::read(&p).unwrap();
        assert!(enc.starts_with(MAGIC_CIFRADO), "backup cifrado: {}", p.display());
        // Round-trip: descifrar reproduce el .fdb original
        let rest = tmp.join("restaurado.fbk");
        descifrar_archivo(&p, &rest, "clave-del-backup").unwrap();
        assert_eq!(fs::read(&rest).unwrap(), b"contenido-fdb-falso");
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn crear_backup_cifrado_sin_password_rechaza_y_no_deja_archivos() {
        let tmp = std::env::temp_dir().join(format!("backup_enc6_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = config_prueba();
        cfg.data_dir = tmp.clone();
        cfg.backup_directory = PathBuf::from("Backups");
        cfg.backup_encryption_enabled = true;
        cfg.backup_encryption_password = "".into();
        let fdb = tmp.join("dinamo_rent_v3.fdb");
        fs::write(&fdb, b"contenido-fdb-falso").unwrap();
        cfg.db_path = fdb;

        // Misconfiguración: cifrado activo sin contraseña → el backup FALLA y
        // no queda ni el temporal en claro ni un backup con nombre final.
        assert!(crear_backup(&cfg).is_err());
        assert_eq!(listar_backups(&cfg).len(), 0);
        let dir = dir_backups(&cfg);
        let sobras: Vec<String> = std::fs::read_dir(&dir)
            .map(|e| {
                e.filter_map(|x| x.ok())
                    .map(|x| x.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        assert!(sobras.is_empty(), "temporales en claro sobrantes: {sobras:?}");
        fs::remove_dir_all(&tmp).unwrap();
    }

    // ─── Estado y próxima corrida ───────────────────────────────────────────

    #[test]
    fn proxima_corrida_calcula_el_siguiente_horario() {
        let minutos = vec![540, 780, 1140, 1380]; // 09:00, 13:00, 19:00, 23:00
        let fecha = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
        // Antes del primero → el de hoy
        let t = fecha.and_hms_opt(8, 0, 0).unwrap();
        assert_eq!(
            proxima_corrida_desde(t, &minutos),
            Some(fecha.and_hms_opt(9, 0, 0).unwrap())
        );
        // Entre horarios → el siguiente de hoy
        let t2 = fecha.and_hms_opt(14, 30, 0).unwrap();
        assert_eq!(
            proxima_corrida_desde(t2, &minutos),
            Some(fecha.and_hms_opt(19, 0, 0).unwrap())
        );
        // Después del último → el primero de mañana
        let t3 = fecha.and_hms_opt(23, 30, 0).unwrap();
        assert_eq!(
            proxima_corrida_desde(t3, &minutos),
            Some((fecha + chrono::Duration::days(1)).and_hms_opt(9, 0, 0).unwrap())
        );
        // Sin horarios → None
        assert_eq!(proxima_corrida_desde(t, &[]), None);
    }

    #[test]
    fn run_backup_registra_estado_y_evita_concurrencia() {
        let tmp = std::env::temp_dir().join(format!("backup_estado_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = config_prueba();
        cfg.data_dir = tmp.clone();
        cfg.backup_directory = PathBuf::from("Backups");
        let fdb = tmp.join("dinamo_rent_v3.fdb");
        fs::write(&fdb, b"contenido-fdb-falso").unwrap();
        cfg.db_path = fdb;

        let estado = EstadoBackup::default();
        // Claim previo → run_backup rechaza (no se solapa con el programado)
        assert!(estado.claimar());
        assert!(run_backup(&cfg, &estado).is_err());
        estado.liberar();
        // Sin claim → crea el backup y registra OK
        let p = run_backup(&cfg, &estado).unwrap();
        assert!(p.exists());
        let info = estado.info(&cfg);
        assert_eq!(info.ultimo_backup, Some(p.to_string_lossy().into_owned()));
        assert_eq!(info.ultimo_error, None);
        // El estado expone la copia
        assert_eq!(info.copias.len(), 1);
        assert_eq!(info.copias[0].nombre, p.file_name().unwrap().to_string_lossy());
        assert!(!info.copias[0].cifrado);
        // Tras liberar, info() muestra ejecutando = false
        assert!(!info.ejecutando);
        fs::remove_dir_all(&tmp).unwrap();
    }

    // ─── Reintentos de I/O (carrera de Defender en Windows) ───────────────────

    #[test]
    fn reintentar_io_absorbe_fallos_transitorios() {
        // Falla 2 veces con sharing violation (os error 32) y luego funciona
        let mut intentos = 0;
        let r = reintentar_io(
            || {
                intentos += 1;
                if intentos < 3 {
                    Err(std::io::Error::from_raw_os_error(32))
                } else {
                    Ok(42)
                }
            },
            5,
            5,
        )
        .unwrap();
        assert_eq!(r, 42);
        assert_eq!(intentos, 3, "debe reintentar 2 veces y acertar al 3er intento");

        // Errores persistentes agotan los intentos y devuelven el último error
        let err = reintentar_io(
            || -> std::io::Result<i32> { Err(std::io::Error::from_raw_os_error(32)) },
            2,
            5,
        )
        .unwrap_err();
        assert_eq!(err.raw_os_error(), Some(32));
    }

    // ─── Restauración ────────────────────────────────────────────────────────

    #[test]
    fn preparar_staging_copia_el_fbk_plano() {
        let tmp = std::env::temp_dir().join(format!("restore_plano_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = config_prueba();
        cfg.data_dir = tmp.clone();
        cfg.backup_directory = PathBuf::from("Backups");
        let dir = dir_backups(&cfg);
        fs::create_dir_all(&dir).unwrap();

        let origen = dir.join(format!("{PREFIJO_BACKUP}20260817_120000.fbk"));
        let contenido = b"backup-en-claro-falso";
        fs::write(&origen, contenido).unwrap();

        let staging = preparar_staging(&cfg, &origen, None).unwrap();
        // Copia en claro: el original no se toca y el staging tiene el contenido
        assert_eq!(fs::read(&staging).unwrap(), contenido);
        assert_eq!(fs::read(&origen).unwrap(), contenido);
        assert_ne!(staging, origen);
        assert!(staging.file_name().unwrap().to_string_lossy().starts_with(PREFIJO_STAGING));
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn preparar_staging_descifra_el_backup_cifrado() {
        let tmp = std::env::temp_dir().join(format!("restore_cifrado_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = config_prueba();
        cfg.data_dir = tmp.clone();
        cfg.backup_directory = PathBuf::from("Backups");
        let dir = dir_backups(&cfg);
        fs::create_dir_all(&dir).unwrap();

        let origen = dir.join(format!("{PREFIJO_BACKUP}20260817_120000.fbk"));
        let contenido = b"backup-cifrado-falso";
        fs::write(&origen, contenido).unwrap();
        let cifrado = dir.join("cifrado.tmp");
        cifrar_archivo(&origen, &cifrado, "clave").unwrap();
        fs::remove_file(&origen).unwrap();

        // Sin contraseña → error claro (requiere el password del usuario)
        assert!(preparar_staging(&cfg, &cifrado, None).is_err());
        // Contraseña incorrecta → error (tag GCM) sin staging sobrante
        assert!(preparar_staging(&cfg, &cifrado, Some("otra")).is_err());
        // Correcta → staging en claro con el contenido original
        let staging = preparar_staging(&cfg, &cifrado, Some("clave")).unwrap();
        assert_eq!(fs::read(&staging).unwrap(), contenido);
        assert!(!es_cifrado(&staging));
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn renombrar_con_reintentos_reemplaza_el_existente() {
        let tmp = std::env::temp_dir().join(format!("restore_rename_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let destino = tmp.join("dinamo_rent_v3.fdb");
        fs::write(&destino, b"bd-actual").unwrap();
        let origen = tmp.join("nueva.fdb");
        fs::write(&origen, b"bd-restaurada").unwrap();

        renombrar_con_reintentos(&origen, &destino, 3, 10).unwrap();
        assert_eq!(fs::read(&destino).unwrap(), b"bd-restaurada");
        assert!(!origen.exists());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn restaurar_sin_gbak_falla_y_deja_la_bd_intacta() {
        let tmp = std::env::temp_dir().join(format!("restore_nogbak_{}", uniq()));
        fs::create_dir_all(&tmp).unwrap();
        let mut cfg = config_prueba(); // resource_dir sin firebird
        cfg.data_dir = tmp.clone();
        let fdb = tmp.join("dinamo_rent_v3.fdb");
        fs::write(&fdb, b"bd-actual-intacta").unwrap();
        cfg.db_path = fdb;
        let staging = tmp.join("staging.fbk");
        fs::write(&staging, b"backup-falso").unwrap();

        // Sin gbak: la restauración falla con error claro y NO toca la BD
        let err = restaurar_fdb_desde_fbk(&cfg, &staging, &cfg.db_path).unwrap_err();
        assert!(err.to_string().contains("gbak"), "{err}");
        assert_eq!(fs::read(&cfg.db_path).unwrap(), b"bd-actual-intacta");
        assert!(!cfg.db_path.with_extension("fdb.restore.tmp").exists(), "sin temporales");
        // restaurar_en_arranque además limpia el staging (éxito o fallo)
        let err2 = restaurar_en_arranque(&cfg, &staging).unwrap_err();
        assert!(err2.to_string().contains("gbak"));
        assert!(!staging.exists(), "staging limpiado tras el arranque");
        assert_eq!(fs::read(&cfg.db_path).unwrap(), b"bd-actual-intacta");
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn registrar_resultado_restauracion_alimenta_el_estado() {
        let estado = EstadoBackup::default();
        estado.registrar_resultado_restauracion("Backup_Dinamo_20260817_120000.fbk", Ok(()));
        let cfg = config_prueba();
        let info = estado.info(&cfg);
        assert_eq!(
            info.ultima_restauracion.as_deref(),
            Some("Backup_Dinamo_20260817_120000.fbk")
        );
        assert_eq!(info.ultima_restauracion_error, None);

        // Un fallo posterior no borra la última restauración exitosa, pero
        // sí expone el error (misma semántica que ultimo_backup/ultimo_error).
        estado.registrar_resultado_restauracion(
            "Backup_Dinamo_20260817_120000.fbk",
            Err("gbak falló".into()),
        );
        let info = estado.info(&cfg);
        assert_eq!(
            info.ultima_restauracion.as_deref(),
            Some("Backup_Dinamo_20260817_120000.fbk")
        );
        assert!(info.ultima_restauracion_error.as_deref().unwrap().contains("gbak falló"));
    }
}
