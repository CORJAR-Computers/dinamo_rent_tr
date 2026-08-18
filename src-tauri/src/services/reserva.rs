//! services/reserva.rs — Lógica de negocio de reservas (puerto de reserva_service.py)
//!
//! Valida fechas/horas/tarifas, calcula el total (días × valor día + horas × valor hora),
//! autocompleta el cliente (nombre/nacionalidad) desde la tabla clientes y gestiona
//! la cancelación con reglas de negocio.

use std::sync::Arc;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr as _;
use serde::Serialize;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::validators::{validate_no_xss, mayusculas};
use crate::core::PooledConnection;
use crate::repositories::cliente::ClienteRepository;
use crate::repositories::reserva::{Reserva, ReservaDatos, ReservaRepository};

/// Estados de reserva por defecto cuando config.ini no define `business.estados_reserva`
const ESTADOS_RESERVA_FALLBACK: [&str; 4] = ["Pendiente", "Confirmada", "Cancelada", "Completada"];

/// Resultado de cancelación (para la UI)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservaCancelada {
    pub reserva: Reserva,
    pub cancelada: bool,
}

pub struct ReservaService;

impl ReservaService {
    /// Lista reservas con filtros opcionales (búsqueda libre o por estado)
    pub fn listar(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
        estado: Option<&str>,
    ) -> Result<Vec<Reserva>, AppError> {
        let term = busqueda.unwrap_or("").trim();
        if !term.is_empty() {
            ReservaRepository::buscar(conn, term)
        } else if let Some(estado) = estado.filter(|e| !e.trim().is_empty() && e.trim() != "Todos") {
            ReservaRepository::obtener_por_estado(conn, estado.trim())
        } else {
            ReservaRepository::obtener_todos(conn)
        }
    }

    /// Próximas reservas (recogida hoy o en el futuro, no canceladas)
    pub fn proximas(conn: &mut PooledConnection, limit: i64) -> Result<Vec<Reserva>, AppError> {
        ReservaRepository::obtener_proximas(conn, limit)
    }

    /// Obtiene una reserva por id
    pub fn obtener(conn: &mut PooledConnection, id: i64) -> Result<Reserva, AppError> {
        ReservaRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe la reserva #{id}")))
    }

    /// Crea una reserva (autocompleta cliente y recalcula el total)
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        mut datos: ReservaDatos,
    ) -> Result<Reserva, AppError> {
        normalizar(&mut datos);
        completar_cliente(conn, &mut datos);
        calcular_total(&mut datos);
        validar(&datos, cfg)?;
        let id = ReservaRepository::insertar(conn, &datos)?;
        Self::obtener(conn, id)
    }

    /// Actualiza una reserva por id
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        id: i64,
        mut datos: ReservaDatos,
    ) -> Result<Reserva, AppError> {
        Self::obtener(conn, id)?;
        normalizar(&mut datos);
        completar_cliente(conn, &mut datos);
        calcular_total(&mut datos);
        validar(&datos, cfg)?;
        ReservaRepository::actualizar(conn, id, &datos)?;
        Self::obtener(conn, id)
    }

    /// Cancela una reserva (no se puede cancelar una ya completada)
    pub fn cancelar(conn: &mut PooledConnection, id: i64) -> Result<ReservaCancelada, AppError> {
        let actual = Self::obtener(conn, id)?;
        if actual.estado == "Cancelada" {
            return Ok(ReservaCancelada { reserva: actual, cancelada: false });
        }
        if actual.estado == "Completada" {
            return Err(AppError::Business(
                "No se puede cancelar una reserva ya completada.".into(),
            ));
        }
        ReservaRepository::cambiar_estado(&mut **conn, id, "Cancelada")?;
        let reserva = Self::obtener(conn, id)?;
        Ok(ReservaCancelada { reserva, cancelada: true })
    }

    /// Elimina una reserva (las rentas asociadas quedan con id_reserva NULL)
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        Self::obtener(conn, id)?;
        ReservaRepository::eliminar(conn, id)
    }

    /// Total de reservas (dashboard)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        ReservaRepository::contar(conn)
    }

    /// Conteo por estado (dashboard)
    pub fn contar_por_estado(conn: &mut PooledConnection) -> Result<Vec<(String, i64)>, AppError> {
        ReservaRepository::contar_por_estado(conn)
    }
}

/// Normaliza campos (trim → mayúsculas, defaults)
fn normalizar(d: &mut ReservaDatos) {
    d.nombre_cliente = mayusculas(&d.nombre_cliente);
    d.nacionalidad = d.nacionalidad.as_ref().map(|s| mayusculas(s)).filter(|s| !s.is_empty());
    d.categoria_vehiculo = d.categoria_vehiculo.as_ref().map(|s| mayusculas(s)).filter(|s| !s.is_empty());
    d.placa_asignada = d.placa_asignada.as_ref().map(|s| mayusculas(s)).filter(|s| !s.is_empty());
    d.ubicacion_recogida = d.ubicacion_recogida.as_ref().map(|s| mayusculas(s)).filter(|s| !s.is_empty());
    d.ubicacion_retorno = d.ubicacion_retorno.as_ref().map(|s| mayusculas(s)).filter(|s| !s.is_empty());
    d.observaciones = d.observaciones.as_ref().map(|s| mayusculas(s)).filter(|s| !s.is_empty());
    // Montos: vacío → "0.00" (evita SQLCODE -303 al enlazar '' a DECIMAL)
    for m in [&mut d.valor_dia, &mut d.valor_hora_adic, &mut d.abono] {
        *m = m.trim().replace(',', ".");
        if m.is_empty() {
            *m = "0.00".into();
        }
    }
    if d.estado.trim().is_empty() {
        d.estado = "Confirmada".into();
    }
}

