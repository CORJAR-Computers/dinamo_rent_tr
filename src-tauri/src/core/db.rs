//! db.rs — Capa de base de datos (puerto de core/database_sa.py)
//!
//! Pool r2d2 sobre rsfbclient en modo EMBEDDED (fbclient.dll cargado en proceso).
//! El driver es síncrono → las operaciones se ejecutan con spawn_blocking.

use std::sync::Arc;

use r2d2_firebird::FirebirdConnectionManager;
use rsfbclient::{builder_native, Execute, Queryable};

use super::config::AppConfig;
use super::error::AppError;

/// Tipo de conexión del pool: builder nativo embedded con carga dinámica
pub type ConnBuilder =
    rsfbclient::NativeConnectionBuilder<rsfbclient::DynLoad, rsfbclient::ConnEmbedded>;
pub type Pool = r2d2::Pool<FirebirdConnectionManager<ConnBuilder>>;
pub type PooledConnection = r2d2::PooledConnection<FirebirdConnectionManager<ConnBuilder>>;

/// Builder embedded con carga dinámica de fbclient.dll (sin host/port/pass).
fn builder_embedded(cfg: &AppConfig) -> ConnBuilder {
    let mut builder = builder_native()
        .with_dyn_load(cfg.fbclient_path.to_string_lossy().to_string())
        .with_embedded();
    builder.db_name(cfg.db_path.to_string_lossy().to_string());
    builder.user(cfg.db_user.clone());
    builder
}

/// Crea el pool de conexiones Firebird Embedded.
///
/// Si la BD no existe (instalación nueva / equipo limpio), la crea con
/// `CREATE DATABASE` antes de abrir el pool: el driver embedded NO crea el
/// archivo al conectar (comportamiento verificado en la prueba de release
/// v1.0.0 — la app se quedaba colgada esperando una BD inexistente).
pub fn create_pool(cfg: &Arc<AppConfig>) -> Result<Pool, AppError> {
    if !cfg.fbclient_path.exists() {
        return Err(AppError::Database(format!(
            "No se encontró fbclient.dll en {:?}",
            cfg.fbclient_path
        )));
    }

    // Firebird embedded carga fbclient.dll por ruta completa, pero el loader de
    // Windows NO busca las dependencias de ese DLL (msvcp140.dll, vcruntime140*.dll,
    // icu*.dll) en la carpeta del propio fbclient.dll: solo busca en el dir de la
    // app, System32, Windows, cwd y PATH. En un equipo limpio sin el runtime VC++
    // instalado en System32, la carga falla con "LoadLibraryExW failed"
    // (ERROR_MOD_NOT_FOUND 126) y la app paniquea al arrancar. Solución: añadir
    // la carpeta que contiene a fbclient.dll al orden de búsqueda del proceso.
    #[cfg(windows)]
    add_firebird_dir_to_dll_search(&cfg.fbclient_path);

    // Instalación nueva: crear la BD vacía si el archivo no existe.
    if !cfg.db_path.exists() {
        log::info!(
            "BD no encontrada en {:?} — creando una nueva (instalación limpia)",
            cfg.db_path
        );
        if let Some(parent) = cfg.db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    AppError::Database(format!(
                        "No se pudo crear el directorio de la BD {:?}: {e}",
                        parent
                    ))
                })?;
            }
        }
        builder_embedded(cfg).create_database().map_err(|e| {
            AppError::Database(format!("No se pudo crear la BD {:?}: {e}", cfg.db_path))
        })?;
    }

    let manager = FirebirdConnectionManager::new(builder_embedded(cfg));
    let pool = r2d2::Pool::builder()
        .max_size(cfg.pool_size.max(1) as u32)
        .build(manager)?;

    // Prueba de conexión inicial
    let mut conn = pool.get()?;
    conn.execute("SELECT 1 FROM RDB$DATABASE", ())?;
    log::info!("Firebird Embedded conectado en {:?}", cfg.db_path);
    Ok(pool)
}

/// Añade el directorio de fbclient.dll al orden de búsqueda de DLLs del proceso
/// (SetDllDirectoryW), una sola vez por proceso.
///
/// Sin esto, en equipos limpios sin el runtime VC++ en System32 el loader no
/// encuentra las DLLs que fbclient.dll necesita (msvcp140.dll, vcruntime140*.dll)
/// aunque estén en la misma carpeta, y `LoadLibraryExW` falla (error 126).
/// Verificado en Windows Sandbox (Windows limpio) con el instalador NSIS.
#[cfg(windows)]
fn add_firebird_dir_to_dll_search(fbclient_path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use std::sync::Once;

    static ONCE: Once = Once::new();
    let Some(dir) = fbclient_path.parent() else {
        return;
    };
    ONCE.call_once(|| {
        extern "system" {
            fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
        }
        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: `wide` es una cadena UTF-16 válida terminada en NUL y sigue viva
        // durante toda la llamada. SetDllDirectoryW es segura de invocar en
        // cualquier momento del ciclo de vida del proceso (kernel32).
        unsafe { SetDllDirectoryW(wide.as_ptr()) };
        log::info!("SetDllDirectoryW -> {:?}", dir);
    });
}

/// Verifica la conexión y devuelve (ok, mensaje) — para diálogo de config BD
pub fn check_connection(pool: &Pool) -> (bool, String) {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return (false, format!("Conexión fallida: {e}")),
    };
    let result: Result<Option<(String,)>, rsfbclient::FbError> =
        conn.query_first("SELECT CURRENT_USER FROM RDB$DATABASE", ());
    match result {
        Ok(Some((user,))) => (true, format!("Firebird ({user}) — Conexión exitosa")),
        Ok(None) => (false, "Firebird — No se obtuvo usuario".to_string()),
        Err(e) => (false, format!("Conexión fallida: {e}")),
    }
}
