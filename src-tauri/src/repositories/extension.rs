//! repositories/extension.rs — Repositorio de extensiones de rentas

use rsfbclient::{Execute, IntoParam, Queryable};
use serde::Serialize;

/// Extensión de una renta (historial)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRenta {
    pub id: i64,
    pub id_renta: i64,
    /// Tipo: "horas" o "dias"
    pub tipo: String,
    /// Cantidad de horas o días
    pub cantidad: i64,
    /// Valor unitario (hora o día)
    pub valor_unitario: String,
    /// Valor total (cantidad × valor_unitario)
    pub valor_total: String,
    /// Observaciones
    pub observaciones: Option<String>,
    /// Usuario que registró
    pub usuario: Option<String>,
    /// Fecha de creación
    pub created_at: Option<String>,
}

pub struct ExtensionRentaRepository;

impl ExtensionRentaRepository {
    /// Inserta una extensión en el historial
    pub fn insertar(
        conn: &mut crate::core::PooledConnection,
        id_renta: i64,
        tipo: &str,
        cantidad: i64,
        valor_unitario: &str,
        valor_total: &str,
        observaciones: Option<&str>,
        usuario: &str,
    ) -> Result<i64, crate::core::error::AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO extensiones_renta (id_renta, tipo, cantidad, valor_unitario, valor_total, observaciones, usuario) \
                 VALUES (?, ?, ?, CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), ?, ?) RETURNING id",
                rsfbclient::ParamsType::Positional(vec![
                    id_renta.into_param(),
                    tipo.to_string().into_param(),
                    cantidad.into_param(),
                    valor_unitario.to_string().into_param(),
                    valor_total.to_string().into_param(),
                    observaciones.map(String::from).into_param(),
                    usuario.to_string().into_param(),
                ]),
            )
            .map_err(|e| crate::core::error::AppError::Database(e.to_string()))?;
        Ok(id)
    }

    /// Lista extensiones de una renta (ordenadas por fecha ascendente)
    pub fn listar_por_renta(
        conn: &mut crate::core::PooledConnection,
        id_renta: i64,
    ) -> Result<Vec<ExtensionRenta>, crate::core::error::AppError> {
        let rows: Vec<(i64, i64, String, i64, String, String, Option<String>, Option<String>, Option<String>)> = conn
            .query(
                "SELECT id, id_renta, tipo, cantidad, \
                 CAST(valor_unitario AS VARCHAR(12)), CAST(valor_total AS VARCHAR(12)), \
                 observaciones, usuario, CAST(created_at AS VARCHAR(30)) \
                 FROM extensiones_renta WHERE id_renta = ? ORDER BY created_at ASC",
                (id_renta,),
            )
            .map_err(|e| crate::core::error::AppError::Database(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| ExtensionRenta {
                id: r.0,
                id_renta: r.1,
                tipo: r.2,
                cantidad: r.3,
                valor_unitario: r.4,
                valor_total: r.5,
                observaciones: r.6,
                usuario: r.7,
                created_at: r.8,
            })
            .collect())
    }

    /// Suma el valor total de todas las extensiones de una renta
    pub fn sumar_valor_total(
        conn: &mut crate::core::PooledConnection,
        id_renta: i64,
    ) -> Result<String, crate::core::error::AppError> {
        let result: Option<(String,)> = conn
            .query_first(
                "SELECT CAST(COALESCE(SUM(valor_total), 0) AS VARCHAR(12)) FROM extensiones_renta WHERE id_renta = ?",
                (id_renta,),
            )
            .map_err(|e| crate::core::error::AppError::Database(e.to_string()))?;
        Ok(result.map(|r| r.0).unwrap_or_else(|| "0.00".to_string()))
    }
}
