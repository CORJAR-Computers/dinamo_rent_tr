//! services/ — Lógica de negocio (puerto de la carpeta `services/` Python)

pub mod auditoria;
pub mod auth;
pub mod auto;
pub mod backup;
pub mod cliente;
pub mod comparendo;
pub mod gasto;
pub mod informe;
pub mod mantenimiento;
pub mod dashboard;
pub mod empresa;
pub mod pii;
pub mod reserva;
pub mod renta;
pub mod rotacion;
pub mod simit;
pub mod usuario;

pub use auth::AppState;
