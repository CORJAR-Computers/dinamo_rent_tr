//! migrations.rs — Runner de migraciones SQL (SQLx migrate no soporta Firebird)
//!
//! - Crea `schema_migrations(version VARCHAR PK, applied_at TIMESTAMP)` si no existe
//! - Aplica en orden los `.sql` de `migrations/` no ejecutados
//! - Cada sentencia se ejecuta en su propia transacción (autocommit) y la
//!   versión se registra al final.
//!
//! # Por qué NO se usa una transacción por migración
//!
//! Firebird NO deja ver en la misma transacción los cambios DDL hechos por ella:
//! un `ALTER TABLE ... ADD columna` seguido de un `UPDATE ... SET columna` dentro
//! de la misma transacción falla con "Column unknown" (las sentencias DSQL usan
//! un snapshot de metadatos fijado al preparar). Las migraciones 0003/0004
//! (no_contrato) combinan DDL + UPDATE backfill, así que una transacción única
//! las rompería en una instalación limpia.
//!
//! # Cómo se evitan los estados parciales (el bug original)
//!
//! El bug que rompía el arranque: una migración fallaba a mitad (p.ej. un
//! CREATE TRIGGER) y dejaba la columna creada pero la migración sin registrar;
//! al reintentar, el ALTER fallaba con "violation of PRIMARY or UNIQUE KEY
//! constraint ... on table RDB$RELATION_FIELDS" (columna duplicada).
//!
//! La defensa es DOBLE:
//!   1. TODAS las migraciones (0001-0014) usan DDL idempotente: EXECUTE BLOCK
//!      con guard contra RDB$RELATIONS / RDB$RELATION_FIELDS / RDB$INDICES /
//!      RDB$RELATION_CONSTRAINTS / RDB$GENERATORS (y RECREATE TRIGGER en 0007).
//!      Si una ejecución quedó a medias, el siguiente arranque se auto-repara
//!      (omite lo ya creado y crea lo que falta) — también en instalaciones
//!      nuevas: 0001-0014 están guardadas (0010/0011/0012/0013 eliminan
//!      índices redundantes/duplicados con DROP condicional si otro índice los
//!      cubre, y 0014 elimina tablas residuales de test si existen).
//!   2. Errores descriptivos: si una migración falla, el mensaje incluye el
//!      nombre de la migración, el número de sentencia y el SQL, y la versión
//!      NO se registra (el siguiente arranque la reintenta).
//!
//! Nota: 0001 solo se ejecuta de verdad sobre una BD vacía (en BDs existentes
//! se registra sin ejecutar vía `has_initial_schema`). 0002-0004 se ejecutan
//! siempre que no estén registradas; sus guards las hacen seguras de reintentar.
//!
//! # Ojo al editar migraciones (splitter)
//!
//! `split_sql_statements` divide por ';' respetando bloques BEGIN...END pero NO
//! respeta comillas: recorta los comentarios `--` de cada línea sin mirar si
//! están dentro de un literal. Las migraciones 0001-0014 llevan DDL dentro de
//! literales de EXECUTE STATEMENT (líneas muy largas): un valor con `--` (p.ej.
//! `DEFAULT 'x--y'`) truncaría la línea y rompería la migración silenciosamente.
//! Evitar `--` dentro de los literales y validar con los tests de migraciones.
//!
//! # Serie actual: 0001-0014
//!
//! 0001-0009 esquema + funciones (0001-0004 idempotentes, 0005-0009 auto-reparables),
//! 0010-0013 consolidación de índices, 0014 limpieza de tablas residuales de test.

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
        let statements = split_sql_statements(&sql);

        // Ejecutar sentencia por sentencia (autocommit). Si una falla, la
        // migración NO se registra y el siguiente arranque la reintenta; las
        // migraciones 0001-0014 son idempotentes y se auto-reparan.
        for (i, stmt) in statements.iter().enumerate() {
            conn.execute(stmt, ()).map_err(|e| {
                AppError::Database(format!(
                    "Migración {name} falló en la sentencia {}/{}: {e}\nSQL:\n{stmt}",
                    i + 1,
                    statements.len()
                ))
            })?;
        }
        conn.execute(
            "INSERT INTO schema_migrations (version) VALUES (?)",
            (name.clone(),),
        )
        .map_err(|e| {
            AppError::Database(format!(
                "Migración {name}: no se pudo registrar la versión: {e}"
            ))
        })?;
        applied_count += 1;
    }

    if applied_count > 0 {
        log::info!("Migraciones aplicadas: {applied_count}");
    } else {
        log::debug!("Migraciones: sin cambios");
    }
    Ok(())
}

/// Verifica si la BD tiene el esquema inicial COMPLETO de 0001.
///
/// Exige las 4 tablas núcleo (confirman que es una BD Dinamo Rent, no una
/// extranjera) MÁS `pagos`, la última tabla que crea 0001: eso garantiza que
/// 0001 terminó. Si solo se exigieran las núcleo, un crash de instalación
/// nueva tras `rentas` (sentencia ~20 de 38) dejaría 0001 registrada como
/// aplicada sin ejecutar el resto, y 0002+ fallarían por tablas inexistentes
/// (p.ej. `IDX_PAGOS_FECHA` sobre `pagos`) sin auto-reparación posible.
pub fn has_initial_schema(pool: &Pool) -> bool {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let result: Result<Option<(i64,)>, rsfbclient::FbError> = conn.query_first(
        "SELECT COUNT(*) FROM RDB$RELATIONS \
         WHERE RDB$SYSTEM_FLAG = 0 \
           AND RDB$RELATION_NAME IN ('USUARIOS', 'AUTOS', 'CLIENTES', 'RENTAS', 'PAGOS')",
        (),
    );
    match result {
        Ok(Some((c,))) => c >= 5,
        _ => false,
    }
}

/// Divide un script SQL en sentencias individuales respetando bloques BEGIN...END
/// (donde los ';' internos pertenecen al cuerpo PSQL y no terminan la sentencia DDL).
pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut block_depth: usize = 0;

    for line in sql.lines() {
        let trimmed_line = line.trim_start();
        if trimmed_line.starts_with("--") {
            continue;
        }

        // Remover comentarios al final de la línea si existen
        let line_content = if let Some(idx) = line.find("--") {
            &line[..idx]
        } else {
            line
        };

        // Escanear palabras para ajustar la profundidad del bloque BEGIN...END
        for word in line_content.split_whitespace() {
            let clean = word
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                .to_uppercase();
            if clean == "BEGIN" {
                block_depth += 1;
            } else if clean == "END" {
                if block_depth > 0 {
                    block_depth -= 1;
                }
            }
        }

        // Procesar caracteres de la línea
        for ch in line_content.chars() {
            current.push(ch);
            if ch == ';' && block_depth == 0 {
                let stmt = current.trim().to_string();
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                current.clear();
            }
        }
        current.push('\n');
    }

    let trailing = current.trim().to_string();
    if !trailing.is_empty() {
        statements.push(trailing);
    }

    statements
}
