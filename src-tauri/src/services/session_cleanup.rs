//! services/session_cleanup.rs — Limpieza periódica de sesiones expiradas.
//!
//! purge_expired() existe en rbac.rs pero nunca se llamaba. Este módulo
//! lanza un hilo de fondo que la ejecuta cada 5 minutos para liberar
//! memoria de sesiones inactivas.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::core::rbac::SessionStore;

/// Intervalo de limpieza en segundos (5 minutos).
const PURGE_INTERVAL_SECS: u64 = 300;

/// Lanza un hilo de fondo que purga las sesiones expiradas cada
/// `PURGE_INTERVAL_SECS` segundos. El hilo termina cuando el
/// `Arc<Mutex<SessionStore>>` se libera (apagado de la app).
pub fn spawn_session_cleanup(sessions: Arc<Mutex<SessionStore>>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(PURGE_INTERVAL_SECS));
        if let Ok(mut store) = sessions.lock() {
            let purged = store.purge_expired();
            if purged > 0 {
                log::info!(
                    "Sesiones expiradas purgadas: {} (activas: {})",
                    purged,
                    store.len()
                );
            }
        }
    });
}
