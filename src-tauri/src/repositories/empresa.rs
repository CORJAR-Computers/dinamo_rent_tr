//! repositories/empresa.rs — Configuración de la empresa (setup inicial)
//!
//! Tabla `EMPRESA_CONFIG` de UNA fila (ID = 1). El logo se guarda como
//! archivo en `data_dir/logos/` (el binario no viaja por Firebird); aquí
//! solo se persiste el nombre del archivo (`LOGO`), null = sin logo.

use rsfbclient::{Execute, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

use serde::Serialize;

/// Configuración de la empresa (serializable al frontend, camelCase).
/// `logo` es una data URL (`data:image/png;base64,...`) o null.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmpresaConfig {
    pub nombre: Option<String>,
    pub nit: Option<String>,
    pub direccion: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub web: Option<String>,
    pub logo: Option<String>,
}

/// Datos recibidos al guardar (logo como data URL o null para quitar).
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EmpresaConfigDatos {
    pub nombre: Option<String>,
    pub nit: Option<String>,
    pub direccion: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub web: Option<String>,
    /// Data URL del logo (`data:<mime>;base64,<b64>`) o null = sin logo.
    pub logo: Option<String>,
}

/// Nombre de archivo del logo dentro de `data_dir/logos/`.
/// MIME permitido -> extensión (mismo mapa en el servicio).
pub fn logo_ext(mime: &str) -> Option<&'static str> {
    match mime.to_ascii_lowercase().as_str() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        _ => None,
    }
}

pub fn logo_mime(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

/// Orden de columnas del SELECT (alineado con `EmpresaRow`)
pub const SELECT_COLS: &str = "NOMBRE, NIT, DIRECCION, TELEFONO, EMAIL, WEB, LOGO";

#[allow(clippy::type_complexity)]
pub type EmpresaRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub struct EmpresaRepository;

impl EmpresaRepository {
    /// Lee la fila única (ID = 1); devuelve None si aún no existe.
    pub fn obtener(conn: &mut PooledConnection) -> Result<Option<EmpresaRow>, AppError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM EMPRESA_CONFIG WHERE ID = 1"
        );
        let row = conn.query_first(&sql, ())?;
        Ok(row)
    }

    /// Inserta o actualiza la fila única. `logo` = nombre de archivo o null.
    pub fn guardar(
        conn: &mut PooledConnection,
        cfg: &EmpresaConfigDatos,
        logo_archivo: Option<&str>,
    ) -> Result<(), AppError> {
        let sql = "UPDATE OR INSERT INTO EMPRESA_CONFIG \
                   (ID, NOMBRE, NIT, DIRECCION, TELEFONO, EMAIL, WEB, LOGO, UPDATED_AT) \
                   VALUES (1, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP) \
                   MATCHING (ID)";
        conn.execute(
            sql,
            (
                cfg.nombre.clone(),
                cfg.nit.clone(),
                cfg.direccion.clone(),
                cfg.telefono.clone(),
                cfg.email.clone(),
                cfg.web.clone(),
                logo_archivo.map(|s| s.to_string()),
            ),
        )?;
        Ok(())
    }
}
