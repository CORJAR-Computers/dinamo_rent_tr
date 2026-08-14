//! services/renta.rs — Lógica de negocio de rentas
//!
//! Valida fechas/horas/tarifas y costos extras, calcula el total
//! (días × valor día + horas × valor hora + extras − descuento, más impuestos),
//! gestiona el cierre con devolución real, los pagos (abono/saldo) y las
//! inspecciones de salida/entrada.

use std::sync::Arc;

use chrono::{NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr as _;
use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};
use serde::Serialize;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::validators::validate_no_xss;
use crate::core::PooledConnection;
use crate::repositories::cliente::ClienteRepository;
use crate::repositories::renta::{
    Inspeccion, InspeccionDatos, Pago, PagoDatos, Renta, RentaCierreDatos, RentaDatos, RentaRepository,
};

/// Construye parámetros posicionales de cualquier longitud (tuplas `IntoParams`
/// limitadas a 15 elementos en rsfbclient).
macro_rules! params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into_param()),+])
    };
}

/// Resultado de cancelación (para la UI)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RentaCancelada {
    pub renta: Renta,
    pub cancelada: bool,
}

pub struct RentaService;

impl RentaService {
    /// Lista rentas con filtros opcionales (búsqueda libre, estado o placa)
    pub fn listar(
        conn: &mut PooledConnection,
        busqueda: Option<&str>,
        estado: Option<&str>,
        placa: Option<&str>,
    ) -> Result<Vec<Renta>, AppError> {
        let term = busqueda.unwrap_or("").trim();
        let est = estado.unwrap_or("").trim();
        let plac = placa.unwrap_or("").trim();
        if !term.is_empty() {
            RentaRepository::buscar(conn, term)
        } else if !est.is_empty() && est != "Todos" {
            RentaRepository::obtener_por_estado(conn, est)
        } else if !plac.is_empty() {
            RentaRepository::obtener_por_placa(conn, plac)
        } else {
            RentaRepository::obtener_todos(conn)
        }
    }

