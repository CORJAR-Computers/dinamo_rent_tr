//! services/gasto.rs — Lógica de negocio de gastos (caja menor)
//!
//! Valida fecha/categoría/descripción/monto, calcula totales por placa y
//! categoría (el backend es la fuente de verdad para los montos).

use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::prelude::FromStr as _;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::validators::{mayusculas, validate_no_xss};
use crate::core::PooledConnection;
use crate::repositories::gasto::{Gasto, GastoDatos, GastoRepository};

/// Categorías de gasto por defecto cuando config.ini no define `business.tipos_gasto`
const TIPOS_GASTO_FALLBACK: [&str; 10] = [
    "Combustible",
    "Peajes",
    "Lavado",
    "Mantenimiento",
    "Repuestos",
    "Parqueadero",
    "Seguros",
    "Multas",
    "Papelería",
    "Otros",
];

/// Total por placa o categoría (para el frontend)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalGasto {
    pub clave: String,
    pub total: String,
}

/// Resumen de totales para la página de gastos
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalesGastos {
    /// Suma de todos los gastos
    pub total_general: String,
    /// Suma de los gastos del mes calendario actual
    pub total_mes: String,
    pub por_placa: Vec<TotalGasto>,
    pub por_categoria: Vec<TotalGasto>,
}

pub struct GastoService;

impl GastoService {
    /// Lista gastos con filtros opcionales (búsqueda libre, placa y/o categoría)
    pub fn listar(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
        placa: Option<&str>,
        categoria: Option<&str>,
    ) -> Result<Vec<Gasto>, AppError> {
        let term = busqueda.unwrap_or("").trim();
        let placa = placa.unwrap_or("").trim();
        let categoria = categoria.unwrap_or("").trim();
        if !term.is_empty() {
            GastoRepository::buscar(conn, term)
        } else if !placa.is_empty() && !categoria.is_empty() && categoria != "Todos" {
            GastoRepository::obtener_por_placa_categoria(conn, placa, categoria)
        } else if !placa.is_empty() {
            GastoRepository::obtener_por_placa(conn, placa)
        } else if !categoria.is_empty() && categoria != "Todos" {
            GastoRepository::obtener_por_categoria(conn, categoria)
        } else {
            GastoRepository::obtener_todos(conn)
        }
    }

    /// Gastos recientes (para el inicio o un panel)
    pub fn recientes(conn: &mut PooledConnection, limit: i64) -> Result<Vec<Gasto>, AppError> {
        GastoRepository::obtener_recientes(conn, limit.max(1))
    }

    /// Obtiene un gasto por id
    pub fn obtener(conn: &mut PooledConnection, id: i64) -> Result<Gasto, AppError> {
        GastoRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe el gasto #{id}")))
    }

    /// Crea un gasto (el actor es el usuario de la sesión, para trazabilidad)
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        actor: &str,
        mut datos: GastoDatos,
    ) -> Result<Gasto, AppError> {
        normalizar(&mut datos);
        validar(&datos, cfg)?;
        let id = GastoRepository::insertar(conn, &datos, actor)?;
        Self::obtener(conn, id)
    }

    /// Actualiza un gasto por id
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        id: i64,
        mut datos: GastoDatos,
    ) -> Result<Gasto, AppError> {
        Self::obtener(conn, id)?;
        normalizar(&mut datos);
        validar(&datos, cfg)?;
        GastoRepository::actualizar(conn, id, &datos)?;
        Self::obtener(conn, id)
    }

    /// Elimina un gasto
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        Self::obtener(conn, id)?;
        GastoRepository::eliminar(conn, id)
    }

    /// Total de gastos (dashboard)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        GastoRepository::contar(conn)
    }

    /// Totales general, del mes, por placa y por categoría (página de gastos)
    pub fn totales(conn: &mut PooledConnection) -> Result<TotalesGastos, AppError> {
        Ok(TotalesGastos {
            total_general: GastoRepository::total_general(conn)?,
            total_mes: GastoRepository::total_mes(conn)?,
            por_placa: GastoRepository::total_por_placa(conn)?
                .into_iter()
                .map(|(clave, total)| TotalGasto { clave, total })
                .collect(),
            por_categoria: GastoRepository::total_por_categoria(conn)?
                .into_iter()
                .map(|(clave, total)| TotalGasto { clave, total })
                .collect(),
        })
    }
}

/// Normaliza campos (trim → mayúsculas, defaults)
fn normalizar(d: &mut GastoDatos) {
    d.placa = d
        .placa
        .as_ref()
        .map(|s| mayusculas(s))
        .filter(|s| !s.is_empty());
    d.categoria = mayusculas(&d.categoria);
    d.descripcion = mayusculas(&d.descripcion);
    d.monto = d.monto.trim().replace(',', ".");
    d.comprobante = d
        .comprobante
        .as_ref()
        .map(|s| mayusculas(s))
        .filter(|s| !s.is_empty());
}

/// Valida los datos del gasto
fn validar(d: &GastoDatos, cfg: &Arc<AppConfig>) -> Result<(), AppError> {
    // Fecha
    NaiveDate::parse_from_str(&d.fecha, "%Y-%m-%d").map_err(|_| {
        AppError::Validation("La fecha del gasto no es válida (formato AAAA-MM-DD).".into())
    })?;

    // Categoría
    let categorias: Vec<&str> = if cfg.tipos_gasto.is_empty() {
        TIPOS_GASTO_FALLBACK.to_vec()
    } else {
        cfg.tipos_gasto.iter().map(|s| s.as_str()).collect()
    };
    if d.categoria.is_empty() {
        return Err(AppError::Validation("La categoría es obligatoria.".into()));
    }
    let cat_upper = d.categoria.trim().to_uppercase();
    let cats_upper: Vec<String> = categorias.iter().map(|c| c.to_uppercase()).collect();
    if !cats_upper.contains(&cat_upper) {
        return Err(AppError::Validation(format!(
            "Categoría inválida '{}'. Permitidas: {}",
            d.categoria,
            categorias.join(", ")
        )));
    }

    // Descripción
    if d.descripcion.is_empty() {
        return Err(AppError::Validation(
            "La descripción es obligatoria.".into(),
        ));
    }
    if d.descripcion.len() > 200 {
        return Err(AppError::Validation(
            "La descripción no puede superar 200 caracteres.".into(),
        ));
    }
    validate_no_xss(&d.descripcion, 200).map_err(|_| {
        AppError::Validation("La descripción contiene caracteres no permitidos.".into())
    })?;

    // Monto
    let monto = Decimal::from_str(&d.monto).unwrap_or_else(|_| Decimal::from(-1));
    if monto < Decimal::ZERO {
        return Err(AppError::Validation(
            "El monto no es un número válido.".into(),
        ));
    }
    if monto == Decimal::ZERO {
        return Err(AppError::Validation(
            "El monto debe ser mayor que cero.".into(),
        ));
    }
    if monto > Decimal::from(9_999_999_999i64) / Decimal::from(100) {
        return Err(AppError::Validation(
            "El monto es demasiado grande (máx. 99.999.999,99).".into(),
        ));
    }

    // Campos de texto libre (consistente con autos/clientes/reservas)
    for (campo, val) in [
        ("la placa", d.placa.as_deref()),
        ("el comprobante", d.comprobante.as_deref()),
    ] {
        if let Some(v) = val {
            if !v.is_empty() && validate_no_xss(v, 2000).is_err() {
                return Err(AppError::Validation(format!(
                    "{campo} contiene caracteres no permitidos."
                )));
            }
        }
    }
    Ok(())
}
