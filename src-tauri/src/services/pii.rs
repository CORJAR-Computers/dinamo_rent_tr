//! services/pii.rs — Gestión de la clave PII (db_encryption_key)
//!
//! Analiza los datos legacy cifrados con Fernet en la tabla clientes,
//! permite probar una clave candidata sin guardarla y persistirla en
//! config.ini + estado en caliente (AppState.pii_key).
//!
//! Columnas PII de clientes (mismo set que services/cliente.rs):
//!   celular, celular2, email, dir_residencia, dir_temporal, no_licencia

use serde::Serialize;

use crate::core::crypto::{is_fernet_token, PiiCipher};
use crate::core::error::AppError;
use crate::core::PooledConnection;
use crate::repositories::cliente::ClienteRepository;
use crate::services::AppState;

/// Conteo de clientes según su estado de descifrado
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiiAnalisis {
    /// ¿Hay clave configurada (no vacía)?
    pub clave_configurada: bool,
    /// Total de clientes en la tabla
    pub total_clientes: i64,
    /// Clientes con al menos un token Fernet legacy
    pub clientes_legacy: i64,
    /// Clientes legacy descifrables con la clave usada en el análisis
    pub clientes_descifrados: i64,
    /// Clientes legacy que siguen ocultos (clave no coincide o ausente)
    pub clientes_ocultos: i64,
    /// Muestra enmascarada de un valor descifrado (celular/email) para confirmar la clave
    pub muestra: Option<MuestraDescifrada>,
}

/// Ejemplo enmascarado de un valor PII descifrado correctamente
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MuestraDescifrada {
    pub cliente: String,
    pub campo: String,
    pub valor: String,
}

/// Resultado de persistir la clave (para el frontend)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaveGuardada {
    pub clave_configurada: bool,
    pub analisis: PiiAnalisis,
}

/// Enmascara un valor sensible: primeros 3 + *** + últimos 2 (si es largo)
fn enmascarar(v: &str) -> String {
    let chars: Vec<char> = v.chars().collect();
    if chars.len() <= 6 {
        return "••••••".into();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 2..].iter().collect();
    format!("{head}••••{tail}")
}

/// Analiza los clientes con una clave dada (no modifica estado)
fn analizar(conn: &mut PooledConnection, clave: &str) -> Result<PiiAnalisis, AppError> {
    let clientes = ClienteRepository::obtener_todos(conn)?;
    let cipher = PiiCipher::new(clave);
    let mut analisis = PiiAnalisis {
        clave_configurada: !clave.trim().is_empty(),
        total_clientes: clientes.len() as i64,
        ..Default::default()
    };

    for c in &clientes {
        // ¿Tiene al menos un token Fernet?
        let campos: [(&str, &Option<String>); 6] = [
            ("celular", &c.celular),
            ("celular2", &c.celular2),
            ("email", &c.email),
            ("dir_residencia", &c.dir_residencia),
            ("dir_temporal", &c.dir_temporal),
            ("no_licencia", &c.no_licencia),
        ];
        let tiene_legacy = campos.iter().any(|(_, v)| match v {
            Some(s) => is_fernet_token(s),
            None => false,
        });
        if !tiene_legacy {
            continue;
        }
        analisis.clientes_legacy += 1;

        // ¿Algún token descifra con esta clave?
        let mut descifra = false;
        for (campo, v) in &campos {
            if let Some(s) = v {
                if is_fernet_token(s) {
                    if let Ok(claro) = cipher.decrypt(s) {
                        descifra = true;
                        if analisis.muestra.is_none() && !claro.trim().is_empty() {
                            analisis.muestra = Some(MuestraDescifrada {
                                cliente: c.nombre_completo.clone(),
                                campo: (*campo).to_string(),
                                valor: enmascarar(&claro),
                            });
                        }
                    } else {
                        log::warn!(
                            "No se pudo descifrar el token PII del cliente id={} (campo {})",
                            c.id,
                            campo
                        );
                    }
                }
            }
        }
        if descifra {
            analisis.clientes_descifrados += 1;
        } else {
            analisis.clientes_ocultos += 1;
        }
    }
    Ok(analisis)
}

pub struct PiiService;

impl PiiService {
    /// Estado actual con la clave efectiva (config.ini o override de la UI)
    pub fn estado(state: &AppState) -> Result<PiiAnalisis, AppError> {
        let mut conn = state.pool.get()?;
        let clave = state.pii_key();
        analizar(&mut conn, &clave)
    }

    /// Prueba una clave candidata SIN guardarla (el frontend la muestra antes)
    pub fn probar_clave(state: &AppState, clave: &str) -> Result<PiiAnalisis, AppError> {
        let mut conn = state.pool.get()?;
        analizar(&mut conn, clave)
    }

    /// Persiste la clave en config.ini, actualiza el estado en caliente y
    /// devuelve el análisis resultante.
    pub fn guardar_clave(state: &AppState, clave: &str) -> Result<ClaveGuardada, AppError> {
        if clave.trim().is_empty() {
            return Err(AppError::Validation(
                "La clave no puede estar vacía. Usa eliminar para quitarla.".into(),
            ));
        }
        state.config.persist_db_encryption_key(clave)?;
        *state.pii_key.lock().unwrap_or_else(|p| p.into_inner()) = clave.trim().to_string();
        let mut conn = state.pool.get()?;
        let analisis = analizar(&mut conn, clave)?;
        Ok(ClaveGuardada {
            clave_configurada: true,
            analisis,
        })
    }

    /// Quita la clave (config.ini + estado en caliente). Los datos Fernet
    /// vuelven a quedar ocultos hasta que se configure una clave válida.
    pub fn eliminar_clave(state: &AppState) -> Result<ClaveGuardada, AppError> {
        state.config.persist_db_encryption_key("")?;
        *state.pii_key.lock().unwrap_or_else(|p| p.into_inner()) = String::new();
        let mut conn = state.pool.get()?;
        let analisis = analizar(&mut conn, "")?;
        Ok(ClaveGuardada {
            clave_configurada: false,
            analisis,
        })
    }
}
