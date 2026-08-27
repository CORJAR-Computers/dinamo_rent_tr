//! services/informe.rs — Balance por rango de fechas.
//!
//! Ingresos: pagos de rentas + abonos de reservas del rango.
//! Egresos: gastos + mantenimientos + comparendos del rango.
//! El balance = ingresos − egresos.
//!
//! TAREA 3.1 (Bloque 3 — Performance): se consolidaron los 6 totales del
//! rango en una sola query (UNION ALL → `InformeRepository::totales_rango`) y
//! los 6 movimientos por placa en una sola query (UNION ALL →
//! `InformeRepository::movimientos_por_placa`). El informe mensual pasa de 13
//! round-trips (7 en `mensual` + 6 en `utilidad_por_vehiculo`) a 5 (4 + 2).

use std::collections::HashMap;

use rust_decimal::prelude::FromStr as _;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::core::error::AppError;
use crate::core::PooledConnection;
use crate::repositories::informe::InformeRepository;

/// Detalle de una renta del rango (para el informe)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RentaInforme {
    pub id: i64,
    pub placa: String,
    pub nombre_cliente: String,
    pub total: String,
    /// Comisión de la renta (neto = total − comisión)
    pub comision: String,
    /// Valor neto = total − comisión (información financiera)
    pub valor_neto: String,
    pub estado: String,
    pub fecha_recogida: String,
}

/// Balance del rango (serializable al frontend)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InformeMensual {
    pub fecha_inicio: String,
    pub fecha_fin: String,
    /// Pagos de rentas recibidos en el rango
    pub ingresos_pagos: String,
    /// Abonos de reservas con recogida en el rango
    pub ingresos_reservas: String,
    pub total_ingresos: String,
    /// Gastos de caja menor del rango
    pub egresos_gastos: String,
    /// Mantenimientos del rango
    pub egresos_mantenimiento: String,
    /// Comparendos del rango
    pub egresos_comparendos: String,
    pub total_egresos: String,
    pub balance: String,
    /// Comisiones de las rentas del rango (costo de intermediarios)
    pub total_comisiones: String,
    /// Ingresos netos = total_ingresos − total_comisiones
    pub ingresos_netos: String,
    /// Balance neto = balance − total_comisiones
    pub balance_neto: String,
    /// Desglose de gastos por categoría
    pub gastos_por_categoria: Vec<(String, String)>,
    /// Rentas con recogida en el rango
    pub rentas: Vec<RentaInforme>,
    /// Utilidad por vehículo
    pub utilidad_por_vehiculo: Vec<UtilidadVehiculo>,
}

/// Utilidad de un vehículo en el rango
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UtilidadVehiculo {
    pub placa: String,
    pub vehiculo: String,
    pub ingresos: String,
    pub costos: String,
    pub utilidad: String,
}

pub struct InformeService;

