//! services/ — Lógica de negocio (puerto de la carpeta `services/` Python)

pub mod auditoria;
pub mod auth;
pub mod auto;
pub mod cliente;
pub mod comparendo;
pub mod gasto;
pub mod informe;
pub mod mantenimiento;
pub mod dashboard;
pub mod pii;
pub mod reserva;
pub mod renta;
pub mod usuario;

pub use auth::AppState;
