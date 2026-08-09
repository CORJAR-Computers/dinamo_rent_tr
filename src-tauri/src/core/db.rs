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
pub type ConnBuilder = rsfbclient::NativeConnectionBuilder<rsfbclient::DynLoad, rsfbclient::ConnEmbedded>;
pub type Pool = r2d2::Pool<FirebirdConnectionManager<ConnBuilder>>;
pub type PooledConnection = r2d2::PooledConnection<FirebirdConnectionManager<ConnBuilder>>;

/// Crea el pool de conexiones Firebird Embedded
pub fn create_pool(cfg: &Arc<AppConfig>) -> Result<Pool, AppError> {
    if !cfg.fbclient_path.exists() {
        return Err(AppError::Database(format!(
            "No se encontró fbclient.dll en {:?}",
            cfg.fbclient_path
        )));
    }

    // Construcción embedded (sin host/port/pass — no aplican)
    let mut builder = builder_native()
        .with_dyn_load(cfg.fbclient_path.to_string_lossy().to_string())
        .with_embedded();
    builder.db_name(cfg.db_path.to_string_lossy().to_string());
    builder.user(cfg.db_user.clone());

    let manager = FirebirdConnectionManager::new(builder);
    let pool = r2d2::Pool::builder()
        .max_size(cfg.pool_size.max(1) as u32)
        .build(manager)?;

    // Prueba de conexión inicial
    let mut conn = pool.get()?;
    conn.execute("SELECT 1 FROM RDB$DATABASE", ())?;
    log::info!("Firebird Embedded conectado en {:?}", cfg.db_path);
    Ok(pool)
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
