//! repositories/auto.rs — Repositorio de vehículos (puerto de auto_repository_sa.py)
//!
//! Queries explícitas en dialecto Firebird con rsfbclient.
//! - DECIMAL → CAST a VARCHAR (parseo exacto en el servicio)
//! - DATE/TIMESTAMP → CAST a VARCHAR (formato 'YYYY-MM-DD')
//! - DOUBLE PRECISION → f64

use rsfbclient::{Execute, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

use serde::Serialize;

/// Vehículo completo (serializable al frontend, camelCase)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Auto {
    pub placa: String,
    pub marca: String,
    pub modelo: String,
    pub version: Option<String>,
    pub color: Option<String>,
    pub tipo: String,
    pub cilindraje: Option<String>,
    pub transmision: Option<String>,
    pub combustible: Option<String>,
    pub no_motor: Option<String>,
    pub no_chasis: Option<String>,
    pub propietario: Option<String>,
    pub estado: String,
    /// Monto como string (decimal exacto)
    pub costo_fijo_mensual: String,
    pub kilometraje: f64,
    pub ubicacion: Option<String>,
    pub tipo_adquisicion: Option<String>,
    pub proximo_aceite: Option<i64>,
    pub proximo_frenos: Option<i64>,
    pub vencimiento_soat: Option<String>,
    pub vencimiento_tecnico: Option<String>,
    pub vencimiento_extintor: Option<String>,
    pub vencimiento_bateria: Option<String>,
    pub observaciones: Option<String>,
    pub fecha_ingreso: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Construye parámetros posicionales de cualquier longitud (las tuplas de
/// `IntoParams` están limitadas a 15 elementos en rsfbclient).
macro_rules! params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into()),+])
    };
}

/// Datos de entrada para crear/actualizar (validados por el servicio)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoDatos {
    pub placa: String,
    pub marca: String,
    pub modelo: String,
    pub version: Option<String>,
    pub color: Option<String>,
    pub tipo: String,
    pub cilindraje: Option<String>,
    pub transmision: Option<String>,
    pub combustible: Option<String>,
    pub no_motor: Option<String>,
    pub no_chasis: Option<String>,
    pub propietario: Option<String>,
    pub estado: String,
    pub costo_fijo_mensual: String,
    pub kilometraje: f64,
    pub ubicacion: Option<String>,
    pub tipo_adquisicion: Option<String>,
    pub proximo_aceite: Option<i64>,
    pub proximo_frenos: Option<i64>,
    pub vencimiento_soat: Option<String>,
    pub vencimiento_tecnico: Option<String>,
    pub vencimiento_extintor: Option<String>,
    pub vencimiento_bateria: Option<String>,
    pub observaciones: Option<String>,
    pub fecha_ingreso: String,
}

/// Orden de columnas del SELECT de autos (debe coincidir con `AutoRow`)
pub const SELECT_COLS: &str = "\
    placa, marca, modelo, version, color, tipo, cilindraje, transmision, combustible, \
    no_motor, no_chasis, propietario, estado, \
    CAST(costo_fijo_mensual AS VARCHAR(20)), kilometraje, ubicacion, tipo_adquisicion, \
    proximo_aceite, proximo_frenos, \
    CAST(vencimiento_soat AS VARCHAR(10)), CAST(vencimiento_tecnico AS VARCHAR(10)), \
    CAST(vencimiento_extintor AS VARCHAR(10)), CAST(vencimiento_bateria AS VARCHAR(10)), \
    CAST(observaciones AS VARCHAR(2000)), \
    CAST(fecha_ingreso AS VARCHAR(10)), \
    CAST(created_at AS VARCHAR(30))";

/// Fila de SELECT de autos (tupla larga — mantener alineada con `SELECT_COLS`)
#[allow(clippy::type_complexity)]
pub type AutoRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    f64,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

fn from_row(r: AutoRow) -> Auto {
    Auto {
        placa: r.0,
        marca: r.1,
        modelo: r.2,
        version: r.3,
        color: r.4,
        tipo: r.5,
        cilindraje: r.6,
        transmision: r.7,
        combustible: r.8,
        no_motor: r.9,
        no_chasis: r.10,
        propietario: r.11,
        estado: r.12,
        costo_fijo_mensual: r.13,
        kilometraje: r.14,
        ubicacion: r.15,
        tipo_adquisicion: r.16,
        proximo_aceite: r.17,
        proximo_frenos: r.18,
        vencimiento_soat: r.19,
        vencimiento_tecnico: r.20,
        vencimiento_extintor: r.21,
        vencimiento_bateria: r.22,
        observaciones: r.23,
        fecha_ingreso: r.24,
        created_at: r.25,
        updated_at: None,
    }
}