    /// Obtiene una renta por id (con pagos e inspecciones)
    pub fn obtener(conn: &mut PooledConnection, id: i64) -> Result<Renta, AppError> {
        RentaRepository::obtener_por_id(conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("No existe la renta #{id}")))
    }

    /// Crea una renta (autocompleta cliente, calcula totales)
    pub fn crear(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        mut datos: RentaDatos,
    ) -> Result<Renta, AppError> {
        normalizar(&mut datos);
        completar_cliente(conn, &mut datos);
        calcular_totales(&mut datos, cfg);
        // El abono inicial descuenta del saldo: saldo = total − abono
        datos.saldo_pendiente =
            (dec(&datos.total, "0.00") - dec(&datos.abono, "0.00")).max(Decimal::ZERO).round_dp(2).to_string();
        validar(&datos, cfg)?;
        let id = RentaRepository::insertar(conn, &datos)?;
        Self::obtener(conn, id)
    }

    /// Actualiza los datos de la renta (los totales se conservan y se recalculan
    /// en el cierre; el estado no se modifica aquí)
    pub fn actualizar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        id: i64,
        mut datos: RentaDatos,
    ) -> Result<Renta, AppError> {
        let actual = Self::obtener(conn, id)?;
        if actual.estado == "Cerrada" {
            return Err(AppError::Business(
                "No se puede editar una renta cerrada. Regístrala como nueva si es necesario.".into(),
            ));
        }
        normalizar(&mut datos);
        completar_cliente(conn, &mut datos);
        calcular_totales(&mut datos, cfg);
        // Conservar el abono ya registrado: saldo = total − abono actual
        datos.saldo_pendiente =
            (dec(&datos.total, "0.00") - dec_str(&actual.abono)).max(Decimal::ZERO).round_dp(2).to_string();
        validar(&datos, cfg)?;
        RentaRepository::actualizar(conn, id, &datos)?;
        Self::obtener(conn, id)
    }

    /// Cambia el vehículo asignado a una renta activa sin cerrarla. En una sola
    /// transacción: libera el auto anterior (solo si no está en otra renta
    /// activa) y marca el nuevo como Rentado. El vehículo nuevo debe existir y
    /// estar Disponible.
    pub fn cambiar_auto(
        conn: &mut PooledConnection,
        id: i64,
        placa_nueva: &str,
        usuario: &str,
    ) -> Result<Renta, AppError> {
        let actual = Self::obtener(conn, id)?;
        if actual.estado == "Cerrada" || actual.estado == "Cancelada" {
            return Err(AppError::Business(
                "No se puede cambiar el vehículo de una renta cerrada o cancelada.".into(),
            ));
        }
        let nueva = placa_nueva.trim().to_uppercase();
        if nueva.is_empty() {
            return Err(AppError::Validation(
                "Debes seleccionar un vehículo para el cambio.".into(),
            ));
        }
        if actual.placa.as_deref() == Some(nueva.as_str()) {
            return Ok(actual); // mismo vehículo: no-op
        }
        // El vehículo nuevo debe existir y estar Disponible
        let estado: Option<(String,)> = conn.query_first(
            "SELECT estado FROM autos WHERE placa = ?",
            (nueva.clone(),),
        )?;
        let Some((estado,)) = estado else {
            return Err(AppError::Business(format!(
                "El vehículo {nueva} no existe."
            )));
        };
        if estado != "Disponible" {
            return Err(AppError::Business(format!(
                "El vehículo {nueva} no está disponible (estado: {estado})."
            )));
        }
        let placa_anterior = actual.placa.clone();
        let mensaje_audit = format!(
            "renta={id}, {} → {}",
            placa_anterior.as_deref().unwrap_or("-"),
            nueva
        );
        conn.with_transaction(|tx| -> Result<(), rsfbclient::FbError> {
            // Liberar el auto anterior solo si está en estado Rentado Y no está
            // en otra renta activa (no sobrescribe marcas manuales como
            // Mantenimiento/Vendido).
            if let Some(old) = placa_anterior.as_ref() {
                tx.execute(
                    "UPDATE autos SET estado = 'Disponible', updated_at = CURRENT_TIMESTAMP \
                     WHERE placa = ? AND estado = 'Rentado' AND NOT EXISTS (\
                        SELECT 1 FROM rentas WHERE placa = ? AND estado IN ('Activa', 'Activo') AND id <> ?)",
                    (old.clone(), old.clone(), id),
                )?;
            }
            tx.execute(
                "UPDATE rentas SET placa = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                (nueva.clone(), id),
            )?;
            tx.execute(
                "UPDATE autos SET estado = 'Rentado', updated_at = CURRENT_TIMESTAMP WHERE placa = ?",
                (nueva.clone(),),
            )?;
            tx.execute(
                "INSERT INTO auditoria (usuario, accion, mensaje, ip, fecha) \
                 VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
                (
                    usuario.to_string(),
                    "CAMBIO AUTO".to_string(),
                    mensaje_audit.clone(),
                    "local".to_string(),
                ),
            )?;
            Ok(())
        })
        .map_err(|e| AppError::Database(e.to_string()))?;
        Self::obtener(conn, id)
    }

    /// Cierra una renta: registra la devolución real y recalcula los totales
    pub fn cerrar(
        conn: &mut PooledConnection,
        cfg: &Arc<AppConfig>,
        id: i64,
        mut datos: RentaCierreDatos,
    ) -> Result<Renta, AppError> {
        let actual = Self::obtener(conn, id)?;
        if actual.estado == "Cerrada" {
            return Err(AppError::Business("La renta ya está cerrada.".into()));
        }
        if actual.estado == "Cancelada" {
            return Err(AppError::Business(
                "No se puede cerrar una renta cancelada.".into(),
            ));
        }
        // Normaliza y valida los datos del cierre
        normalizar_cierre(&mut datos);
        validar_cierre(&datos)?;

        // Totales con los valores del cierre (o los de la renta si no se ajustan).
        // Si el operador no envió días/horas y la devolución real tiene fecha y
        // hora, se calculan automáticamente (regla: excedente > 3 h = día completo).
        let (dias_auto, horas_auto) = calcular_dias_horas(
            &actual.fecha_recogida,
            actual.hora_recogida.as_deref(),
            datos.fecha_devolucion_real.as_deref(),
            datos.hora_devolucion_real.as_deref(),
        )
        .unwrap_or((actual.dias_calculados, actual.horas_extras));
        let dias = datos.dias_calculados.unwrap_or(dias_auto).max(0);
        let horas = datos.horas_extras.unwrap_or(horas_auto).max(0);
        let vdia = dec(datos.valor_dia.as_deref().unwrap_or(""), &actual.valor_dia);
        let vhe = dec(datos.valor_hora_extra.as_deref().unwrap_or(""), &actual.valor_hora_extra);
        let desc = dec(datos.descuento.as_deref().unwrap_or(""), &actual.descuento);

        let bruto = vdia * Decimal::from(dias) + vhe * Decimal::from(horas);
        let extras = sum_dec(&[
            &actual.valor_dia_extra,
            &actual.costo_lavado,
            &actual.costo_silla,
            &actual.costo_retorno,
            &actual.costo_domicilio,
            &actual.costo_cables,
            &actual.costo_inversor,
        ]);
        let subtotal = (bruto + extras - desc).max(Decimal::ZERO);
        // IVA según el flag guardado en la renta (checkbox del formulario)
        let imp = if actual.cobra_iva { impuesto(cfg) } else { Decimal::ZERO };
        let impuestos = (subtotal * imp).round_dp(2);
        let total = subtotal + impuestos;
        let abono = dec_str(&actual.abono);
        let saldo = (total - abono).max(Decimal::ZERO);

        // Pre-calcular los valores parseados fuera del closure (la transacción
        // sólo propaga `FbError`, no `AppError`).
        let fecha_dev = parse_fecha_opt(&datos.fecha_devolucion_real)?;
        let hora_dev = parse_hora(&datos.hora_devolucion_real)?;
        let km_final = opt_str(&datos.km_final);
        let tanque_final = opt_str(&datos.tanque_final);
        let valor_dia = datos.valor_dia.as_deref().map(|s| s.trim().replace(',', "."));
        let valor_hora_extra = datos.valor_hora_extra.as_deref().map(|s| s.trim().replace(',', "."));
        let descuento = datos.descuento.as_deref().map(|s| s.trim().replace(',', "."));
        let observaciones = opt_str(&datos.observaciones);
        let subtotal_s = subtotal.round_dp(2).to_string();
        let impuestos_s = impuestos.to_string();
        let total_s = total.round_dp(2).to_string();
        let saldo_s = saldo.round_dp(2).to_string();
        let placa_auto = actual.placa.clone();
        let usuario_audit = "sistema".to_string();

        // TRANSACCIÓN: UPDATE rentas (estado Cerrada + devolución) + UPDATE autos
        // (liberar vehículo) + INSERT auditoría. Atómico: si cualquiera falla,
        // rollback y la renta queda en su estado anterior.
        conn.with_transaction(|tx| -> Result<(), rsfbclient::FbError> {
            tx.execute(
                "UPDATE rentas SET \
                    estado = 'Cerrada', \
                    fecha_devolucion_real = ?, hora_devolucion_real = ?, km_final = ?, tanque_final = ?, \
                    dias_calculados = COALESCE(?, dias_calculados), \
                    horas_extras = COALESCE(?, horas_extras), \
                    valor_dia = CAST(COALESCE(?, valor_dia) AS DECIMAL(12,2)), \
                    valor_hora_extra = CAST(COALESCE(?, valor_hora_extra) AS DECIMAL(12,2)), \
                    descuento = CAST(COALESCE(?, descuento) AS DECIMAL(12,2)), \
                    subtotal = CAST(? AS DECIMAL(12,2)), impuestos = CAST(? AS DECIMAL(12,2)), \
                    total = CAST(? AS DECIMAL(12,2)), saldo_pendiente = CAST(? AS DECIMAL(12,2)), \
                    observaciones = COALESCE(?, observaciones) \
                 WHERE id = ?",
                params![
                    fecha_dev,
                    hora_dev,
                    km_final,
                    tanque_final,
                    dias,
                    horas,
                    valor_dia,
                    valor_hora_extra,
                    descuento,
                    subtotal_s,
                    impuestos_s,
                    total_s,
                    saldo_s,
                    observaciones,
                    id,
                ],
            )?;
            // Liberar el vehículo (sólo si la renta tenía placa asignada)
            if let Some(placa) = placa_auto.as_ref() {
                tx.execute(
                    "UPDATE autos SET estado = 'Disponible', updated_at = CURRENT_TIMESTAMP \
                     WHERE placa = ?",
                    (placa.clone(),),
                )?;
            }
            // Auditoría del cierre
            tx.execute(
                "INSERT INTO auditoria (usuario, accion, mensaje, ip, fecha) \
                 VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
                (
                    usuario_audit.clone(),
                    "CIERRE RENTA".to_string(),
                    format!("renta={id}, placa={}", placa_auto.as_deref().unwrap_or("-")),
                    "local".to_string(),
                ),
            )?;
            Ok(())
        })
        .map_err(|e| AppError::Database(e.to_string()))?;
        Self::obtener(conn, id)
    }

    /// Cancela una renta activa (no las cerradas)
    pub fn cancelar(conn: &mut PooledConnection, id: i64) -> Result<RentaCancelada, AppError> {
        let actual = Self::obtener(conn, id)?;
        if actual.estado == "Cancelada" {
            return Ok(RentaCancelada { renta: actual, cancelada: false });
        }
        if actual.estado == "Cerrada" {
            return Err(AppError::Business(
                "No se puede cancelar una renta cerrada.".into(),
            ));
        }
        RentaRepository::cancelar(conn, id)?;
        let renta = Self::obtener(conn, id)?;
        Ok(RentaCancelada { renta, cancelada: true })
    }

    /// Elimina una renta (soft-delete: marca deleted_at en rentas y pagos).
    /// Las inspecciones no tienen deleted_at; dejan de ser accesibles porque la
    /// renta no aparece en los SELECTs. Ver RentaRepository::eliminar.
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        Self::obtener(conn, id)?;
        RentaRepository::eliminar(conn, id)
    }

    /// Registra un pago contra una renta activa y actualiza abono/saldo
    pub fn registrar_pago(
        conn: &mut PooledConnection,
        id_renta: i64,
        usuario: &str,
        mut pago: PagoDatos,
    ) -> Result<Pago, AppError> {
        let renta = Self::obtener(conn, id_renta)?;
        if renta.estado != "Activa" && renta.estado != "Activo" {
            return Err(AppError::Business(
                "Solo se pueden registrar pagos en rentas activas.".into(),
            ));
        }
        pago.monto = pago.monto.trim().replace(',', ".");
        pago.metodo_pago = pago.metodo_pago.trim().to_string();
        pago.concepto = pago.concepto.trim().to_string();
        let monto = Decimal::from_str(&pago.monto).map_err(|_| {
            AppError::Validation("El monto del pago no es un número válido.".into())
        })?;
        if monto <= Decimal::ZERO {
            return Err(AppError::Validation("El monto del pago debe ser mayor a cero.".into()));
        }
        if pago.metodo_pago.is_empty() {
            return Err(AppError::Validation("El método de pago es obligatorio.".into()));
        }
        if pago.concepto.is_empty() || pago.concepto.len() > 80 {
            return Err(AppError::Validation(
                "El concepto es obligatorio (máx. 80 caracteres).".into(),
            ));
        }
        validate_no_xss(&pago.metodo_pago, 50).map_err(|_| {
            AppError::Validation("El método de pago contiene caracteres no permitidos.".into())
        })?;
        validate_no_xss(&pago.concepto, 80).map_err(|_| {
            AppError::Validation("El concepto contiene caracteres no permitidos.".into())
        })?;

        let saldo_actual = dec_str(&renta.saldo_pendiente);
        if monto > saldo_actual {
            return Err(AppError::Business(format!(
                "El pago ({}) supera el saldo pendiente ({}).",
                monto.round_dp(2),
                saldo_actual.round_dp(2)
            )));
        }
        let abono_nuevo = (dec_str(&renta.abono) + monto).round_dp(2);
        let saldo_nuevo = (saldo_actual - monto).round_dp(2);
        // TRANSACCIÓN: INSERT pago + UPDATE rentas (abono/saldo) + INSERT auditoría.
        // Atómico: si cualquiera falla, se hace rollback y no queda pago huérfano
        // ni abono desincronizado. Patrón: conn.with_transaction(|tx| -> Result<T, FbError>).
        let id = conn
            .with_transaction(|tx| -> Result<i64, rsfbclient::FbError> {
                let (id,): (i64,) = tx.execute_returnable(
                    "INSERT INTO pagos (id_renta, monto, metodo_pago, concepto, observaciones, usuario) \
                     VALUES (?, CAST(? AS DECIMAL(12,2)), ?, ?, ?, ?) RETURNING id",
                    params![
                        id_renta,
                        pago.monto.to_string(),
                        pago.metodo_pago.to_string(),
                        pago.concepto.to_string(),
                        opt_str(&pago.observaciones),
                        usuario.to_string(),
                    ],
                )?;
                tx.execute(
                    "UPDATE rentas SET abono = CAST(? AS DECIMAL(12,2)), \
                     saldo_pendiente = CAST(? AS DECIMAL(12,2)) WHERE id = ?",
                    (
                        abono_nuevo.to_string(),
                        saldo_nuevo.to_string(),
                        id_renta,
                    ),
                )?;
                tx.execute(
                    "INSERT INTO auditoria (usuario, accion, mensaje, ip, fecha) \
                     VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
                    (
                        usuario.to_string(),
                        "PAGO RENTA".to_string(),
                        format!("renta={id_renta}, pago={id}, monto={}", pago.monto),
                        "local".to_string(),
                    ),
                )?;
                Ok(id)
            })
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(RentaRepository::pagos_de(conn, id_renta)?
            .into_iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| Pago {
                id,
                id_renta,
                fecha: String::new(),
                monto: pago.monto,
                metodo_pago: pago.metodo_pago.clone(),
                concepto: pago.concepto.clone(),
                observaciones: pago.observaciones.clone(),
                usuario: Some(usuario.to_string()),
            }))
    }

    /// Registra una inspección (Salida/Entrada) de una renta
    pub fn registrar_inspeccion(
        conn: &mut PooledConnection,
        id_renta: i64,
        mut inspeccion: InspeccionDatos,
    ) -> Result<Inspeccion, AppError> {
        let renta = Self::obtener(conn, id_renta)?;
        if renta.estado == "Cerrada" || renta.estado == "Cancelada" {
            return Err(AppError::Business(
                "No se pueden registrar inspecciones en rentas cerradas o canceladas.".into(),
            ));
        }
        inspeccion.tipo = inspeccion.tipo.trim().to_string();
        inspeccion.kilometraje = inspeccion.kilometraje.trim().to_string();
        inspeccion.nivel_gasolina = inspeccion.nivel_gasolina.trim().to_string();
        if inspeccion.tipo != "Salida" && inspeccion.tipo != "Entrada" {
            return Err(AppError::Validation(
                "El tipo de inspección debe ser 'Salida' o 'Entrada'.".into(),
            ));
        }
        if inspeccion.kilometraje.parse::<f64>().is_err() {
            return Err(AppError::Validation("El kilometraje no es un número válido.".into()));
        }
        if inspeccion.nivel_gasolina.is_empty() {
            return Err(AppError::Validation("El nivel de gasolina es obligatorio.".into()));
        }
        let id = RentaRepository::insertar_inspeccion(conn, id_renta, &inspeccion)?;
        Ok(RentaRepository::inspecciones_de(conn, id_renta)?
            .into_iter()
            .find(|i| i.id == id)
            .unwrap_or_else(|| Inspeccion {
                id,
                id_renta,
                tipo: inspeccion.tipo,
                fecha: String::new(),
                kilometraje: inspeccion.kilometraje,
                nivel_gasolina: inspeccion.nivel_gasolina,
                limpieza: inspeccion.limpieza,
                tiene_repuesto: inspeccion.tiene_repuesto,
                tiene_gato_cruceta: inspeccion.tiene_gato_cruceta,
                tiene_kit_carretera: inspeccion.tiene_kit_carretera,
                tiene_documentos: inspeccion.tiene_documentos,
                danos_carroceria: inspeccion.danos_carroceria,
                observaciones: inspeccion.observaciones,
            }))
    }

    /// Rentas activas (para el calendario y el dashboard)
    pub fn activas(conn: &mut PooledConnection) -> Result<Vec<Renta>, AppError> {
        RentaRepository::activas(conn)
    }

    /// Total de rentas (dashboard)
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        RentaRepository::contar(conn)
    }

    /// Conteo por estado (dashboard)
    pub fn contar_por_estado(conn: &mut PooledConnection) -> Result<Vec<(String, i64)>, AppError> {
        RentaRepository::contar_por_estado(conn)
    }
}

