//! repositories/informe.rs — Consultas agregadas para el informe mensual.
//!
//! Sumas por rango de fechas (inicio - fin).
//! Los montos viajan como VARCHAR (CAST) para parseo exacto en la UI.
//!
//! TAREA 3.1 (Bloque 3 — Performance): se consolidaron las 6 agregaciones de
//! totales del rango en una sola query con UNION ALL (`totales_rango`) y las 6
//! agregaciones por placa en una sola query con UNION ALL
//! (`movimientos_por_placa`). Esto reduce los round-trips del informe mensual
//! de 13 (7 + 6) a 5 (4 en `mensual` + 2 en `utilidad_por_vehiculo`).

use rsfbclient::{IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;

pub struct InformeRepository;

/// 6 totales del rango en una sola query (UNION ALL).
///
/// Resultado de `totales_rango`: ingresos por pagos, ingresos por abonos de
/// reservas, egresos por gastos, egresos por mantenimiento, egresos por
/// comparendos y total de comisiones. Sustituye 6 round-trips por 1.
#[derive(Debug, Clone, Default)]
pub struct TotalesRango {
    pub ingresos_pagos: String,
    pub ingresos_reservas: String,
    pub egresos_gastos: String,
    pub egresos_mantenimiento: String,
    pub egresos_comparendos: String,
    pub total_comisiones: String,
}

impl TotalesRango {
    /// Garantiza que ningún campo quede vacío (defense-in-depth: SUM siempre
    /// devuelve 1 fila, así que el UNION ALL siempre devuelve 6 filas, pero si
    /// por algún motivo una subquery no devolviera fila el campo queda en
    /// "0.00" como en la implementación original).
    fn with_defaults(mut self) -> Self {
        let defaults = [
            (&mut self.ingresos_pagos, "0.00"),
            (&mut self.ingresos_reservas, "0.00"),
            (&mut self.egresos_gastos, "0.00"),
            (&mut self.egresos_mantenimiento, "0.00"),
            (&mut self.egresos_comparendos, "0.00"),
            (&mut self.total_comisiones, "0.00"),
        ];
        for (field, def) in defaults {
            if field.is_empty() {
                *field = def.into();
            }
        }
        self
    }
}

impl InformeRepository {
    /// 6 totales del rango en una sola query (UNION ALL).
    ///
    /// Reemplaza las 6 llamadas separadas (`ingresos_pagos`,
    /// `ingresos_abonos_reservas`, `egresos_gastos`, `egresos_mantenimiento`,
    /// `egresos_comparendos`, `comisiones`) por una única consulta que hace 6
    /// agregaciones con `UNION ALL`. El primer parámetro posicional de cada
    /// subquery es `inicio` y el segundo es `fin` (12 parámetros en total).
    ///
    /// Cada subquery está etiquetada con un literal `tipo` (PAGOS, RESERVAS,
    /// GASTOS, MANT, COMP, COMIS) para que el Rust pueda repartir los
    /// resultados al struct `TotalesRango`.
    pub fn totales_rango(
        conn: &mut PooledConnection,
        inicio: &str,
        fin: &str,
    ) -> Result<TotalesRango, AppError> {
        // 12 params posicionales: inicio/fin repetidos 6 veces (uno por subquery).
        let params: Vec<String> = [inicio.to_string(), fin.to_string()]
            .iter()
            .cycle()
            .take(12)
            .cloned()
            .collect();
        let ptype = ParamsType::Positional(params.iter().map(|p| p.into_param()).collect());

        let sql = "\
            SELECT CAST('PAGOS' AS VARCHAR(8)) AS tipo, \
                   CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) AS monto \
            FROM pagos WHERE fecha >= ? AND fecha <= ? AND deleted_at IS NULL \
            UNION ALL \
            SELECT CAST('RESERVAS' AS VARCHAR(8)), \
                   CAST(COALESCE(SUM(abono), 0) AS VARCHAR(12)) \
            FROM reservas WHERE estado <> 'Cancelada' \
              AND fecha_recogida >= ? AND fecha_recogida <= ? \
            UNION ALL \
            SELECT CAST('GASTOS' AS VARCHAR(8)), \
                   CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) \
            FROM gastos WHERE fecha >= ? AND fecha <= ? AND deleted_at IS NULL \
            UNION ALL \
            SELECT CAST('MANT' AS VARCHAR(8)), \
                   CAST(COALESCE(SUM(total_mantenimiento), 0) AS VARCHAR(12)) \
            FROM mantenimiento_vehiculos \
            WHERE pieza_varias_fecha >= ? AND pieza_varias_fecha <= ? \
              AND deleted_at IS NULL \
            UNION ALL \
            SELECT CAST('COMP' AS VARCHAR(8)), \
                   CAST(COALESCE(SUM(monto), 0) AS VARCHAR(12)) \
            FROM comparendos WHERE fecha_infraccion >= ? AND fecha_infraccion <= ? \
              AND deleted_at IS NULL \
            UNION ALL \
            SELECT CAST('COMIS' AS VARCHAR(8)), \
                   CAST(COALESCE(SUM(comision), 0) AS VARCHAR(12)) \
            FROM rentas WHERE fecha_recogida >= ? AND fecha_recogida <= ? \
              AND deleted_at IS NULL";

        let rows: Vec<(String, String)> = conn.query(sql, ptype)?;
        let mut t = TotalesRango::default();
        for (tipo, monto) in rows {
            match tipo.as_str() {
                "PAGOS" => t.ingresos_pagos = monto,
                "RESERVAS" => t.ingresos_reservas = monto,
                "GASTOS" => t.egresos_gastos = monto,
                "MANT" => t.egresos_mantenimiento = monto,
                "COMP" => t.egresos_comparendos = monto,
                "COMIS" => t.total_comisiones = monto,
                _ => {}
            }
        }
        Ok(t.with_defaults())
    }

    /// Movimientos por placa del rango en una sola query (UNION ALL).
    ///
    /// Reemplaza las 6 llamadas separadas (`ingresos_por_placa`,
    /// `abonos_reservas_por_placa`, `gastos_por_placa`, `comisiones_por_placa`,
    /// `mantenimiento_por_placa`, `comparendos_por_placa`) por una única
    /// consulta que hace 6 agregaciones por placa con `UNION ALL`. Devuelve
    /// tuplas `(placa, tipo, monto)` donde `tipo` ∈
    /// `{INGRESO, ABONO, GASTO, COMISION, MANT, COMP}`.
    ///
    /// El service acumula estos movimientos en un `HashMap<placa, (ingresos,
    /// costos)>`: INGRESO y ABONO suman a ingresos; GASTO, COMISION, MANT y
    /// COMP suman a costos.
    pub fn movimientos_por_placa(
        conn: &mut PooledConnection,
        inicio: &str,
        fin: &str,
    ) -> Result<Vec<(String, String, String)>, AppError> {
        // 12 params posicionales: inicio/fin repetidos 6 veces (uno por subquery).
        let params: Vec<String> = [inicio.to_string(), fin.to_string()]
            .iter()
            .cycle()
            .take(12)
            .cloned()
            .collect();
        let ptype = ParamsType::Positional(params.iter().map(|p| p.into_param()).collect());

        let sql = "\
            SELECT r.placa, CAST('INGRESO' AS VARCHAR(8)) AS tipo, \
                   CAST(SUM(p.monto) AS VARCHAR(12)) AS monto \
            FROM pagos p JOIN rentas r ON r.id = p.id_renta \
            WHERE r.placa IS NOT NULL AND p.fecha >= ? AND p.fecha <= ? \
              AND p.deleted_at IS NULL AND r.deleted_at IS NULL \
            GROUP BY r.placa \
            UNION ALL \
            SELECT placa_asignada, CAST('ABONO' AS VARCHAR(8)), \
                   CAST(SUM(abono) AS VARCHAR(12)) \
            FROM reservas \
            WHERE estado <> 'Cancelada' AND placa_asignada IS NOT NULL \
              AND fecha_recogida >= ? AND fecha_recogida <= ? \
            GROUP BY placa_asignada \
            UNION ALL \
            SELECT placa, CAST('GASTO' AS VARCHAR(8)), \
                   CAST(SUM(monto) AS VARCHAR(12)) \
            FROM gastos WHERE placa IS NOT NULL \
              AND fecha >= ? AND fecha <= ? AND deleted_at IS NULL \
            GROUP BY placa \
            UNION ALL \
            SELECT placa, CAST('COMISION' AS VARCHAR(8)), \
                   CAST(SUM(comision) AS VARCHAR(12)) \
            FROM rentas WHERE placa IS NOT NULL AND comision > 0 \
              AND fecha_recogida >= ? AND fecha_recogida <= ? \
              AND deleted_at IS NULL \
            GROUP BY placa \
            UNION ALL \
            SELECT placa, CAST('MANT' AS VARCHAR(8)), \
                   CAST(SUM(total_mantenimiento) AS VARCHAR(12)) \
            FROM mantenimiento_vehiculos \
            WHERE pieza_varias_fecha >= ? AND pieza_varias_fecha <= ? \
              AND deleted_at IS NULL \
            GROUP BY placa \
            UNION ALL \
            SELECT placa, CAST('COMP' AS VARCHAR(8)), \
                   CAST(SUM(monto) AS VARCHAR(12)) \
            FROM comparendos WHERE fecha_infraccion >= ? AND fecha_infraccion <= ? \
              AND deleted_at IS NULL \
            GROUP BY placa";

        let rows: Vec<(String, String, String)> = conn.query(sql, ptype)?;
        Ok(rows)
    }

    /// Suma de gastos agrupada por categoría (para el desglose)
    pub fn gastos_por_categoria(
        conn: &mut PooledConnection,
        inicio: &str,
        fin: &str,
    ) -> Result<Vec<(String, String)>, AppError> {
        let rows: Vec<(String, String)> = conn.query(
            "SELECT categoria, CAST(SUM(monto) AS VARCHAR(12)) FROM gastos \
             WHERE fecha >= ? AND fecha <= ? AND deleted_at IS NULL \
             GROUP BY categoria ORDER BY SUM(monto) DESC",
            (inicio.to_string(), fin.to_string()),
        )?;
        Ok(rows)
    }

    /// Rentas con recogida en el rango (detalle de ingresos del período)
    pub fn rentas_del_mes(
        conn: &mut PooledConnection,
        inicio: &str,
        fin: &str,
    ) -> Result<Vec<(i64, String, String, String, String, String, String, String)>, AppError> {
        let rows: Vec<(i64, String, String, String, String, String, String, String)> = conn.query(
            "SELECT r.id, COALESCE(r.placa, ''), r.nombre_cliente, \
                    CAST(r.total AS VARCHAR(12)), r.estado, \
                    CAST(r.comision AS VARCHAR(12)), CAST(r.valor_neto AS VARCHAR(12)), \
                    CAST(r.fecha_recogida AS VARCHAR(10)) \
             FROM rentas r \
             WHERE r.fecha_recogida >= ? AND r.fecha_recogida <= ? \
               AND r.deleted_at IS NULL \
             ORDER BY r.fecha_recogida, r.id",
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
