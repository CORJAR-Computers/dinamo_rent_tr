//! rbac.rs — Sesiones y control de acceso (puerto de core/rbac.py + SessionManager)
//!
//! El `SessionStore` vive en `tauri::State` y mantiene las sesiones en memoria
//! (mismo modelo que la app Python: single-user desktop).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionData {
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub nombre: String,
    pub debe_cambiar_password: bool,
    #[serde(skip)]
    pub last_activity: Instant,
}

/// Almacén de sesiones en memoria con timeout por inactividad
#[derive(Default)]
pub struct SessionStore {
    sessions: HashMap<String, SessionData>,
    timeout: Duration,
}

impl SessionStore {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            sessions: HashMap::new(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Crea una sesión y devuelve el token
    pub fn create(
        &mut self,
        user_id: i64,
        username: &str,
        role: &str,
        nombre: &str,
        debe_cambiar_password: bool,
    ) -> String {
        let token = crate::core::security::generate_secure_token(32);
        self.sessions.insert(
            token.clone(),
            SessionData {
                user_id,
                username: username.to_string(),
                role: role.to_string(),
                nombre: nombre.to_string(),
                debe_cambiar_password,
                last_activity: Instant::now(),
            },
        );
        token
    }

    /// Valida una sesión y refresca last_activity. Devuelve error si expiró.
    pub fn validate(&mut self, token: &str) -> Result<SessionData, AppError> {
        let Some(data) = self.sessions.get_mut(token) else {
            return Err(AppError::SessionExpired);
        };
        if data.last_activity.elapsed() > self.timeout {
            self.sessions.remove(token);
            return Err(AppError::SessionExpired);
        }
        data.last_activity = Instant::now();
        Ok(data.clone())
    }

    /// Valida sesión y rol requerido
    pub fn validate_role(
        &mut self,
        token: &str,
        required_roles: &[&str],
    ) -> Result<SessionData, AppError> {
        let session = self.validate(token)?;
        if required_roles.is_empty() || required_roles.contains(&session.role.as_str()) {
            Ok(session)
        } else {
            Err(AppError::Permission)
        }
    }

    pub fn destroy(&mut self, token: &str) {
        self.sessions.remove(token);
    }

    /// Elimina sesiones expiradas; devuelve cuántas se purgaron
    pub fn purge_expired(&mut self) -> usize {
        let timeout = self.timeout;
        let before = self.sessions.len();
        self.sessions
            .retain(|_, s| s.last_activity.elapsed() <= timeout);
        before - self.sessions.len()
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

/// Helper de conveniencia: requiere sesión activa
pub fn require_active_session(
    store: &mut SessionStore,
    token: &str,
) -> Result<SessionData, AppError> {
    store.validate(token)
}

/// Helper de conveniencia: requiere rol
pub fn require_role(
    store: &mut SessionStore,
    token: &str,
    roles: &[&str],
) -> Result<SessionData, AppError> {
    store.validate_role(token, roles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SessionStore {
        SessionStore::new(3600)
    }

    #[test]
    fn create_and_validate() {
        let mut s = store();
        let token = s.create(1, "admin", "Administrador", "Admin", false);
        let data = s.validate(&token).unwrap();
        assert_eq!(data.username, "admin");
        assert_eq!(data.role, "Administrador");
    }

    #[test]
    fn invalid_token_fails() {
        let mut s = store();
        assert!(s.validate("no-existe").is_err());
    }

    #[test]
    fn role_check() {
        let mut s = store();
        let token = s.create(1, "juan", "Operador", "Juan", false);
        assert!(s.validate_role(&token, &["Administrador"]).is_err());
        assert!(s
            .validate_role(&token, &["Administrador", "Operador"])
            .is_ok());
        assert!(s.validate_role(&token, &[]).is_ok());
    }

    #[test]
    fn destroy() {
        let mut s = store();
        let token = s.create(1, "x", "Operador", "X", false);
        s.destroy(&token);
        assert!(s.validate(&token).is_err());
    }

    #[test]
    fn expiration() {
        let mut s = SessionStore::new(0); // timeout inmediato
        let token = s.create(1, "x", "Operador", "X", false);
        std::thread::sleep(Duration::from_millis(5));
        assert!(s.validate(&token).is_err());
    }
}
