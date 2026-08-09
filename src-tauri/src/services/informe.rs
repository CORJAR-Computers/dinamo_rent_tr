//! services/informe.rs — Balance por rango de fechas.
//!
//! Ingresos: pagos de rentas + abonos de reservas del rango.
//! Egresos: gastos + mantenimientos + comparendos del rango.
//! El balance = ingresos − egresos.

use std::collections::HashMap;

use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr as _;
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
    /// Calcula el balance del rango de fechas indicado
    pub fn mensual(conn: &mut PooledConnection, fecha_inicio: &str, fecha_fin: &str) -> Result<InformeMensual, AppError> {
        let ingresos_pagos = InformeRepository::ingresos_pagos(conn, fecha_inicio, fecha_fin)?;
        let ingresos_reservas = InformeRepository::ingresos_abonos_reservas(conn, fecha_inicio, fecha_fin)?;
        let egresos_gastos = InformeRepository::egresos_gastos(conn, fecha_inicio, fecha_fin)?;
        let egresos_mantenimiento = InformeRepository::egresos_mantenimiento(conn, fecha_inicio, fecha_fin)?;
        let egresos_comparendos = InformeRepository::egresos_comparendos(conn, fecha_inicio, fecha_fin)?;

        let total_ingresos = sum(&[&ingresos_pagos, &ingresos_reservas]);
        let total_egresos = sum(&[&egresos_gastos, &egresos_mantenimiento, &egresos_comparendos]);
        let balance = (dec(&total_ingresos) - dec(&total_egresos)).round_dp(2);

        let rentas = InformeRepository::rentas_del_mes(conn, fecha_inicio, fecha_fin)?
            .into_iter()
            .map(|(id, placa, nombre_cliente, total, estado, fecha_recogida)| RentaInforme {
                id,
                placa,
                nombre_cliente,
                total,
                estado,
                fecha_recogida,
            })
            .collect();

        Ok(InformeMensual {
            fecha_inicio: fecha_inicio.to_string(),
            fecha_fin: fecha_fin.to_string(),
            ingresos_pagos,
            ingresos_reservas,
            total_ingresos: total_ingresos.clone(),
            egresos_gastos,
            egresos_mantenimiento,
            egresos_comparendos,
            total_egresos: total_egresos.clone(),
            balance: balance.to_string(),
            gastos_por_categoria: InformeRepository::gastos_por_categoria(conn, fecha_inicio, fecha_fin)?,
            rentas,
            utilidad_por_vehiculo: utilidad_por_vehiculo(conn, fecha_inicio, fecha_fin)?,
        })
    }
}

fn utilidad_por_vehiculo(conn: &mut PooledConnection, inicio: &str, fin: &str) -> Result<Vec<UtilidadVehiculo>, AppError> {
    let mut mapa: HashMap<String, (Decimal, Decimal)> = HashMap::new();
    let mut acumular = |placa: String, monto: &str, ingresos: bool| {
        let m = dec(monto);
        let e = mapa.entry(placa).or_insert((Decimal::ZERO, Decimal::ZERO));
        if ingresos {
            e.0 += m;
        } else {
            e.1 += m;
        }
    };

    for (placa, total) in InformeRepository::ingresos_por_placa(conn, inicio, fin)? {
        acumular(placa, &total, true);
    }
    for (placa, total) in InformeRepository::abonos_reservas_por_placa(conn, inicio, fin)? {
        acumular(placa, &total, true);
    }
    for (placa, total) in InformeRepository::gastos_por_placa(conn, inicio, fin)? {
        acumular(placa, &total, false);
    }
    for (placa, total) in InformeRepository::mantenimiento_por_placa(conn, inicio, fin)? {
        acumular(placa, &total, false);
    }
    for (placa, total) in InformeRepository::comparendos_por_placa(conn, inicio, fin)? {
        acumular(placa, &total, false);
    }

    let vehiculos: HashMap<String, String> = InformeRepository::vehiculos(conn)?
        .into_iter()
        .collect();

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
