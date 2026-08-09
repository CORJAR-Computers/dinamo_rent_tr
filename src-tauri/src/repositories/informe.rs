//! repositories/informe.rs — Consultas agregadas para el informe mensual.
//!
//! Sumas por rango de fechas (inicio - fin).
//! Los montos viajan como VARCHAR (CAST) para parseo exacto en la UI.

use rsfbclient::{Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

pub struct InformeRepository;

impl InformeRepository {
    /// Suma de pagos de rentas del rango (dinero realmente recibido)
    pub fn ingresos_pagos(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM pagos \
             WHERE fecha >= ? AND fecha <= ?",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de abonos de reservas con recogida en el rango (ingreso anticipado)
    pub fn ingresos_abonos_reservas(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(abono), 0) AS VARCHAR(12)) FROM reservas \
             WHERE estado <> 'Cancelada' \
               AND fecha_recogida >= ? AND fecha_recogida <= ?",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de gastos del rango
    pub fn egresos_gastos(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM gastos \
             WHERE fecha >= ? AND fecha <= ?",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de mantenimientos del rango
    pub fn egresos_mantenimiento(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(total_mantenimiento), 0) AS VARCHAR(12)) \
             FROM mantenimiento_vehiculos \
             WHERE pieza_varias_fecha >= ? AND pieza_varias_fecha <= ?",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de comparendos del rango (egreso potencial de la flota)
    pub fn egresos_comparendos(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<String, AppError> {
        let row: Option<(Option<String>,)> = conn.query_first(
            "SELECT CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) FROM comparendos \
             WHERE fecha_infraccion >= ? AND fecha_infraccion <= ?",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(row.and_then(|(s,)| s).unwrap_or_else(|| "0.00".into()))
    }

    /// Suma de gastos agrupada por categoría (para el desglose)
    pub fn gastos_por_categoria(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT categoria, CAST(SUM(monto) AS VARCHAR(12)) FROM gastos \
             WHERE fecha >= ? AND fecha <= ? \
             GROUP BY categoria ORDER BY SUM(monto) DESC",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    /// Rentas con recogida en el rango (detalle de ingresos del período)
    pub fn rentas_del_mes(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<(i64, String, String, String, String, String)>, AppError> {
        let rows: Vec<(i64, String, String, String, String, String)> = conn.query(
            "SELECT r.id, COALESCE(r.placa, ''), r.nombre_cliente, \
                    CAST(r.total AS VARCHAR(12)), r.estado, \
                    CAST(r.fecha_recogida AS VARCHAR(10)) \
             FROM rentas r \
             WHERE r.fecha_recogida >= ? AND r.fecha_recogida <= ? \
             ORDER BY r.fecha_recogida, r.id",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    // ── Utilidad por vehículo ────────────────────────────────────────────────

    /// Ingresos del rango por placa
    pub fn ingresos_por_placa(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT r.placa, CAST(SUM(p.monto) AS VARCHAR(12)) \
             FROM pagos p JOIN rentas r ON r.id = p.id_renta \
             WHERE r.placa IS NOT NULL \
               AND p.fecha >= ? AND p.fecha <= ? \
             GROUP BY r.placa",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    /// Abonos de reservas por placa
    pub fn abonos_reservas_por_placa(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa_asignada, CAST(SUM(abono) AS VARCHAR(12)) \
             FROM reservas \
             WHERE estado <> 'Cancelada' AND placa_asignada IS NOT NULL \
               AND fecha_recogida >= ? AND fecha_recogida <= ? \
             GROUP BY placa_asignada",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    /// Gastos por placa
    pub fn gastos_por_placa(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa, CAST(SUM(monto) AS VARCHAR(12)) FROM gastos \
             WHERE placa IS NOT NULL \
               AND fecha >= ? AND fecha <= ? \
             GROUP BY placa",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    /// Mantenimientos por placa
    pub fn mantenimiento_por_placa(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa, CAST(SUM(total_mantenimiento) AS VARCHAR(12)) \
             FROM mantenimiento_vehiculos \
             WHERE pieza_varias_fecha >= ? AND pieza_varias_fecha <= ? \
             GROUP BY placa",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    /// Comparendos por placa
    pub fn comparendos_por_placa(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa, CAST(SUM(monto) AS VARCHAR(12)) FROM comparendos \
             WHERE fecha_infraccion >= ? AND fecha_infraccion <= ? \
             GROUP BY placa",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    /// Marca + modelo por placa (para mostrar el vehículo en la tabla de utilidad)
    pub fn vehiculos(conn: &mut PooledConnection) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT placa, COALESCE(marca || ' ' || modelo, '') FROM autos",
            (),
        )?;
        Ok(rows)
    }
}