/// Autocompleta nombre_cliente y nacionalidad desde la tabla clientes cuando hay id_cliente
fn completar_cliente(conn: &mut PooledConnection, d: &mut ReservaDatos) {
    if let Some(idc) = d.id_cliente {
        if let Ok(Some(c)) = ClienteRepository::obtener_por_id(conn, idc) {
            d.nombre_cliente = c.nombre_completo;
            d.nacionalidad = c.nacionalidad;
        }
    }
}

/// Recalcula el total desde las tarifas (el backend es la fuente de verdad)
fn calcular_total(d: &mut ReservaDatos) {
    let dias = d.dias_calculados.max(0);
    let horas = d.horas_extras.max(0);
    let vdia = Decimal::from_str(&d.valor_dia).unwrap_or(Decimal::ZERO).max(Decimal::ZERO);
    let vha = Decimal::from_str(&d.valor_hora_adic).unwrap_or(Decimal::ZERO).max(Decimal::ZERO);
    let total = vdia * Decimal::from(dias) + vha * Decimal::from(horas);
    d.total = total.round_dp(2).to_string();
}

/// Valida los datos de la reserva
fn validar(d: &ReservaDatos, cfg: &Arc<AppConfig>) -> Result<(), AppError> {
    if d.nombre_cliente.is_empty() || d.nombre_cliente.len() > 200 {
        return Err(AppError::Validation(
            "El nombre del cliente es obligatorio (máx. 200 caracteres).".into(),
        ));
    }
    validate_no_xss(&d.nombre_cliente, 200).map_err(|_| {
        AppError::Validation("El nombre del cliente contiene caracteres no permitidos.".into())
    })?;

    // Fechas
    let recogida = NaiveDate::parse_from_str(&d.fecha_recogida, "%Y-%m-%d").map_err(|_| {
        AppError::Validation("La fecha de recogida no es válida (formato AAAA-MM-DD).".into())
    })?;
    let retorno = NaiveDate::parse_from_str(&d.fecha_retorno, "%Y-%m-%d").map_err(|_| {
        AppError::Validation("La fecha de retorno no es válida (formato AAAA-MM-DD).".into())
    })?;
    if retorno < recogida {
        return Err(AppError::Validation(
            "La fecha de retorno no puede ser anterior a la fecha de recogida.".into(),
        ));
    }
    // Horas (opcionales, formato HH:MM[:SS])
    for (campo, hora) in [
        ("hora de recogida", &d.hora_recogida),
        ("hora de retorno", &d.hora_retorno),
    ] {
        if let Some(h) = hora {
            if !h.is_empty() && !es_hora_valida(h) {
                return Err(AppError::Validation(format!(
                    "La {campo} no es válida (formato HH:MM)."
                )));
            }
        }
    }

    // Días y horas
    if d.dias_calculados < 0 {
        return Err(AppError::Validation("Los días calculados no pueden ser negativos.".into()));
    }
    if d.horas_extras < 0 {
        return Err(AppError::Validation("Las horas extras no pueden ser negativas.".into()));
    }

    // Tarifas
    let vdia = Decimal::from_str(&d.valor_dia).unwrap_or_else(|_| Decimal::from(-1));
    let vha = Decimal::from_str(&d.valor_hora_adic).unwrap_or_else(|_| Decimal::from(-1));
    if vdia < Decimal::ZERO {
        return Err(AppError::Validation("El valor del día no es un número válido.".into()));
    }
    if vha < Decimal::ZERO {
        return Err(AppError::Validation("El valor de la hora adicional no es un número válido.".into()));
    }
    let abono = Decimal::from_str(&d.abono).unwrap_or_else(|_| Decimal::from(-1));
    if abono < Decimal::ZERO {
        return Err(AppError::Validation("El abono no es un número válido.".into()));
    }
    let total = Decimal::from_str(&d.total).unwrap_or_else(|_| Decimal::from(-1));
    if abono > total {
        return Err(AppError::Validation(
            "El abono no puede ser mayor que el total de la reserva.".into(),
        ));
    }

    // Estado
    let estados: Vec<&str> = if cfg.estados_reserva.is_empty() {
        ESTADOS_RESERVA_FALLBACK.to_vec()
    } else {
        cfg.estados_reserva.iter().map(|s| s.as_str()).collect()
    };
    if !estados.contains(&d.estado.as_str()) {
        return Err(AppError::Validation(format!(
            "Estado inválido '{}'. Permitidos: {}",
            d.estado,
            estados.join(", ")
        )));
    }

    // Campos de texto libre (consistente con autos/clientes/usuarios)
    for (campo, val) in [
        ("la nacionalidad", d.nacionalidad.as_deref()),
        ("la categoría del vehículo", d.categoria_vehiculo.as_deref()),
        ("la placa asignada", d.placa_asignada.as_deref()),
        ("la ubicación de recogida", d.ubicacion_recogida.as_deref()),
        ("la ubicación de retorno", d.ubicacion_retorno.as_deref()),
        ("las observaciones", d.observaciones.as_deref()),
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

/// ¿Hora válida en formato HH:MM o HH:MM:SS?
fn es_hora_valida(h: &str) -> bool {
    let parts: Vec<&str> = h.split(':').collect();
    if parts.len() != 2 && parts.len() != 3 {
        return false;
    }
    for p in &parts {
        if p.len() != 2 || !p.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    let hh: u32 = parts[0].parse().unwrap_or(99);
    let mm: u32 = parts[1].parse().unwrap_or(99);
    if hh > 23 || mm > 59 {
        return false;
    }
    if parts.len() == 3 {
        let ss: u32 = parts[2].parse().unwrap_or(99);
        if ss > 59 {
            return false;
        }
    }
    true
}
