//! commands/logs.rs — Comandos Tauri para gestión de logs y errores
//!
//! Permite al frontend:
//!   - Leer los últimos N líneas del log de la app
//!   - Exportar todos los logs como un archivo de texto
//!   - Registrar errores del frontend (JS) en un archivo separado
//!   - Leer los errores del frontend

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::core::error::{AppError, ErrorPayload};
use crate::services::AppState;
use tauri::State;

use super::require_usuario_admin;

type Cmd<T> = Result<T, ErrorPayload>;

fn cmd(err: AppError) -> ErrorPayload {
    err.to_payload()
}

/// Devuelve las últimas `lineas` del log principal (app.log).
#[tauri::command]
pub fn leer_logs(
    state: State<'_, AppState>,
    session_id: String,
    lineas: Option<usize>,
) -> Cmd<String> {
    require_usuario_admin(&state, &session_id)?;
    let log_path = state.config.data_dir.join("logs").join("app.log");
    read_last_lines(&log_path, lineas.unwrap_or(500))
}

/// Devuelve los errores del frontend (frontend_errors.log).
#[tauri::command]
pub fn leer_errores_frontend(
    state: State<'_, AppState>,
    session_id: String,
    lineas: Option<usize>,
) -> Cmd<String> {
    require_usuario_admin(&state, &session_id)?;
    let err_path = state
        .config
        .data_dir
        .join("logs")
        .join("frontend_errors.log");
    read_last_lines(&err_path, lineas.unwrap_or(200))
}

/// Escapa caracteres de control en strings provenientes del frontend
/// para prevenir log injection (falsificación de entradas mediante \n, \r, etc.).
/// Ver mejora #3 del roadmap de Dinamo Rent ERP.
fn sanitize_log(s: &str) -> String {
    s.replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\x00', "\\x00")
        .replace('\x1b', "\\x1b")
}

/// Sanitiza un Option<&str> (devuelve "-" si es None, sanitizado si tiene valor).
fn sanitize_opt(s: Option<&str>) -> String {
    match s {
        None => "-".to_string(),
        Some(v) => sanitize_log(v),
    }
}

/// Registra un error del frontend en frontend_errors.log.
#[tauri::command]
pub fn registrar_error_frontend(
    state: State<'_, AppState>,
    session_id: String,
    mensaje: String,
    stack: Option<String>,
    url: Option<String>,
    linea: Option<u32>,
    columna: Option<u32>,
) -> Cmd<()> {
    use super::require_session;
    let sesion = require_session(&state, &session_id)?;

    let log_dir = state.config.data_dir.join("logs");
    let _ = fs::create_dir_all(&log_dir);
    let err_path = log_dir.join("frontend_errors.log");

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let entry = format!(
        "[{}] [user={}] {}\n  url: {}\n  línea: {}:{}\n  stack: {}\n\n",
        timestamp,
        sesion.username,
        sanitize_log(&mensaje),
        sanitize_opt(url.as_deref()),
        linea.unwrap_or(0),
        columna.unwrap_or(0),
        sanitize_opt(stack.as_deref()),
    );

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&err_path)
        .map_err(|e| {
            cmd(AppError::Generic(format!(
                "No se pudo abrir frontend_errors.log: {e}"
            )))
        })?;

    file.write_all(entry.as_bytes())
        .map_err(|e| cmd(AppError::Generic(format!("No se pudo escribir: {e}"))))?;

    Ok(())
}

