//! core/ — Núcleo de la aplicación (puerto de la carpeta `core/` Python)

pub mod audit;
pub mod config;
pub mod crypto;
pub mod db;
/// Deserializadores tolerantes de montos monetarios (`String` | número JSON)
/// para los DTOs de comandos Tauri. Ver el módulo para la motivación.
pub mod decimal_string;
pub mod error;
pub mod migrations;
pub mod rbac;
/// Helpers compartidos por los repositorios (DRY de `map_fb_error`,
/// `opt_str`, `parse_fecha_opt`, `parse_hora_opt` y el macro `params!`).
/// Ver módulo para el detalle de la migración parcial (Bloque 4 / TAREA 4.2).
pub mod repository;
pub mod security;
pub mod validators;

/// Tipos compartidos usados por varios módulos
pub type Pool = db::Pool;
pub type PooledConnection = db::PooledConnection;