/// Normaliza campos (trim, placa a mayúsculas, montos con coma → punto)
fn normalizar(d: &mut RentaDatos) {
    d.placa = d.placa.as_ref().map(|s| s.trim().to_uppercase()).filter(|s| !s.is_empty());
    d.nombre_cliente = d.nombre_cliente.trim().to_string();
    d.no_licencia = d.no_licencia.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.nacionalidad = d.nacionalidad.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.ubicacion_recogida = d.ubicacion_recogida.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.ubicacion_retorno = d.ubicacion_retorno.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.observaciones = d.observaciones.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.tanque_salida = d.tanque_salida.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    // Montos: vacío → "0.00". Sin esto, un campo monetario en blanco se enlaza
    // como '' a CAST(? AS DECIMAL) y Firebird falla con SQLCODE -303
    // "conversion error from string ''" (error real visto en producción al
    // crear/editar rentas). Incluye `abono`, que antes quedaba fuera de la lista.
    for m in [
        &mut d.valor_dia,
        &mut d.valor_hora_extra,
        &mut d.valor_dia_extra,
        &mut d.costo_lavado,
        &mut d.costo_silla,
        &mut d.costo_retorno,
        &mut d.costo_domicilio,
        &mut d.costo_cables,
        &mut d.costo_inversor,
        &mut d.valor_gasolina,
        &mut d.descuento,
        &mut d.abono,
    ] {
        *m = m.trim().replace(',', ".");
        if m.is_empty() {
            *m = "0.00".into();
        }
    }
    if d.km_salida.trim().is_empty() {
        d.km_salida = "0".into();
    }
}

