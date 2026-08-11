//! crypto.rs — Cifrado de columnas PII (puerto de core/security_crypto.py)
//!
//! La app Python usaba Fernet. Aquí ciframos con AES-256-GCM (formato `v1:...`)
//! y mantenemos compatibilidad de LECTURA con tokens Fernet legacy (`gAAAA...`)
//! usando la misma `db_encryption_key` de config.ini.
//!
//! Formatos soportados por `PiiCipher::decrypt`:
//!   - `v1:{nonce_b64}:{ct_b64}`  → AES-256-GCM (nuevo)
//!   - `gAAAA...`                 → Fernet legacy (verifica HMAC + AES-128-CBC)
//!   - cualquier otro             → se devuelve tal cual (datos en claro)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use base64::engine::general_purpose::URL_SAFE as B64_URL;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URL_NOPAD;
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::error::AppError;

const V1_PREFIX: &str = "v1:";
/// Prefijo de los tokens Fernet (version byte 0x80 → base64 `gAAAA`)
const FERNET_PREFIX: &str = "gAAAA";
/// Límite de capas de cifrado anidadas que `PiiCipher::decrypt` des-envuelve.
/// Protege ante bucles infinitos por corrupción o tokens maliciosos.
const MAX_CAPAS_CIFRADO: usize = 8;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type HmacSha256 = Hmac<Sha256>;

/// Deriva la clave AES-256 (32 bytes) desde db_encryption_key (Fernet base64 de 44 chars u otra)
pub fn derive_key(db_encryption_key: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(db_encryption_key.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest[..32]);
    key
}

/// Cifra un valor con AES-256-GCM: devuelve `v1:{nonce_b64}:{ct_b64}`.
/// Valores vacíos pasan tal cual.
pub fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<String, AppError> {
    if plaintext.is_empty() {
        return Ok(plaintext.to_string());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| {
        AppError::Crypto(format!("Error creando cifrador AES: {e}"))
    })?;
    let mut nonce_bytes = [0u8; 12];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| AppError::Crypto(format!("Error cifrando: {e}")))?;
    Ok(format!(
        "{V1_PREFIX}{}:{}",
        B64.encode(nonce_bytes),
        B64.encode(ct)
    ))
}

/// Desencripta un valor cifrado con AES-256-GCM (formato v1:).
/// Si no tiene prefijo v1:, devuelve el valor tal cual (datos en claro).
pub fn decrypt(key: &[u8; 32], stored: &str) -> Result<String, AppError> {
    if stored.is_empty() {
        return Ok(stored.to_string());
    }
    let rest = match stored.strip_prefix(V1_PREFIX) {
        Some(r) => r,
        None => return Ok(stored.to_string()),
    };
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(AppError::Crypto("Formato de dato cifrado inválido".into()));
    }
    let nonce_bytes = B64
        .decode(parts[0])
        .map_err(|e| AppError::Crypto(format!("Nonce inválido: {e}")))?;
    let ct = B64
        .decode(parts[1])
        .map_err(|e| AppError::Crypto(format!("Ciphertext inválido: {e}")))?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| {
        AppError::Crypto(format!("Error creando descifrador AES: {e}"))
    })?;
    let pt = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ct.as_ref())
        .map_err(|_| AppError::Crypto("No se pudo desencriptar (¿clave incorrecta?)".into()))?;
    String::from_utf8(pt).map_err(|e| AppError::Crypto(format!("Dato desencriptado no UTF-8: {e}")))
}

// ═══════════════════════════════════════════════════════════════════════════
// FERNET LEGACY (compatibilidad de lectura con la app Python)
// ═══════════════════════════════════════════════════════════════════════════

/// Desencripta un token Fernet legacy.
///
/// `fernet_key_b64` es la `db_encryption_key` de config.ini (base64 de 32 bytes:
/// los primeros 16 bytes son la clave de firma HMAC, los últimos 16 la AES-128).
///
/// Estructura del token (tras base64url):
///   [0x80] [timestamp 8B] [IV 16B] [ciphertext] [HMAC-SHA256 32B]
/// Decodifica base64url tolerando padding opcional (Python Fernet incluye '=')
fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    B64_URL.decode(s.trim()).or_else(|_| B64_URL_NOPAD.decode(s.trim())).ok()
}

