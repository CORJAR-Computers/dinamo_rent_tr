//! services/cliente.rs — Lógica de negocio de clientes (puerto de cliente_service.py)
//!
//! Aplica cifrado/descifrado PII (PiiCipher) sobre las columnas sensibles:
//! celular, celular2, email, dir_residencia, dir_temporal, no_licencia.
//! Si un dato legacy (Fernet) no puede descifrarse (clave ausente en config.ini),
//! se devuelve `None` para no exponer basura en la UI.

use std::sync::Arc;

use chrono::NaiveDate;
use serde::Serialize;

use crate::core::config::AppConfig;
use crate::core::crypto::PiiCipher;
use crate::core::error::AppError;
use crate::core::validators::validate_no_xss;
use crate::core::PooledConnection;
use crate::repositories::cliente::{Cliente, ClienteDatos, ClienteRepository};

/// Resultado de listado con metadatos de descifrado (para la UI)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClienteConPii {
    pub cliente: Cliente,
    /// true si alguna columna PII no pudo descifrarse (legacy sin clave)
    pub pii_oculto: bool,
}

pub struct ClienteService;

impl ClienteService {
    /// Lista clientes con filtros opcionales (búsqueda o estado)
    pub fn listar(
        conn: &mut PooledConnection,
        cipher: &PiiCipher,
        busqueda: Option<&str>,
        estado: Option<&str>,
    ) -> Result<Vec<ClienteConPii>, AppError> {
        let term = busqueda.unwrap_or("").trim();
        let raw = if !term.is_empty() {
            ClienteRepository::buscar(conn, term)?
        } else if let Some(estado) =
            estado.filter(|e| !e.trim().is_empty() && e.trim() != "Todos")
        {
            ClienteRepository::obtener_por_estado(conn, estado.trim())?
        } else {
            ClienteRepository::obtener_todos(conn)?
        };
        Ok(raw.into_iter().map(|c| descifrar(cipher, c)).collect())
    }

    /// Obtiene un cliente por id (PII descifrada)
    pub fn obtener(
        conn: &mut PooledConnection,
        cipher: &PiiCipher,
        id: i64,
    ) -> Result<ClienteConPii, AppError> {
        let c = ClienteRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe el cliente #{id}")))?;
        Ok(descifrar(cipher, c))
    }

    /// Crea un cliente validando documento único
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        cipher: &PiiCipher,
        mut datos: ClienteDatos,
    ) -> Result<ClienteConPii, AppError> {
        normalizar(&mut datos);
        validar(&datos, cfg)?;
        cifrar(cipher, &mut datos)?;
        let id = ClienteRepository::insertar(conn, &datos)?;
        let c = ClienteRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::Generic("No se pudo recuperar el cliente creado".into()))?;
        Ok(descifrar(cipher, c))
    }

    /// Actualiza un cliente por id
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        cipher: &PiiCipher,
        id: i64,
        mut datos: ClienteDatos,
    ) -> Result<ClienteConPii, AppError> {
        ClienteRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe el cliente #{id}")))?;
        normalizar(&mut datos);
        validar(&datos, cfg)?;
        cifrar(cipher, &mut datos)?;
        ClienteRepository::actualizar(conn, id, &datos)?;
        let c = ClienteRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::Generic("No se pudo recuperar el cliente actualizado".into()))?;
        Ok(descifrar(cipher, c))
    }

    /// Elimina un cliente por id
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        ClienteRepository::eliminar(conn, id)
    }

    /// Total de clientes (dashboard)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        ClienteRepository::contar(conn)
    }

    /// Últimos clientes creados (dashboard)
    pub fn recientes(
        conn: &mut PooledConnection,
        cipher: &PiiCipher,
        limit: i64,
    ) -> Result<Vec<ClienteConPii>, AppError> {
        let raw = ClienteRepository::recientes(conn, limit)?;
        Ok(raw.into_iter().map(|c| descifrar(cipher, c)).collect())
    }
}