impl InformeService {
    /// Calcula el balance del rango de fechas indicado.
    ///
    /// Round-trips después de la consolidación (TAREA 3.1):
    ///   1. `totales_rango` (UNION ALL de 6 agregaciones)
    ///   2. `rentas_del_mes`
    ///   3. `gastos_por_categoria`
    ///   4. `utilidad_por_vehiculo` (que internamente hace 2: `vehiculos` +
    ///      `movimientos_por_placa`)
    ///
    /// Total: 5 round-trips (antes: 13).
    pub fn mensual(
        conn: &mut PooledConnection,
        fecha_inicio: &str,
        fecha_fin: &str,
    ) -> Result<InformeMensual, AppError> {
        // 1 query (UNION ALL) en vez de 6 round-trips por separado.
        let totales = InformeRepository::totales_rango(conn, fecha_inicio, fecha_fin)?;

        let total_ingresos = sum(&[&totales.ingresos_pagos, &totales.ingresos_reservas]);
        let total_egresos = sum(&[
            &totales.egresos_gastos,
            &totales.egresos_mantenimiento,
            &totales.egresos_comparendos,
        ]);
        let balance = (dec(&total_ingresos) - dec(&total_egresos)).round_dp(2);
        // Comisiones de las rentas del rango (costo de intermediarios): los
        // netos reflejan lo que la empresa realmente se queda.
        let ingresos_netos = (dec(&total_ingresos) - dec(&totales.total_comisiones)).round_dp(2);
        let balance_neto = (balance - dec(&totales.total_comisiones)).round_dp(2);

        let rentas = InformeRepository::rentas_del_mes(conn, fecha_inicio, fecha_fin)?
            .into_iter()
            .map(
                |(
                    id,
                    placa,
                    nombre_cliente,
                    total,
                    estado,
                    comision,
                    valor_neto,
                    fecha_recogida,
                )| {
                    RentaInforme {
                        id,
                        placa,
                        nombre_cliente,
                        total,
                        comision,
                        valor_neto,
                        estado,
                        fecha_recogida,
                    }
                },
            )
            .collect();

        Ok(InformeMensual {
            fecha_inicio: fecha_inicio.to_string(),
            fecha_fin: fecha_fin.to_string(),
            ingresos_pagos: totales.ingresos_pagos,
            ingresos_reservas: totales.ingresos_reservas,
            total_ingresos: total_ingresos.clone(),
            egresos_gastos: totales.egresos_gastos,
            egresos_mantenimiento: totales.egresos_mantenimiento,
            egresos_comparendos: totales.egresos_comparendos,
            total_egresos: total_egresos.clone(),
            balance: balance.to_string(),
            total_comisiones: totales.total_comisiones,
            ingresos_netos: ingresos_netos.to_string(),
            balance_neto: balance_neto.to_string(),
            gastos_por_categoria: InformeRepository::gastos_por_categoria(
                conn,
                fecha_inicio,
                fecha_fin,
            )?,
            rentas,
            utilidad_por_vehiculo: utilidad_por_vehiculo(conn, fecha_inicio, fecha_fin)?,
        })
    }
}

fn utilidad_por_vehiculo(
    conn: &mut PooledConnection,
    inicio: &str,
    fin: &str,
) -> Result<Vec<UtilidadVehiculo>, AppError> {
    let mut mapa: HashMap<String, (Decimal, Decimal)> = HashMap::new();
    let vehiculos: HashMap<String, String> =
        InformeRepository::vehiculos(conn)?.into_iter().collect();

    // 1 sola query (UNION ALL) en vez de 6 round-trips por separado.
    // Cada fila trae (placa, tipo, monto); INGRESO/ABONO suman a ingresos,
    // GASTO/COMISION/MANT/COMP suman a costos.
    for (placa, tipo, monto) in InformeRepository::movimientos_por_placa(conn, inicio, fin)? {
        if placa.is_empty() {
            continue;
        }
        let m = dec(&monto);
        let e = mapa.entry(placa).or_insert((Decimal::ZERO, Decimal::ZERO));
        match tipo.as_str() {
            "INGRESO" | "ABONO" => e.0 += m,
            _ => e.1 += m,
        }
    }

    let mut filas: Vec<UtilidadVehiculo> = mapa
        .into_iter()
        .filter(|(_, (ingresos, costos))| *ingresos > Decimal::ZERO || *costos > Decimal::ZERO)
        .map(|(placa, (ingresos, costos))| {
            let utilidad = (ingresos - costos).round_dp(2);
            UtilidadVehiculo {
                placa: placa.clone(),
                vehiculo: vehiculos.get(&placa).cloned().unwrap_or_default(),
                ingresos: ingresos.round_dp(2).to_string(),
                costos: costos.round_dp(2).to_string(),
                utilidad: utilidad.to_string(),
            }
        })
        .collect();

    filas.sort_by(|a, b| b.utilidad.cmp(&a.utilidad));
    Ok(filas)
}

fn dec(v: &str) -> Decimal {
    Decimal::from_str(v.trim()).unwrap_or(Decimal::ZERO)
}

fn sum(vals: &[&str]) -> String {
    vals.iter()
        .fold(Decimal::ZERO, |acc, v| acc + dec(v))
        .round_dp(2)
        .to_string()
}
