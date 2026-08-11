//! services/rotacion.rs — Rotación de la clave PII (db_encryption_key)
//!
//! Lógica central del bin `rotate_pii_key` (SECURITY.md §2.1): descifra las
//! columnas PII de `clientes` con la clave VIEJA (tokens Fernet legacy y
//! AES-GCM `v1:`) y las re-cifra con la clave NUEVA en una transacción
//! atómica, registrando el evento de auditoría `PII_KEY_ROTATED` sin exponer
//! la clave. También lo ejercita el test de integración
//! `tests/rotacion_integration.rs` sobre una copia temporal de la BD.

use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

use crate::core::crypto::{is_fernet_token, PiiCipher};
use crate::core::error::AppError;
use crate::core::PooledConnection;
use crate::repositories::cliente::{ClienteRow, SELECT_COLS};

/// Parámetros posicionales de cualquier longitud (tuplas `IntoParams`
/// limitadas a 15 elementos en rsfbclient).
macro_rules! params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into_param()),+])
    };
}

/// Columnas PII de `clientes` (mismo set que services/cliente.rs)
const PII_COLUMNS: [&str; 6] = [
    "CELULAR",
    "CELULAR2",
    "EMAIL",
    "DIR_RESIDENCIA",
    "DIR_TEMPORAL",
    "NO_LICENCIA",
];

/// Resultado de una rotación de clave
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultadoRotacion {
    /// Clientes procesados
    pub clientes: usize,
    /// Tokens Fernet legacy re-cifrados
    pub fernet: usize,
    /// Tokens AES-GCM v1: re-cifrados
    pub aes_v1: usize,
}

/// Re-cifra un valor PII de la clave vieja a la nueva.
/// Devuelve `None` si el valor era NULL/vacío. Los valores en claro se cifran
/// (una clave vieja nunca debe dejar PII sin cifrar).
///
/// Desencripta con la clave VIEJA tanto tokens Fernet legacy como AES-GCM
/// `v1:` (`PiiCipher::decrypt` soporta ambos formatos): si un token `v1:` se
/// tratara como texto plano se re-cifraría DOBLE (corrompiendo el dato y
/// desbordando la columna).
fn reencryptar(
    cipher_viejo: &PiiCipher,
    cipher_nuevo: &PiiCipher,
    valor: &str,
) -> Result<String, AppError> {
    let v = valor.trim();
    if v.is_empty() {
        return Ok(String::new());
    }
    if is_fernet_token(v) || v.starts_with("v1:") {
        let claro = cipher_viejo.decrypt(v).map_err(|e| {
            AppError::Crypto(format!("Token no descifrable con la clave vieja: {e}"))
        })?;
        cipher_nuevo
            .encrypt(&claro)
            .map_err(|e| AppError::Crypto(format!("Error re-cifrando con clave nueva: {e}")))
    } else {
        cipher_nuevo
            .encrypt(v)
            .map_err(|e| AppError::Crypto(format!("Error cifrando valor en claro: {e}")))
    }
}

/// Rota la clave PII de la tabla `clientes`: descifra cada columna con
/// `old_key` y la re-cifra con `new_key` (AES-256-GCM `v1:`) en UNA
/// transacción atómica, registrando el evento de auditoría `PII_KEY_ROTATED`
/// (usuario `sistema`, ip `local`) **sin exponer la clave** en el mensaje.
///
/// Pre-valida TODAS las filas antes de escribir: si algún token no se
/// descifra con `old_key`, devuelve `Err` y no se escribe nada.
pub fn rotar_clave_pii(
    conn: &mut PooledConnection,
    old_key: &str,
    new_key: &str,
) -> Result<ResultadoRotacion, AppError> {
    if old_key.trim().is_empty() || new_key.trim().is_empty() {
        return Err(AppError::Validation(
            "Las claves no pueden estar vacías".into(),
        ));
    }

    let cipher_viejo = PiiCipher::new(old_key);
    let cipher_nuevo = PiiCipher::new(new_key);

    let rows: Vec<ClienteRow> = conn.query(
        &format!("SELECT {SELECT_COLS} FROM clientes ORDER BY id"),
        (),
    )?;

    // Pre-validar TODAS antes de escribir (aborta si la clave vieja falla).
    // Índices PII en ClienteRow: 6=celular, 7=celular2, 8=email,
    // 13=dir_residencia, 14=dir_temporal, 17=no_licencia.
    let pii_idx: [usize; 6] = [6, 7, 8, 13, 14, 17];
    let mut plan: Vec<(i64, [Option<String>; 6])> = Vec::with_capacity(rows.len());
    let mut fernet = 0usize;
    let mut aes_v1 = 0usize;
    for row in &rows {
        let valores: [Option<&str>; 6] = std::array::from_fn(|i| {
            let idx = pii_idx[i];
            match idx {
                6 => row.6.as_deref(),
                7 => row.7.as_deref(),
                8 => row.8.as_deref(),
                13 => row.13.as_deref(),
                14 => row.14.as_deref(),
                17 => row.17.as_deref(),
                _ => unreachable!(),
            }
        });
        let mut nuevos: [Option<String>; 6] = std::array::from_fn(|_| None);
        for (i, v) in valores.into_iter().enumerate() {
            let Some(s) = v else { continue };
            if s.trim().is_empty() {
                continue;
            }
            if is_fernet_token(s) {
                fernet += 1;
            } else if s.starts_with("v1:") {
                aes_v1 += 1;
            }
            nuevos[i] = Some(reencryptar(&cipher_viejo, &cipher_nuevo, s)?);
        }
        plan.push((row.0, nuevos));
    }

    // Mensaje de auditoría: nunca incluir las claves ni datos PII.
    let mensaje_auditoria = format!(
        "Rotación de clave PII completada: {} clientes, {} tokens re-cifrados ({} Fernet legacy, {} AES-GCM v1:)",
        plan.len(),
        fernet + aes_v1,
        fernet,
        aes_v1
    );

    // Re-cifra en una sola transacción (atómico) + auditoría PII_KEY_ROTATED.
    let update = format!(
        "UPDATE clientes SET {}, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        PII_COLUMNS
            .iter()
            .map(|c| format!("{c} = ?"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    conn.with_transaction(|tx| -> Result<(), rsfbclient::FbError> {
        for (id, nuevos) in &plan {
            tx.execute(
                &update,
                params![
                    nuevos[0].clone(),
                    nuevos[1].clone(),
                    nuevos[2].clone(),
                    nuevos[3].clone(),
                    nuevos[4].clone(),
                    nuevos[5].clone(),
                    *id,
                ],
            )?;
        }
        tx.execute(
            "INSERT INTO auditoria (usuario, accion, mensaje, ip, fecha) \
             VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
            params![
                "sistema".to_string(),
                "PII_KEY_ROTATED".to_string(),
                mensaje_auditoria,
                "local".to_string(),
            ],
        )?;
        Ok(())
    })
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(ResultadoRotacion {
        clientes: plan.len(),
        fernet,
        aes_v1,
    })
}
