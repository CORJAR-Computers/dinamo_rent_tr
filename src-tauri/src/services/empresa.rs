//! services/empresa.rs — Configuración de la empresa (setup inicial)
//!
//! Valida los datos, persiste el logo como archivo en `<data_dir>/logos/`
//! y la fila en `EMPRESA_CONFIG`. Expone la vista pública (nombre + logo)
//! para el login y el menú lateral, sin requerir sesión.

use std::path::PathBuf;

use base64::Engine;

use crate::core::audit::log_audit;
use crate::core::error::AppError;
use crate::core::PooledConnection;
use crate::repositories::empresa::{EmpresaConfig, EmpresaConfigDatos, EmpresaRepository};

/// Tamaño máximo del logo en bytes (2 MB) tras decodificar base64.
const LOGO_MAX_BYTES: usize = 2 * 1024 * 1024;

pub struct EmpresaService;

impl EmpresaService {
    /// Ruta absoluta del archivo de logo si existe (por el nombre guardado).
    fn logo_path(data_dir: &std::path::Path, archivo: Option<&str>) -> Option<PathBuf> {
        let nombre = archivo?;
        if nombre.is_empty() {
            return None;
        }
        let path = data_dir.join("logos").join(nombre);
        path.exists().then_some(path)
    }

    /// Convierte un archivo de logo a data URL (`data:<mime>;base64,...`).
    fn logo_a_data_url(data_dir: &std::path::Path, archivo: Option<&str>) -> Option<String> {
        let path = Self::logo_path(data_dir, archivo)?;
        let bytes = std::fs::read(&path).ok()?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let mime = crate::repositories::empresa::logo_mime(ext)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Some(format!("data:{mime};base64,{b64}"))
    }

    /// Lee la configuración completa (con logo como data URL).
    pub fn obtener(
        conn: &mut PooledConnection,
        data_dir: &std::path::Path,
    ) -> Result<EmpresaConfig, AppError> {
        let row = EmpresaRepository::obtener(conn)?;
        Ok(match row {
            Some((nombre, nit, dir, tel, email, web, logo)) => EmpresaConfig {
                nombre,
                nit,
                direccion: dir,
                telefono: tel,
                email,
                web,
                logo: Self::logo_a_data_url(data_dir, logo.as_deref()),
            },
            None => EmpresaConfig::default(),
        })
    }

    /// Vista pública (login / menú lateral): solo nombre + logo.
    pub fn publica(
        conn: &mut PooledConnection,
        data_dir: &std::path::Path,
    ) -> Result<EmpresaConfig, AppError> {
        let mut cfg = Self::obtener(conn, data_dir)?;
        cfg.nit = None;
        cfg.direccion = None;
        cfg.telefono = None;
        cfg.email = None;
        cfg.web = None;
        Ok(cfg)
    }

    /// Guarda los datos de la empresa. `logo = Some(data_url)` reemplaza el
    /// logo; `None` lo elimina. Registra auditoría.
    pub fn guardar(
        conn: &mut PooledConnection,
        data_dir: &std::path::Path,
        datos: EmpresaConfigDatos,
        usuario: &str,
    ) -> Result<EmpresaConfig, AppError> {
        // ── Validación básica (sin XSS: el frontend imprime como texto) ──
        let limpiar = |s: &Option<String>, max: usize| -> Option<String> {
            s.as_ref().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
                .map(|v| v.chars().take(max).collect())
        };
        let nombre = limpiar(&datos.nombre, 120);
        let nit = limpiar(&datos.nit, 40);
        let direccion = limpiar(&datos.direccion, 200);
        let telefono = limpiar(&datos.telefono, 40);
        let email = limpiar(&datos.email, 120);
        let web = limpiar(&datos.web, 120);

        // ── Logo: data URL -> archivo (o eliminar si viene null/vacío) ──
        // Borra siempre los logos previos `empresa.*` para no dejar huérfanos.
        let limpiar_logos_previos = || -> Result<(), AppError> {
            let dir = data_dir.join("logos");
            if !dir.exists() {
                return Ok(());
            }
            for ent in std::fs::read_dir(&dir).map_err(|e| {
                AppError::Generic(format!("No se pudo leer la carpeta de logos: {e}"))
            })? {
                if let Ok(ent) = ent {
                    let nombre_archivo = ent.file_name().to_string_lossy().to_string();
                    if nombre_archivo.starts_with("empresa.") {
                        let _ = std::fs::remove_file(ent.path());
                    }
                }
            }
            Ok(())
        };

        let logo_archivo: Option<String> = match datos.logo.as_deref().map(str::trim) {
            None | Some("") => {
                // Quitar logo.
                limpiar_logos_previos()?;
                None
            }
            Some(data_url) => {
                let (mime, b64) = parse_data_url(data_url)?;
                let ext = crate::repositories::empresa::logo_ext(&mime)
                    .ok_or_else(|| AppError::Generic(format!("Formato de logo no soportado: {mime}")))?;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|_| AppError::Generic("Logo inválido: base64 corrupto".into()))?;
                if bytes.is_empty() {
                    limpiar_logos_previos()?;
                    None
                } else if bytes.len() > LOGO_MAX_BYTES {
                    return Err(AppError::Generic(format!(
                        "Logo demasiado grande: {} KB (máx {} KB)",
                        bytes.len() / 1024,
                        LOGO_MAX_BYTES / 1024
                    )));
                } else {
                    limpiar_logos_previos()?;
                    let dir = data_dir.join("logos");
                    std::fs::create_dir_all(&dir).map_err(|e| {
                        AppError::Generic(format!("No se pudo crear la carpeta de logos: {e}"))
                    })?;
                    let nombre_archivo = format!("empresa.{ext}");
                    std::fs::write(dir.join(&nombre_archivo), &bytes).map_err(|e| {
                        AppError::Generic(format!("No se pudo escribir el logo: {e}"))
                    })?;
                    Some(nombre_archivo)
                }
            }
        };

        EmpresaRepository::guardar(
            conn,
            &EmpresaConfigDatos {
                nombre: nombre.clone(),
                nit: nit.clone(),
                direccion: direccion.clone(),
                telefono: telefono.clone(),
                email: email.clone(),
                web: web.clone(),
                logo: None,
            },
            logo_archivo.as_deref(),
        )?;

        log_audit(
            conn,
            usuario,
            "CONFIG_EMPRESA",
            "Configuración de la empresa actualizada (setup inicial)",
            "local",
        )?;

        Ok(EmpresaConfig {
            nombre,
            nit,
            direccion,
            telefono,
            email,
            web,
            logo: logo_archivo
                .as_deref()
                .and_then(|a| Self::logo_a_data_url(data_dir, Some(a))),
        })
    }
}

/// Separa `data:<mime>;base64,<b64>` en (mime, b64).
fn parse_data_url(data_url: &str) -> Result<(String, &str), AppError> {
    let resto = data_url
        .strip_prefix("data:")
        .ok_or_else(|| AppError::Generic("Logo inválido: falta prefijo data:".into()))?;
    let (meta, b64) = resto
        .split_once(',')
        .ok_or_else(|| AppError::Generic("Logo inválido: falta base64".into()))?;
    let mime = meta
        .split(';')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if mime.is_empty() {
        return Err(AppError::Generic("Logo inválido: falta MIME".into()));
    }
    Ok((mime, b64))
}