/// Mapea errores de Firebird a AppError (duplicados, FKs)
fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("duplicate") || lower.contains("unique") {
        AppError::Duplicate("Ya existe un vehículo con esa placa.".into())
    } else {
        AppError::Database(msg)
    }
}

pub struct AutoRepository;

impl AutoRepository {
    /// Lista todos los vehículos (por marca, placa)
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Auto>, AppError> {
        let rows: Vec<AutoRow> =
            conn.query(&format!("SELECT {SELECT_COLS} FROM autos ORDER BY marca, placa"), ())?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Busca vehículos por placa, marca o modelo (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Auto>, AppError> {
        let like = format!("%{}%", term.trim());
        let rows: Vec<AutoRow> = conn.query(
            &format!(
                "SELECT {SELECT_COLS} FROM autos \
                 WHERE UPPER(placa) LIKE UPPER(?) OR UPPER(marca) LIKE UPPER(?) OR UPPER(modelo) LIKE UPPER(?) \
                 ORDER BY marca, placa"
            ),
            (like.clone(), like.clone(), like),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Filtra por estado (Todos / Disponible / Rentado / ...)
    pub fn obtener_por_estado(conn: &mut PooledConnection, estado: &str) -> Result<Vec<Auto>, AppError> {
        let rows: Vec<AutoRow> = conn.query(
            &format!("SELECT {SELECT_COLS} FROM autos WHERE estado = ? ORDER BY marca, placa"),
            (estado.to_string(),),
        )?;
        Ok(rows.into_iter().map(from_row).collect())
    }

    /// Obtiene un vehículo por placa
    pub fn obtener_por_placa(
        conn: &mut PooledConnection,
        placa: &str,
    ) -> Result<Option<Auto>, AppError> {
        let row: Option<AutoRow> = conn.query_first(
            &format!("SELECT {SELECT_COLS} FROM autos WHERE placa = ?"),
            (placa.trim().to_string(),),
        )?;
        Ok(row.map(from_row))
    }

    /// ¿Existe un vehículo con esa placa?
    pub fn existe(conn: &mut PooledConnection, placa: &str) -> Result<bool, AppError> {
        let count: Option<(i64,)> = conn.query_first(
            "SELECT COUNT(*) FROM autos WHERE placa = ?",
            (placa.trim().to_string(),),
        )?;
        Ok(count.map(|(c,)| c).unwrap_or(0) > 0)
    }

    /// Crea un vehículo. La placa es la PK.
    pub fn insertar(conn: &mut PooledConnection, d: &AutoDatos) -> Result<(), AppError> {
        conn.execute(
            "INSERT INTO autos (\
                placa, marca, modelo, version, color, tipo, cilindraje, transmision, combustible, \
                no_motor, no_chasis, propietario, estado, costo_fijo_mensual, kilometraje, ubicacion, \
                tipo_adquisicion, proximo_aceite, proximo_frenos, vencimiento_soat, \
                vencimiento_tecnico, vencimiento_extintor, vencimiento_bateria, observaciones, fecha_ingreso \
             ) VALUES (\
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CAST(? AS DECIMAL(12,2)), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? \
             )",
            params![
                d.placa.to_string(),
                d.marca.to_string(),
                d.modelo.to_string(),
                opt_str(&d.version),
                opt_str(&d.color),
                d.tipo.to_string(),
                opt_str(&d.cilindraje),
                opt_str(&d.transmision),
                opt_str(&d.combustible),
                opt_str(&d.no_motor),
                opt_str(&d.no_chasis),
                opt_str(&d.propietario),
                d.estado.to_string(),
                d.costo_fijo_mensual.to_string(),
                d.kilometraje,
                opt_str(&d.ubicacion),
                opt_str(&d.tipo_adquisicion),
                d.proximo_aceite,
                d.proximo_frenos,
                opt_str(&d.vencimiento_soat),
                opt_str(&d.vencimiento_tecnico),
                opt_str(&d.vencimiento_extintor),
                opt_str(&d.vencimiento_bateria),
                opt_str(&d.observaciones),
                d.fecha_ingreso.to_string(),
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Actualiza un vehículo por placa
    pub fn actualizar(conn: &mut PooledConnection, placa: &str, d: &AutoDatos) -> Result<(), AppError> {
        conn.execute(
            "UPDATE autos SET \
                marca = ?, modelo = ?, version = ?, color = ?, tipo = ?, cilindraje = ?, \
                transmision = ?, combustible = ?, no_motor = ?, no_chasis = ?, propietario = ?, \
                estado = ?, costo_fijo_mensual = CAST(? AS DECIMAL(12,2)), kilometraje = ?, ubicacion = ?, \
                tipo_adquisicion = ?, proximo_aceite = ?, proximo_frenos = ?, vencimiento_soat = ?, \
                vencimiento_tecnico = ?, vencimiento_extintor = ?, vencimiento_bateria = ?, \
                observaciones = ?, fecha_ingreso = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE placa = ?",
            params![
                d.marca.to_string(),
                d.modelo.to_string(),
                opt_str(&d.version),
                opt_str(&d.color),
                d.tipo.to_string(),
                opt_str(&d.cilindraje),
                opt_str(&d.transmision),
                opt_str(&d.combustible),
                opt_str(&d.no_motor),
                opt_str(&d.no_chasis),
                opt_str(&d.propietario),
                d.estado.to_string(),
                d.costo_fijo_mensual.to_string(),
                d.kilometraje,
                opt_str(&d.ubicacion),
                opt_str(&d.tipo_adquisicion),
                d.proximo_aceite,
                d.proximo_frenos,
                opt_str(&d.vencimiento_soat),
                opt_str(&d.vencimiento_tecnico),
                opt_str(&d.vencimiento_extintor),
                opt_str(&d.vencimiento_bateria),
                opt_str(&d.observaciones),
                d.fecha_ingreso.to_string(),
                placa.to_string(),
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Elimina un vehículo (FKs: mantenimiento/comparendos CASCADE, gastos/reservas SET NULL,
    /// rentas SIN cascada → bloquea si hay rentas asociadas)
    pub fn eliminar(conn: &mut PooledConnection, placa: &str) -> Result<(), AppError> {
        match conn.execute("DELETE FROM autos WHERE placa = ?", (placa.trim().to_string(),)) {
            Ok(_) => Ok(()),
            Err(e) => {
                let lower = e.to_string().to_lowercase();
                if lower.contains("foreign key")
                    || lower.contains("still referenced")
                    || lower.contains("not a valid reference")
                {
                    Err(AppError::Business(
                        "El vehículo tiene registros asociados (rentas, mantenimiento, comparendos) y no puede eliminarse."
                            .into(),
                    ))
                } else {
                    Err(AppError::Database(e.to_string()))
                }
            }
        }
    }

    /// Actualiza el km del próximo cambio de aceite (lo dispara el servicio de
    /// mantenimiento al registrar un cambio de aceite). `None` limpia el campo.
    pub fn actualizar_proximo_aceite(
        conn: &mut PooledConnection,
        placa: &str,
        km: Option<i64>,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE autos SET proximo_aceite = ?, updated_at = CURRENT_TIMESTAMP WHERE placa = ?",
            (km, placa.trim().to_string()),
        )?;
        Ok(())
    }

    /// Lista las placas de la flota que aún se operan (para el Agente SIMIT).
    /// Excluye Vendido y Baja: no tiene sentido consultar comparendos de
    /// vehículos que ya salieron de la flota.
    pub fn placas_activas(conn: &mut PooledConnection) -> Result<Vec<String>, AppError> {
        let rows: Vec<(String,)> = conn.query(
            "SELECT placa FROM autos \
             WHERE estado NOT IN ('Vendido', 'Baja') ORDER BY placa",
            (),
        )?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    /// Total de vehículos
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> = conn.query_first("SELECT COUNT(*) FROM autos", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Conteo por estado (para dashboard: Disponible, Rentado, Mantenimiento, ...)
    pub fn contar_por_estado(conn: &mut PooledConnection) -> Result<Vec<(String, i64)>, AppError> {
        let rows: Vec<(String, i64)> = conn.query(
            "SELECT estado, COUNT(*) FROM autos GROUP BY estado ORDER BY estado",
            (),
        )?;
        Ok(rows)
    }
}

fn opt_str(v: &Option<String>) -> Option<String> {
    v.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}