pub fn fernet_decrypt(fernet_key_b64: &str, token: &str) -> Option<String> {
    let raw_key = b64url_decode(fernet_key_b64)?;
    if raw_key.len() != 32 {
        return None;
    }
    let token_bytes = b64url_decode(token)?;
    if token_bytes.len() < 57 {
        return None; // 1 + 8 + 16 + 16 + 32 mínimo (1 bloque ct + hmac)
    }
    if token_bytes[0] != 0x80 {
        return None;
    }
    let (header_ct, signature) = token_bytes.split_at(token_bytes.len() - 32);

    // 1) Verificar HMAC-SHA256 (clave de firma = primeros 16 bytes)
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&raw_key[..16]).ok()?;
    mac.update(header_ct);
    let computed = mac.finalize().into_bytes();
    if !bool::from(computed.as_slice().ct_eq(signature)) {
        return None;
    }

    // 2) Descifrar AES-128-CBC (clave = últimos 16 bytes, IV = 16 bytes tras timestamp)
    let iv: [u8; 16] = header_ct[9..25].try_into().ok()?;
    let ct = &header_ct[25..];
    let key: [u8; 16] = raw_key[16..32].try_into().ok()?;
    let mut buf = ct.to_vec();
    let pt = Aes128CbcDec::new_from_slices(&key, &iv)
        .ok()?
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .ok()?;
    String::from_utf8(pt.to_vec()).ok()
}

/// ¿El valor tiene formato de token Fernet legacy?
pub fn is_fernet_token(stored: &str) -> bool {
    stored.starts_with(FERNET_PREFIX)
}

// ═══════════════════════════════════════════════════════════════════════════
// PiiCipher — cifrador unificado de columnas PII
// ═══════════════════════════════════════════════════════════════════════════

/// Cifrador de columnas PII: escribe AES-256-GCM (v1:) y lee los tres formatos.
pub struct PiiCipher {
    aes_key: [u8; 32],
    fernet_key: Option<String>,
}

impl PiiCipher {
    /// Crea el cifrador a partir de `db_encryption_key` de config.ini.
    pub fn new(db_encryption_key: &str) -> Self {
        let aes_key = derive_key(db_encryption_key);
        let fernet_key = if db_encryption_key.trim().is_empty() {
            None
        } else {
            Some(db_encryption_key.trim().to_string())
        };
        Self { aes_key, fernet_key }
    }