/// Descifra las columnas PII de un cliente. Las no descifrables → None.
fn descifrar(cipher: &PiiCipher, mut c: Cliente) -> ClienteConPii {
    let mut pii_oculto = false;
    for campo in [
        &mut c.celular,
        &mut c.celular2,
        &mut c.email,
        &mut c.dir_residencia,
        &mut c.dir_temporal,
        &mut c.no_licencia,
    ] {
        if let Some(v) = campo {
            match cipher.decrypt(v) {
                Ok(claro) => *campo = Some(claro),
                Err(_) => {
                    pii_oculto = true;
                    *campo = None;
                }
            }
        }
    }
    ClienteConPii { cliente: c, pii_oculto }
}

/// Cifra las columnas PII antes de persistir
fn cifrar(cipher: &PiiCipher, d: &mut ClienteDatos) -> Result<(), AppError> {
    for campo in [
        &mut d.celular,
        &mut d.celular2,
        &mut d.email,
        &mut d.dir_residencia,
        &mut d.dir_temporal,
        &mut d.no_licencia,
    ] {
        if let Some(v) = campo {
            *campo = Some(cipher.encrypt(v.trim())?);
        }
    }
    Ok(())
}

/// Normaliza campos (trim, nombre completo calculado, defaults)
fn normalizar(d: &mut ClienteDatos) {
    d.tipo_doc = d.tipo_doc.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.no_doc = d.no_doc.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.nombres = d.nombres.trim().to_string();
    d.apellidos = d.apellidos.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.nombre_completo = match &d.apellidos {
        Some(ap) if !ap.is_empty() => format!("{} {}", d.nombres, ap),
        _ => d.nombres.clone(),
    }
    .trim()
    .to_string();
    if d.estado.trim().is_empty() {
        d.estado = "Activo".into();
    }
}

/// Valida los datos del cliente (espejo de cliente_service.py).
/// La unicidad del documento la garantiza el índice único de la BD
/// (se mapea a AppError::Duplicate en el repositorio).
fn validar(d: &ClienteDatos, cfg: &Arc<AppConfig>) -> Result<(), AppError> {
    if d.nombres.is_empty() || d.nombres.len() > 100 {
        return Err(AppError::Validation(
            "El nombre del cliente es obligatorio (máx. 100 caracteres).".into(),
        ));
    }
    if let Some(doc) = &d.no_doc {
        if doc.len() > 30 {
            return Err(AppError::Validation(
                "El número de documento no puede superar 30 caracteres.".into(),
            ));
        }
    }
    if !cfg.estados_cliente.is_empty() && !cfg.estados_cliente.contains(&d.estado) {
        return Err(AppError::Validation(format!(
            "Estado inválido '{}'. Permitidos: {}",
            d.estado,
            cfg.estados_cliente.join(", ")
        )));
    }
    if let Some(email) = &d.email {
        if !email.is_empty()
            && !(email.contains('@') && email.contains('.') && email.len() <= 100)
        {
            return Err(AppError::Validation("El correo electrónico no es válido.".into()));
        }
    }
    if let Some(ven) = &d.vencimiento_licencia {
        if !ven.is_empty() && NaiveDate::parse_from_str(ven, "%Y-%m-%d").is_err() {
            return Err(AppError::Validation(
                "La fecha de vencimiento de la licencia no es válida.".into(),
            ));
        }
    }
    // Sanitización XSS en campos de texto libre
    for (campo, valor) in [
        ("nombres", Some(&d.nombres)),
        ("apellidos", d.apellidos.as_ref()),
        ("dirección residencia", d.dir_residencia.as_ref()),
        ("dirección temporal", d.dir_temporal.as_ref()),
        ("hotel", d.hotel.as_ref()),
        ("ciudad", d.ciudad.as_ref()),
    ] {
        if let Some(v) = valor {
            if !v.is_empty() {
                validate_no_xss(v, 255).map_err(|_| {
                    AppError::Validation(format!(
                        "El campo {campo} contiene caracteres no permitidos."
                    ))
                })?;
            }
        }
    }
    Ok(())
}