fn normalizar_cierre(d: &mut RentaCierreDatos) {
    d.km_final = d.km_final.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.tanque_final = d.tanque_final.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    d.observaciones = d.observaciones.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    for m in [&mut d.valor_dia, &mut d.valor_hora_extra, &mut d.descuento] {
        if let Some(v) = m {
            *v = v.trim().replace(',', ".");
            if v.is_empty() {
                *m = None; // vacío = "mantener" (COALESCE conserva el valor actual)
            }
        }
    }
}

/// Autocompleta nombre_cliente y nacionalidad desde la tabla clientes
fn completar_cliente(conn: &mut PooledConnection, d: &mut RentaDatos) {
    if let Some(idc) = d.id_cliente {
        if let Ok(Some(c)) = ClienteRepository::obtener_por_id(conn, idc) {
            d.nombre_cliente = c.nombre_completo;
            d.nacionalidad = c.nacionalidad;
        }
    }
}

/// Recalcula subtotal/impuestos/total/saldo desde las tarifas y costos
/// (el backend es la fuente de verdad). El abono se conserva.
fn calcular_totales(d: &mut RentaDatos, cfg: &Arc<AppConfig>) {
    let dias = d.dias_calculados.max(0);
    let horas = d.horas_extras.max(0);
    let vdia = dec(&d.valor_dia, "");
    let vhe = dec(&d.valor_hora_extra, "");
    let vde = dec(&d.valor_dia_extra, "");
    let extras = sum_dec(&[
        &vde.to_string(),
        &d.costo_lavado,
        &d.costo_silla,
        &d.costo_retorno,
        &d.costo_domicilio,
        &d.costo_cables,
        &d.costo_inversor,
        &d.valor_gasolina,
    ]);
    let desc = dec(&d.descuento, "");
    let subtotal = (vdia * Decimal::from(dias) + vhe * Decimal::from(horas) + extras - desc).max(Decimal::ZERO);
    // IVA solo si el formulario marcó «cobrar IVA» (checkbox por renta)
    let imp = if d.cobra_iva { impuesto(cfg) } else { Decimal::ZERO };
    let impuestos = (subtotal * imp).round_dp(2);
    let total = subtotal + impuestos;
    d.subtotal = subtotal.round_dp(2).to_string();
    d.impuestos = impuestos.to_string();
    d.total = total.round_dp(2).to_string();
    d.saldo_pendiente = total.round_dp(2).to_string();
}

