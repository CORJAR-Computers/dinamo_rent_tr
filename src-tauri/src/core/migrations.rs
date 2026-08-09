//! migrations.rs — Runner de migraciones SQL (SQLx migrate no soporta Firebird)
//!
//! - Crea `schema_migrations(version VARCHAR PK, applied_at TIMESTAMP)` si no existe
//! - Aplica en orden los `.sql` de `migrations/` no ejecutados
//! - Cada migración corre en su propia transacción (DDL transaccional de Firebird)

use std::path::Path;

use rsfbclient::{Execute, Queryable};

use crate::core::error::AppError;

use super::db::Pool;

/// Aplica las migraciones pendientes. `migrations_dir` = src-tauri/migrations
pub fn run_migrations(pool: &Pool, migrations_dir: &Path) -> Result<(), AppError> {
    let mut conn = pool.get()?;

    // 1) Crear tabla de migraciones si no existe (Firebird no soporta IF NOT EXISTS)
    let has_migrations_table: Option<(i64,)> = conn.query_first(
        "SELECT COUNT(*) FROM RDB$RELATIONS WHERE RDB$RELATION_NAME = 'SCHEMA_MIGRATIONS'",
        (),
    )?;
    if has_migrations_table.map(|(c,)| c).unwrap_or(0) == 0 {
        conn.execute(
            "CREATE TABLE schema_migrations (
                version VARCHAR(100) NOT NULL PRIMARY KEY,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        )?;
    }

    // 2) Leer migraciones aplicadas
    let applied: Vec<String> = conn
        .query("SELECT version FROM schema_migrations", ())
        .map(|rows: Vec<(String,)>| rows.into_iter().map(|r| r.0).collect())
        .unwrap_or_default();

    // 3) Listar archivos .sql en orden
    let mut files: Vec<_> = std::fs::read_dir(migrations_dir)
        .map_err(|e| AppError::Generic(format!("No se pudo leer migrations dir: {e}")))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "sql").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());

    // Si la BD ya tiene el esquema inicial (ej: .fdb de producción reutilizado),
    // registrar 0001 como aplicada sin ejecutarla.
    let schema_exists = has_initial_schema(pool);

    let mut applied_count = 0;
    for entry in files {
        let name = entry.file_name().to_string_lossy().to_string();
        if applied.contains(&name) {
            continue;
        }

        // BD existente con el esquema completo → marcar como aplicada
        if schema_exists && name.starts_with("0001") {
            conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?)",
                (name.clone(),),
            )?;
            log::info!("BD existente detectada — migración {name} registrada como aplicada");
            applied_count += 1;
            continue;
        }

        let sql = std::fs::read_to_string(entry.path())
            .map_err(|e| AppError::Generic(format!("Error leyendo {name}: {e}")))?;

        log::info!("Aplicando migración: {name}");
        // Firebird ejecuta una sentencia por statement. Se eliminan PRIMERO las
        // líneas de comentario ('--') y luego se divide por ';': si se dividiera
        // antes, un comentario al inicio de un segmento descartaría la sentencia
        // completa que le sigue (y un ';' dentro de un comentario rompería todo).
        let sql_limpio: String = sql
            .lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let statements: Vec<&str> = sql_limpio
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for stmt in statements {
            conn.execute(stmt, ())?;
        }
        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?)",
            (name.clone(),),
        )?;
        applied_count += 1;
    }

    if applied_count > 0 {
        log::info!("Migraciones aplicadas: {applied_count}");
    } else {
        log::debug!("Migraciones: sin cambios");
    }
    Ok(())
}

/// Verifica si la BD tiene el esquema inicial (tabla usuarios)
pub fn has_initial_schema(pool: &Pool) -> bool {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let result: Result<Option<(i64,)>, rsfbclient::FbError> = conn.query_first(
        "SELECT COUNT(*) FROM RDB$RELATIONS WHERE RDB$SYSTEM_FLAG = 0 AND RDB$RELATION_NAME = 'USUARIOS'",
        (),
    );
    match result {
        Ok(Some((c,))) => c > 0,
        _ => false,
    }
}
