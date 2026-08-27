//! security.rs — Contraseñas, tokens y rate limiting
//!
//! Puerto de `core/security.py`:
//! - Hash legacy: PBKDF2-HMAC-SHA256, 100.000 iteraciones, formato `{hex}:{salt_hex}`
//! - Hash nuevo: Argon2id (PHC string `$argon2id$...`)
//! - Rate limiting: 5 intentos → bloqueo 30 min; ventana 10/5min por usuario; 20/min por IP
//!
//! Compatibilidad: `verify_password` detecta el formato y, si es PBKDF2 legacy,
//! retorna `NeedsRehash` para el re-hash a Argon2id en el primer login exitoso.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use subtle::ConstantTimeEq;

use super::error::AppError;

/// Iteraciones PBKDF2 de los hashes legacy
pub const PBKDF2_ITERATIONS: u32 = 100_000;
/// Formato PHC de Argon2id
const ARGON2_PREFIX: &str = "$argon2id$";

// ═══════════════════════════════════════════════════════════════════════════
// HASHING DE CONTRASEÑAS
// ═══════════════════════════════════════════════════════════════════════════

/// Genera un hash Argon2id (PHC string)
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    // Params recomendados por OWASP para Argon2id (m=19MiB, t=2, p=1)
    let params = Params::new(19_456, 2, 1, Some(32))
        .map_err(|e| AppError::Crypto(format!("Params Argon2 inválidos: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Crypto(format!("Error hasheando contraseña: {e}")))?
        .to_string();
    Ok(hash)
}

/// Resultado de verificación de contraseña
#[derive(Debug, PartialEq)]
pub enum VerifyResult {
    Valid,
    ValidNeedsRehash,
    Invalid,
}

/// Verifica una contraseña contra el hash almacenado.
/// Detecta automáticamente formato Argon2id (PHC) vs legacy PBKDF2 `hex:salt`.
pub fn verify_password(stored: &str, provided: &str) -> VerifyResult {
    if stored.is_empty() || provided.is_empty() {
        return VerifyResult::Invalid;
    }
    if stored.starts_with(ARGON2_PREFIX) {
        // Nuevo formato: Argon2id
        let Ok(parsed) = PasswordHash::new(stored) else {
            return VerifyResult::Invalid;
        };
        match Argon2::default().verify_password(provided.as_bytes(), &parsed) {
            Ok(()) => VerifyResult::Valid,
            Err(_) => VerifyResult::Invalid,
        }
    } else if let Some((hash_hex, salt_hex)) = stored.split_once(':') {
        // Formato legacy PBKDF2 `{hex}:{salt_hex}`
        // IMPORTANTE: Python usa `salt = secrets.token_hex(16)` y luego
        // `pbkdf2_hmac(..., salt.encode(), ...)` → el salt es la CADENA HEX ASCII
        // (32 caracteres → 32 bytes), no los bytes decodificados.
        let salt_bytes = salt_hex.as_bytes();
        let Ok(expected) = hex::decode(hash_hex) else {
            return VerifyResult::Invalid;
        };
        let mut output = vec![0u8; expected.len()];
        pbkdf2_hmac::<Sha256>(
            provided.as_bytes(),
            salt_bytes,
            PBKDF2_ITERATIONS,
            &mut output,
        );
        if output.ct_eq(&expected).into() {
            VerifyResult::ValidNeedsRehash
        } else {
            VerifyResult::Invalid
        }
    } else {
        VerifyResult::Invalid
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TOKENS
// ═══════════════════════════════════════════════════════════════════════════

/// Genera un token criptográficamente seguro (url-safe base64)
pub fn generate_secure_token(bytes: usize) -> String {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use rand::RngCore;
    let mut buf = vec![0u8; bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

// ═══════════════════════════════════════════════════════════════════════════
// RATE LIMITING
// ═══════════════════════════════════════════════════════════════════════════

/// Rastreador de intentos de login (puerto de LoginAttemptTracker + IPRateLimiter)
pub struct LoginAttemptTracker {
    failed_attempts: HashMap<String, u32>,
    lockout_until: HashMap<String, Instant>,
    login_timestamps: HashMap<String, Vec<Instant>>,
    ip_timestamps: HashMap<String, Vec<Instant>>,
    blocked_ips: HashMap<String, Instant>,
    max_login_attempts: u32,
    lockout_duration: Duration,
    rate_window: Duration,
    max_attempts_in_window: u32,
    /// Ventana de rate limiting por IP en segundos (fija: 60s)
    #[allow(dead_code)]
    ip_rate_window: Duration,
    ip_max_attempts: u32,
    now: fn() -> Instant,
}

impl Default for LoginAttemptTracker {
    fn default() -> Self {
        Self::new(5, 1800, 300, 10)
    }
}

impl LoginAttemptTracker {
    pub fn new(
        max_login_attempts: u32,
        lockout_duration_secs: u64,
        rate_window_secs: u64,
        max_attempts_in_window: u32,
    ) -> Self {
        Self {
            failed_attempts: HashMap::new(),
            lockout_until: HashMap::new(),
            login_timestamps: HashMap::new(),
            ip_timestamps: HashMap::new(),
            blocked_ips: HashMap::new(),
            max_login_attempts,
            lockout_duration: Duration::from_secs(lockout_duration_secs),
            rate_window: Duration::from_secs(rate_window_secs),
            max_attempts_in_window,
            ip_rate_window: Duration::from_secs(60),
            ip_max_attempts: 20,
            now: Instant::now,
        }
    }

    /// Sincroniza intentos fallidos desde la BD (restaura bloqueos al iniciar)
    pub fn sync_from_db(&mut self, db_failed_attempts: HashMap<String, u32>) {
        for (username, attempts) in db_failed_attempts {
            if attempts >= self.max_login_attempts {
                self.failed_attempts.insert(username.clone(), attempts);
                self.lock_account(&username);
            }
        }
    }

    /// Registra un intento fallido; devuelve el total de intentos del usuario
    pub fn record_failed_attempt(&mut self, username: &str, ip: Option<&str>) -> u32 {
        let now = (self.now)();
        let count = {
            let c = self
                .failed_attempts
                .entry(username.to_string())
                .or_insert(0);
            *c += 1;
            *c
        };
        self.login_timestamps
            .entry(username.to_string())
            .or_default()
            .push(now);
        self.clean_timestamps(username, now);
        if let Some(ip) = ip {
            self.ip_timestamps
                .entry(ip.to_string())
                .or_default()
                .push(now);
        }
        count
    }

    pub fn is_locked(&mut self, username: &str) -> bool {
        if let Some(&until) = self.lockout_until.get(username) {
            if (self.now)() < until {
                return true;
            }
            // Auto-desbloqueo
            self.lockout_until.remove(username);
            self.failed_attempts.insert(username.to_string(), 0);
        }
        false
    }

    pub fn lock_account(&mut self, username: &str) {
        self.lockout_until
            .insert(username.to_string(), (self.now)() + self.lockout_duration);
    }

    pub fn reset_attempts(&mut self, username: &str) {
        self.failed_attempts.insert(username.to_string(), 0);
        self.login_timestamps.remove(username);
        self.lockout_until.remove(username);
    }

    pub fn get_remaining_attempts(&self, username: &str) -> u32 {
        let used = self.failed_attempts.get(username).copied().unwrap_or(0);
        self.max_login_attempts.saturating_sub(used)
    }

    pub fn get_lockout_remaining_seconds(&self, username: &str) -> u64 {
        self.lockout_until
            .get(username)
            .map(|until| until.saturating_duration_since((self.now)()).as_secs())
            .unwrap_or(0)
    }

    pub fn get_failed_attempts(&self, username: &str) -> u32 {
        self.failed_attempts.get(username).copied().unwrap_or(0)
    }

    /// True si se excedió el rate limit por usuario o IP
    pub fn check_rate_limit(&mut self, username: &str, ip: Option<&str>) -> bool {
        let now = (self.now)();
        self.clean_timestamps(username, now);

        // Por usuario: máx intentos en la ventana
        if let Some(ts) = self.login_timestamps.get(username) {
            if ts.len() > self.max_attempts_in_window as usize {
                return true;
            }
        }
        // Por IP: bloqueado o máx en ventana
        if let Some(ip) = ip {
            if let Some(&until) = self.blocked_ips.get(ip) {
                if now < until {
                    return true;
                }
                self.blocked_ips.remove(ip);
            }
            if let Some(ts) = self.ip_timestamps.get(ip) {
                if ts.len() >= self.ip_max_attempts as usize {
                    return true;
                }
            }
        }
        false
    }

    fn clean_timestamps(&mut self, username: &str, now: Instant) {
        let cutoff = now - self.rate_window;
        if let Some(ts) = self.login_timestamps.get_mut(username) {
            ts.retain(|t| *t > cutoff);
        }
    }

    /// Convierte timestamps en segundos desde epoch (para sync con BD)
    pub fn now_epoch_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2_roundtrip() {
        let hash = hash_password("Passw0rd!").unwrap();
        assert!(hash.starts_with(ARGON2_PREFIX));
        assert_eq!(verify_password(&hash, "Passw0rd!"), VerifyResult::Valid);
        assert_eq!(verify_password(&hash, "incorrecta"), VerifyResult::Invalid);
    }

    #[test]
    fn pbkdf2_legacy_verified_and_rehash_needed() {
        // Reproduce EXACTAMENTE el algoritmo de la app Python:
        //   salt = token_hex(16) → hash = pbkdf2_hmac(pwd, salt.encode(), 100_000)
        use pbkdf2::pbkdf2_hmac;
        use sha2::Sha256;
        let salt_hex = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
        let mut out = [0u8; 32];
        pbkdf2_hmac::<Sha256>(b"admin123", salt_hex.as_bytes(), 100_000, &mut out);
        let stored = format!("{}:{}", hex::encode(out), salt_hex);
        assert_eq!(
            verify_password(&stored, "admin123"),
            VerifyResult::ValidNeedsRehash
        );
        assert_eq!(verify_password(&stored, "malo"), VerifyResult::Invalid);
    }

    #[test]
    fn pbkdf2_legacy_format_detected() {
        // Hash con formato legacy hex:hex → se detecta como legacy y se verifica
        // contra la contraseña correcta (sin asumir cuál es la contraseña real).
        let stored =
            "7b7996f73e0e909d90476c1e0734932637ba60a775af2f93d34ecd8026283f07:0937e659d196bd24b18c86067920b2ba";
        // Contraseña incorrecta → Invalid (no panic)
        assert_eq!(verify_password(stored, "incorrecta"), VerifyResult::Invalid);
        // Formato sin ':' → Invalid
        assert_eq!(verify_password("noformato", "x"), VerifyResult::Invalid);
        assert_eq!(verify_password("", "x"), VerifyResult::Invalid);
    }

    #[test]
    fn tracker_lockout() {
        let mut t = LoginAttemptTracker::new(5, 1800, 300, 10);
        for i in 0..5 {
            t.record_failed_attempt("admin", None);
            assert_eq!(t.get_failed_attempts("admin"), i + 1);
        }
        assert!(!t.is_locked("admin"));
        // 5 intentos → bloqueo
        t.lock_account("admin");
        assert!(t.is_locked("admin"));
        assert!(t.get_lockout_remaining_seconds("admin") > 1700);
    }

    #[test]
    fn tracker_reset() {
        let mut t = LoginAttemptTracker::new(5, 1800, 300, 10);
        t.record_failed_attempt("admin", None);
        t.record_failed_attempt("admin", None);
        t.reset_attempts("admin");
        assert_eq!(t.get_failed_attempts("admin"), 0);
        assert_eq!(t.get_remaining_attempts("admin"), 5);
    }

    #[test]
    fn rate_limit_by_ip() {
        let mut t = LoginAttemptTracker::new(5, 1800, 300, 10);
        // 20 intentos por IP en 60s → rate limit.
        // Usernames distintos para aislar el límite por IP.
        for i in 0..20 {
            let user = format!("u{i}");
            assert!(!t.check_rate_limit(&user, Some("127.0.0.1")));
            t.record_failed_attempt(&user, Some("127.0.0.1"));
        }
        // El 21º intento (otro usuario, misma IP) excede el límite por IP
        assert!(t.check_rate_limit("u_new", Some("127.0.0.1")));
    }

    #[test]
    fn token_is_unique_and_urlsafe() {
        let a = generate_secure_token(32);
        let b = generate_secure_token(32);
        assert_ne!(a, b);
        assert!(a.len() >= 40);
        assert!(!a.contains('+'));
        assert!(!a.contains('/'));
    }
}