/// Porcentaje de impuesto de config.ini (business.impuesto_porcentaje, default 0)
fn impuesto(cfg: &Arc<AppConfig>) -> Decimal {
    Decimal::from_str(&cfg.impuesto_porcentaje.to_string())
        .unwrap_or(Decimal::ZERO)
        / Decimal::from(100)
}

/// Umbral de tolerancia: si el excedente sobre el último día completo supera
/// estas horas, se cobra un día completo en vez de horas extras.
const HORAS_TOLERANCIA_DIA_COMPLETO: i64 = 3;

/// Calcula días y horas extras entre la recogida y la devolución real
/// (regla de negocio: cada 24 h = 1 día; el excedente de hasta 3 h se cobra
/// como horas extras redondeadas hacia arriba; más de 3 h = día completo).
/// Devuelve `None` si falta fecha u hora en cualquiera de los dos extremos.
fn calcular_dias_horas(
    fecha_recogida: &str,
    hora_recogida: Option<&str>,
    fecha_dev: Option<&str>,
    hora_dev: Option<&str>,
) -> Option<(i64, i64)> {
    let recogida = NaiveDate::parse_from_str(fecha_recogida.trim(), "%Y-%m-%d").ok()?;
    let rec_h = parse_hora_ref(hora_recogida)?;
    let fecha_dev = parse_fecha_ref(fecha_dev)?;
    let dev_h = parse_hora_ref(hora_dev)?;
    let inicio = recogida.and_time(rec_h);
    let fin = fecha_dev.and_time(dev_h);
    let minutos = (fin - inicio).num_minutes().max(0);
    let dia_min = 24 * 60;
    let dias = minutos / dia_min;
    let rem = minutos % dia_min;
    if rem > HORAS_TOLERANCIA_DIA_COMPLETO * 60 {
        // Excedente > 3 h → se cobra el día completo
        Some((dias + 1, 0))
    } else {
        // Fracciones de hora redondeadas hacia arriba (1 min = 1 hora)
        Some((dias, (rem + 59) / 60))
    }
}