/// Exporta todos los logs como un solo archivo de texto.
#[tauri::command]
pub fn exportar_logs(state: State<'_, AppState>, session_id: String) -> Cmd<String> {
    require_usuario_admin(&state, &session_id)?;

    let log_dir = state.config.data_dir.join("logs");
    let mut output = String::new();

    output.push_str(&format!(
        "=== Dinamo Rent ERP — Exportación de Logs ===\n\
         Fecha: {}\n\
         Versión: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        env!("CARGO_PKG_VERSION"),
    ));

    // app.log
    let app_log = log_dir.join("app.log");
    if app_log.exists() {
        output.push_str("═══════════════════════════════════════════\n");
        output.push_str("  APP.LOG (backend)\n");
        output.push_str("═══════════════════════════════════════════\n");
        match fs::read_to_string(&app_log) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let start = lines.len().saturating_sub(2000);
                for line in &lines[start..] {
                    output.push_str(line);
                    output.push('\n');
                }
                output.push_str(&format!(
                    "\n(mostrando últimas {} de {} líneas)\n\n",
                    lines.len() - start,
                    lines.len()
                ));
            }
            Err(e) => output.push_str(&format!("(error leyendo app.log: {e})\n\n")),
        }
    }

    // frontend_errors.log
    let err_log = log_dir.join("frontend_errors.log");
    if err_log.exists() {
        output.push_str("═══════════════════════════════════════════\n");
        output.push_str("  FRONTEND_ERRORS.LOG (errores JS)\n");
        output.push_str("═══════════════════════════════════════════\n");
        match fs::read_to_string(&err_log) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let start = lines.len().saturating_sub(500);
                for line in &lines[start..] {
                    output.push_str(line);
                    output.push('\n');
                }
                output.push_str(&format!(
                    "\n(mostrando últimas {} de {} líneas)\n\n",
                    lines.len() - start,
                    lines.len()
                ));
            }
            Err(e) => output.push_str(&format!("(error leyendo frontend_errors.log: {e})\n\n")),
        }
    }

    // Info del sistema
    output.push_str("═══════════════════════════════════════════\n");
    output.push_str("  INFO DEL SISTEMA\n");
    output.push_str("═══════════════════════════════════════════\n");
    output.push_str(&format!("OS: {}\n", std::env::consts::OS));
    output.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    output.push_str(&format!("Data dir: {}\n", state.config.data_dir.display()));
    output.push_str(&format!("DB path: {}\n", state.config.db_path.display()));

    if log_dir.exists() {
        output.push_str("\nArchivos de log:\n");
        if let Ok(entries) = fs::read_dir(&log_dir) {
            for entry in entries.flatten() {
                let meta = entry.metadata().ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = meta
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Local> = t.into();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_else(|| "?".into());
                output.push_str(&format!(
                    "  {} ({} bytes, modificado: {})\n",
                    entry.file_name().to_string_lossy(),
                    size,
                    modified
                ));
            }
        }
    }

    Ok(output)
}

/// Trunca los archivos de log.
#[tauri::command]
pub fn limpiar_logs(state: State<'_, AppState>, session_id: String) -> Cmd<u32> {
    require_usuario_admin(&state, &session_id)?;

    let log_dir = state.config.data_dir.join("logs");
    let mut eliminados = 0u32;

    if log_dir.exists() {
        let entries: Vec<_> = fs::read_dir(&log_dir)
            .map_err(|e| {
                cmd(AppError::Generic(format!(
                    "No se pudo leer directorio: {e}"
                )))
            })?
            .flatten()
            .collect();

        for entry in entries {
            let path = entry.path();
            if path.extension().map(|e| e == "log").unwrap_or(false)
                && fs::File::create(&path).is_ok()
            {
                eliminados += 1;
            }
        }
    }

    log::info!("Logs truncados: {eliminados} archivo(s)");
    Ok(eliminados)
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Lee las últimas N líneas de un archivo.
fn read_last_lines(path: &PathBuf, n: usize) -> Cmd<String> {
    if !path.exists() {
        return Ok(String::from("(no hay logs disponibles)"));
    }

    let file = fs::File::open(path).map_err(|e| {
        cmd(AppError::Generic(format!(
            "No se pudo abrir {}: {e}",
            path.display()
        )))
    })?;

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let start = lines.len().saturating_sub(n);
    let output: String = lines[start..].join("\n");

    if output.is_empty() {
        Ok(String::from("(log vacío)"))
    } else {
        Ok(output)
    }
}