    /// Cifra un valor PII (AES-256-GCM). Vacío pasa tal cual.
    ///
    /// Si `db_encryption_key` está vacía, la clave se deriva de un valor fijo:
    /// se advierte en el log porque cambiar la clave después haría ilegibles
    /// los datos cifrados con la clave derivada.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, AppError> {
        if self.fernet_key.is_none() && !plaintext.trim().is_empty() {
            log::warn!(
                "Cifrando PII con clave derivada por defecto: configura db_encryption_key en config.ini para evitar pérdida de datos al cambiarla."
            );
        }
        encrypt(&self.aes_key, plaintext)
    }

    /// Desencripta un valor PII en cualquiera de los formatos soportados.
    ///
    /// Des-envuelve de forma iterativa todas las capas de cifrado (v1: y Fernet)
    /// hasta obtener el texto en claro, con un límite de profundidad por seguridad.
    /// Esto protege la vista ante cifrados anidados accidentales (por ejemplo, la
    /// rotación de clave con un binario defectuoso que re-cifraba tokens ya cifrados).
    ///
    /// Para tokens Fernet legacy sin clave configurada (o con clave inválida)
    /// devuelve `Err(AppError::Crypto)`, y el servicio decide cómo mostrarlo.
    pub fn decrypt(&self, stored: &str) -> Result<String, AppError> {
        let mut actual = stored.to_string();
        let mut capas = 0usize;
        loop {
            if actual.starts_with(V1_PREFIX) {
                actual = decrypt(&self.aes_key, &actual)?;
                capas += 1;
            } else if is_fernet_token(&actual) {
                let claro = match self.fernet_key.as_deref() {
                    Some(key) => fernet_decrypt(key, &actual).ok_or_else(|| {
                        AppError::Crypto(
                            "Dato Fernet legacy no desencriptable (clave de config.ini inválida)"
                                .into(),
                        )
                    })?,
                    None => {
                        return Err(AppError::Crypto(
                            "Datos PII legacy cifrados (Fernet): se requiere db_encryption_key en config.ini"
                                .into(),
                        ))
                    }
                };
                actual = claro;
                capas += 1;
            } else {
                return Ok(actual); // dato en claro
            }
            if capas > MAX_CAPAS_CIFRADO {
                return Err(AppError::Crypto(
                    "Exceso de capas de cifrado anidadas (posible corrupción)".into(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let key = derive_key("REDACTED_OLD_KEY=");
        let ct = encrypt(&key, "3101234567").unwrap();
        assert!(ct.starts_with(V1_PREFIX));
        assert_eq!(decrypt(&key, &ct).unwrap(), "3101234567");
    }

    #[test]
    fn plaintext_passthrough() {
        let key = derive_key("test");
        assert_eq!(decrypt(&key, "").unwrap(), "");
        // Valor legacy sin cifrar → se devuelve tal cual
        assert_eq!(decrypt(&key, "valor-en-claro").unwrap(), "valor-en-claro");
    }

    #[test]
    fn key_derivation_stable() {
        let k1 = derive_key("misma-clave");
        let k2 = derive_key("misma-clave");
        assert_eq!(k1, k2);
    }

    #[test]
    fn fernet_legacy_roundtrip() {
        // Vector generado con Python cryptography (ver historia del repo):
        let key = "KP6_mu0mUEsqMcNjz6Z2HaaqCTfMNE0zTZNC-9GBF9s=";
        let token = "gAAAAABqdgoctcLf9K9H-r2OLdNCk65oVGpCNvwzy1OCK_CwkJfMlOs9kfKy-JWxaio5ZOzClYSwyzUeuX0S8HQl2otr-9O-og==";
        assert_eq!(fernet_decrypt(key, token).as_deref(), Some("3101234567"));
        // Token manipulado → None
        let mut tampered = token.to_string();
        tampered.replace_range(40..41, "x");
        assert_eq!(fernet_decrypt(key, &tampered), None);
    }

    #[test]
    fn pii_cipher_decrypts_legacy_and_v1() {
        let cipher = PiiCipher::new("KP6_mu0mUEsqMcNjz6Z2HaaqCTfMNE0zTZNC-9GBF9s=");
        let token = "gAAAAABqdgoctcLf9K9H-r2OLdNCk65oVGpCNvwzy1OCK_CwkJfMlOs9kfKy-JWxaio5ZOzClYSwyzUeuX0S8HQl2otr-9O-og==";
        assert_eq!(cipher.decrypt(token).unwrap(), "3101234567");
        // Escritura nueva en v1:
        let ct = cipher.encrypt("3217654321").unwrap();
        assert!(ct.starts_with(V1_PREFIX));
        assert_eq!(cipher.decrypt(&ct).unwrap(), "3217654321");
        // En claro:
        assert_eq!(cipher.decrypt("texto-en-claro").unwrap(), "texto-en-claro");
    }

    #[test]
    fn pii_cipher_sin_clave_rechaza_fernet() {
        let cipher = PiiCipher::new("");
        let token = "gAAAAABqdgoctcLf9K9H-r2OLdNCk65oVGpCNvwzy1OCK_CwkJfMlOs9kfKy-JWxaio5ZOzClYSwyzUeuX0S8HQl2otr-9O-og==";
        assert!(cipher.decrypt(token).is_err());
    }

    #[test]
    fn pii_cipher_desenvuelve_doble_cifrado_v1() {
        // Un valor cifrado dos veces con la misma clave (regresión del bin de
        // rotación viejo) debe des-envolverse hasta el texto en claro.
        let cipher = PiiCipher::new("clave-de-prueba-para-doble-cifrado");
        let claro = "3101234567";
        let una_capa = cipher.encrypt(claro).unwrap();
        let doble = cipher.encrypt(&una_capa).unwrap();
        assert!(doble.starts_with(V1_PREFIX));
        assert_eq!(cipher.decrypt(&doble).unwrap(), claro);
    }

    #[test]
    fn pii_cipher_desenvuelve_tres_capas_v1() {
        let cipher = PiiCipher::new("otra-clave-de-prueba");
        let claro = "hola@ejemplo.com";
        let mut tok = cipher.encrypt(claro).unwrap();
        for _ in 0..2 {
            tok = cipher.encrypt(&tok).unwrap();
        }
        assert_eq!(cipher.decrypt(&tok).unwrap(), claro);
    }

    #[test]
    fn pii_cipher_desenvuelve_fernet_tras_v1() {
        // Capa externa v1: + capa interna Fernet legacy (misma clave)
        let key = "KP6_mu0mUEsqMcNjz6Z2HaaqCTfMNE0zTZNC-9GBF9s=";
        let token = "gAAAAABqdgoctcLf9K9H-r2OLdNCk65oVGpCNvwzy1OCK_CwkJfMlOs9kfKy-JWxaio5ZOzClYSwyzUeuX0S8HQl2otr-9O-og==";
        let cipher = PiiCipher::new(key);
        let envuelto = cipher.encrypt(token).unwrap();
        assert_eq!(cipher.decrypt(&envuelto).unwrap(), "3101234567");
    }

    #[test]
    fn pii_cipher_acepta_hasta_el_limite_de_capas() {
        let cipher = PiiCipher::new("clave-limite-capas");
        let claro = "12345";
        let mut tok = cipher.encrypt(claro).unwrap();
        // Total de capas = MAX_CAPAS_CIFRADO (8): debe des-envolverse sin error
        for _ in 0..(MAX_CAPAS_CIFRADO - 1) {
            tok = cipher.encrypt(&tok).unwrap();
        }
        assert_eq!(cipher.decrypt(&tok).unwrap(), claro);
    }

    #[test]
    fn pii_cipher_rechaza_exceso_de_capas() {
        let cipher = PiiCipher::new("clave-limite-capas");
        let claro = "12345";
        let mut tok = cipher.encrypt(claro).unwrap();
        // Una capa más del límite (9 en total): debe devolver Err (protección anti-bucle)
        for _ in 0..MAX_CAPAS_CIFRADO {
            tok = cipher.encrypt(&tok).unwrap();
        }
        assert!(cipher.decrypt(&tok).is_err());
    }
}