/// Parsea `Option<&str>` (HH:MM o HH:MM:SS) → `NaiveTime`; `None`/vacío → None
fn parse_hora_ref(v: Option<&str>) -> Option<NaiveTime> {
    let h = v.map(str::trim).filter(|s| !s.is_empty())?;
    let h = if h.len() == 5 { format!("{h}:00") } else { h.to_string() };
    NaiveTime::parse_from_str(&h, "%H:%M:%S").ok()
}

/// Parsea `Option<&str>` (AAAA-MM-DD) → `NaiveDate`; `None`/vacío → None
fn parse_fecha_ref(v: Option<&str>) -> Option<NaiveDate> {
    let s = v.map(str::trim).filter(|s| !s.is_empty())?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// Parsea un decimal desde un string, con fallback si está vacío
fn dec(v: &str, fallback: &str) -> Decimal {
    let s = if v.trim().is_empty() { fallback } else { v };
    Decimal::from_str(s).unwrap_or(Decimal::ZERO).max(Decimal::ZERO)
}

fn dec_str(v: &str) -> Decimal {
    Decimal::from_str(v).unwrap_or(Decimal::ZERO).max(Decimal::ZERO)
}

fn sum_dec(vals: &[&str]) -> Decimal {
    vals.iter().fold(Decimal::ZERO, |acc, v| acc + dec(v, "0.00"))
}

/// Valida los datos de la renta
fn validar(d: &RentaDatos, cfg: &Arc<AppConfig>) -> Result<(), AppError> {
    if d.nombre_cliente.is_empty() || d.nombre_cliente.len() > 200 {
        return Err(AppError::Validation(
            "El nombre del cliente es obligatorio (máx. 200 caracteres).".into(),
        ));
    }
    validate_no_xss(&d.nombre_cliente, 200).map_err(|_| {
        AppError::Validation("El nombre del cliente contiene caracteres no permitidos.".into())
    })?;

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

    if d.dias_calculados < 0 {
        return Err(AppError::Validation("Los días calculados no pueden ser negativos.".into()));
    }
    if d.horas_extras < 0 {
        return Err(AppError::Validation("Las horas extras no pueden ser negativas.".into()));
    }

    // Montos: todos deben ser números válidos ≥ 0 (no silenciar a 0)
    for (campo, m) in [
        ("valor del día", &d.valor_dia),
        ("valor de la hora extra", &d.valor_hora_extra),
        ("valor del día extra", &d.valor_dia_extra),
        ("costo de lavado", &d.costo_lavado),
        ("costo de silla", &d.costo_silla),
        ("costo de retorno", &d.costo_retorno),
        ("costo de domicilio", &d.costo_domicilio),
        ("costo de cables", &d.costo_cables),
        ("costo de inversor", &d.costo_inversor),
        ("valor de gasolina", &d.valor_gasolina),
        ("descuento", &d.descuento),
    ] {
        let v = if m.trim().is_empty() {
            Decimal::ZERO
        } else {
            match Decimal::from_str(m.trim()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(AppError::Validation(format!(
                        "El {campo} no es un número válido."
                    )));
                }
            }
        };
        if v < Decimal::ZERO {
            return Err(AppError::Validation(format!(
                "El {campo} no es un número válido."
            )));
        }
    }
    if let Some(p) = &d.placa {
        validate_no_xss(p, 20).map_err(|_| {
            AppError::Validation("La placa contiene caracteres no permitidos.".into())
        })?;
    }

    // Campos de texto libre (consistente con reservas/autos/clientes)
    for (campo, val) in [
        ("la licencia de conducción", d.no_licencia.as_deref()),
        ("la nacionalidad", d.nacionalidad.as_deref()),
        ("la ubicación de recogida", d.ubicacion_recogida.as_deref()),
        ("la ubicación de retorno", d.ubicacion_retorno.as_deref()),
        ("las observaciones", d.observaciones.as_deref()),
        ("el tanque de salida", d.tanque_salida.as_deref()),
    ] {
        if let Some(v) = val {
            if !v.is_empty() && validate_no_xss(v, 2000).is_err() {
                return Err(AppError::Validation(format!(
                    "{campo} contiene caracteres no permitidos."
                )));
            }
        }
    }
    // Referencias cruzadas
    if d.id_reserva.is_some() && d.id_reserva == Some(0) {
        return Err(AppError::Validation("La reserva de origen no es válida.".into()));
    }
    let _ = cfg; // config se usa en calcular_totales
    Ok(())
}

