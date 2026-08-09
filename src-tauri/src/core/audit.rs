//! audit.rs — Registro de auditoría (puerto de core/logger.py get_audit_logger)
//!
//! Escribe eventos (login OK/fallido, cambios de contraseña, accesos denegados)
//! en la tabla `auditoria`.

use rsfbclient::Execute;

use super::db::PooledConnection;
use super::error::AppError;

/// Inserta un evento de auditoría en la BD
pub fn log_audit(
    conn: &mut PooledConnection,
    usuario: &str,
    accion: &str,
    mensaje: &str,
    ip: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO auditoria (usuario, accion, mensaje, ip, fecha)
         VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
        (
            usuario.to_string(),
            accion.to_string(),
            mensaje.to_string(),
            ip.to_string(),
        ),
    )?;
    Ok(())
}
