//! core/ — Núcleo de la aplicación (puerto de la carpeta `core/` Python)

pub mod audit;
pub mod config;
pub mod crypto;
pub mod db;
pub mod error;
pub mod migrations;
pub mod rbac;
pub mod security;
pub mod validators;

/// Tipos compartidos usados por varios módulos
pub type Pool = db::Pool;
pub type PooledConnection = db::PooledConnection;