fn validar_cierre(d: &RentaCierreDatos) -> Result<(), AppError> {
    if let Some(f) = d.fecha_devolucion_real.as_deref() {
        if !f.is_empty() && NaiveDate::parse_from_str(f.trim(), "%Y-%m-%d").is_err() {
            return Err(AppError::Validation(
                "La fecha de devolución no es válida (formato AAAA-MM-DD).".into(),
            ));
        }
    }
    if let Some(h) = d.hora_devolucion_real.as_deref() {
        if !h.is_empty() && !es_hora_valida(h) {
            return Err(AppError::Validation(
                "La hora de devolución no es válida (formato HH:MM).".into(),
            ));
        }
    }
    for (campo, v) in [("el kilometraje final", &d.km_final)] {
        if let Some(k) = v {
            if !k.is_empty() && k.parse::<f64>().is_err() {
                return Err(AppError::Validation(format!(
                    "{campo} no es un número válido."
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

// ──────────────────────────────────────────────────────────────────────────
// Helpers para las transacciones de `registrar_pago` y `cerrar` (duplicados
// de `repositories/renta.rs` porque son privados allí; se mantienen aquí para
// no alterar el API público del repositorio).
// ──────────────────────────────────────────────────────────────────────────

/// Recorta un `Option<String>` y descarta vacíos (devuelve `None`).
fn opt_str(v: &Option<String>) -> Option<String> {
    v.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Parsea `Option<String>` → `Option<NaiveDate>` (formato AAAA-MM-DD).
fn parse_fecha_opt(v: &Option<String>) -> Result<Option<NaiveDate>, AppError> {
    match opt_str(v) {
        None => Ok(None),
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| AppError::Validation("Fecha inválida (formato AAAA-MM-DD).".into())),
    }
}

/// Parsea `Option<String>` → `Option<NaiveTime>` (formato HH:MM o HH:MM:SS).
fn parse_hora(v: &Option<String>) -> Result<Option<NaiveTime>, AppError> {
    match opt_str(v) {
        None => Ok(None),
        Some(h) => {
            let h = if h.len() == 5 { format!("{h}:00") } else { h };
            NaiveTime::parse_from_str(&h, "%H:%M:%S")
                .map(Some)
                .map_err(|_| AppError::Validation("Hora inválida (formato HH:MM).".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dias_horas_exacto_sin_excedente() {
        // 2026-01-01 10:00 → 2026-01-03 10:00 = 48 h = 2 días, 0 horas
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), Some("10:00"));
        assert_eq!(r, Some((2, 0)));
    }

    #[test]
    fn excedente_menor_igual_3_horas_se_cobra_por_horas() {
        // 2 h de excedente → 2 horas extras
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), Some("12:00"));
        assert_eq!(r, Some((2, 2)));
        // exactamente 3 h → horas extras (el día completo es solo si SUPERA 3 h)
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), Some("13:00"));
        assert_eq!(r, Some((2, 3)));
        // 3 h y 1 minuto → día completo
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), Some("13:01"));
        assert_eq!(r, Some((3, 0)));
    }

    #[test]
    fn excedente_mayor_3_horas_cobra_dia_completo() {
        // 6 h de excedente → 3 días, 0 horas extras
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), Some("16:00"));
        assert_eq!(r, Some((3, 0)));
        // 25 h de excedente (73 h totales) → 3 días completos + 1 h; la 1 h
        // restante ≤ 3 h se cobra como 1 hora extra
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-04"), Some("11:00"));
        assert_eq!(r, Some((3, 1)));
    }

    #[test]
    fn fracciones_de_hora_se_redondean_hacia_arriba() {
        // 30 min de excedente → 1 hora extra
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), Some("10:30"));
        assert_eq!(r, Some((2, 1)));
        // 2 h 20 min → 3 horas extra
        let r = calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), Some("12:20"));
        assert_eq!(r, Some((2, 3)));
    }

    #[test]
    fn faltan_horas_o_fechas_devuelve_none() {
        assert_eq!(calcular_dias_horas("2026-01-01", None, Some("2026-01-03"), Some("10:00")), None);
        assert_eq!(calcular_dias_horas("2026-01-01", Some("10:00"), Some("2026-01-03"), None), None);
        assert_eq!(calcular_dias_horas("2026-01-01", Some("10:00"), None, Some("10:00")), None);
    }
}
