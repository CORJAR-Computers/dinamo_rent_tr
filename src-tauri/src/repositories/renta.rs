//! repositories/renta.rs — Repositorio de rentas (con pagos e inspecciones)
//!
//! Queries explícitas en dialecto Firebird con rsfbclient.
//! - DECIMAL → CAST a VARCHAR (parseo exacto en el servicio/frontend)
//! - DATE/TIME/TIMESTAMP → CAST a VARCHAR

use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

use crate::core::error::AppError;
use crate::core::PooledConnection;
// Helpers centralizados (Bloque 4 / TAREA 4.2): antes estaban duplicados
// localmente en este archivo. La migración los importa de `core::repository`
// para DRY. Se conserva un wrapper `map_fb_error` (1 línea) que delega en
// `map_fb_error_fk` con el mensaje FK específico de rentas (preserva UX).
use crate::core::repository::{opt_str, params, parse_fecha, parse_fecha_opt, parse_hora_opt};

use serde::Serialize;
// ts-rs (Bloque 4 / TAREA 4.3): genera tipos TypeScript en
// `src/lib/types/generated/` cuando se ejecuta `cargo test`. El frontend
// puede importarlos en lugar de mantenerlos a mano en `src/lib/api.ts`.
// El atributo `#[ts(export, export_to = "...")]` controla la generación.
use ts_rs::TS;

/// Pago registrado contra una renta
///
/// Contrato TypeScript generado por ts-rs en `src/lib/types/generated/Pago.ts`
/// (Bloque 4 / TAREA 4.3). El frontend puede importarlo con:
///   `import type { Pago } from '$lib/types/generated/Pago';`
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../src/lib/types/generated/")]
pub struct Pago {
    pub id: i64,
    pub id_renta: i64,
    pub fecha: String,
    pub monto: String,
    pub metodo_pago: String,
    pub concepto: String,
    pub observaciones: Option<String>,
    pub usuario: Option<String>,
}

/// Inspección de salida/entrada de una renta
///
/// Contrato TypeScript generado por ts-rs en `src/lib/types/generated/Inspeccion.ts`
/// (Bloque 4 / TAREA 4.3). Necesario derivar `TS` aquí porque `Renta` la
/// referencia como `Vec<Inspeccion>` (ts-rs requiere que todo tipo anidado
/// tambien implemente `TS`).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../src/lib/types/generated/")]
pub struct Inspeccion {
    pub id: i64,
    pub id_renta: i64,
    pub tipo: String,
    pub fecha: String,
    pub kilometraje: String,
    pub nivel_gasolina: String,
    pub limpieza: Option<String>,
    pub tiene_repuesto: bool,
    pub tiene_gato_cruceta: bool,
    pub tiene_kit_carretera: bool,
    pub tiene_documentos: bool,
    pub danos_carroceria: Option<String>,
    pub observaciones: Option<String>,
}

/// Renta completa (serializable al frontend, camelCase)
///
/// Contrato TypeScript generado por ts-rs en `src/lib/types/generated/Renta.ts`
/// (Bloque 4 / TAREA 4.3). Es el contrato FE<->BE mas importante: cualquier
/// cambio en los campos de `Renta` se refleja automaticamente en el `.ts`
/// generado al correr `cargo test`, evitando drift entre Rust y Svelte.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../src/lib/types/generated/")]
pub struct Renta {
    pub id: i64,
    /// Número de contrato: secuencia DENTRO del año (2026-001, 2026-002, ...)
    pub no_contrato: i64,
    /// Año del contrato (año de creación de la renta)
    pub anio_contrato: i64,
    pub placa: Option<String>,
    pub id_cliente: Option<i64>,
    pub nombre_cliente: String,
    pub no_licencia: Option<String>,
    pub nacionalidad: Option<String>,
    pub fecha_recogida: String,
    pub hora_recogida: Option<String>,
    pub ubicacion_recogida: Option<String>,
    pub fecha_retorno: String,
    pub hora_retorno: Option<String>,
    pub ubicacion_retorno: Option<String>,
    pub dias_calculados: i64,
    pub horas_extras: i64,
    pub valor_dia: String,
    pub valor_hora_extra: String,
    pub valor_dia_extra: String,
    pub costo_lavado: String,
    pub costo_silla: String,
    pub costo_retorno: String,
    pub costo_domicilio: String,
    pub costo_cables: String,
    pub costo_inversor: String,
    /// Valor de gasolina a cobrar cuando el cliente entrega/recibe sin tanque lleno
    pub valor_gasolina: String,
    pub descuento: String,
    pub subtotal: String,
    pub impuestos: String,
    /// ¿Cobra IVA esta renta? (checkbox del formulario; false = sin IVA)
    pub cobra_iva: bool,
    /// ¿Tiene comisión esta renta? (checkbox del formulario; false = sin comisión)
    pub tiene_comision: bool,
    /// Valor de la comisión a descontar (información financiera: neto = total − comisión)
    pub comision: String,
    /// Valor neto = total − comisión (persistido para reportes financieros)
    pub valor_neto: String,
    pub total: String,
    pub abono: String,
    pub saldo_pendiente: String,
    pub estado: String,
    pub observaciones: Option<String>,
    pub fecha_devolucion_real: Option<String>,
    pub hora_devolucion_real: Option<String>,
    pub km_final: Option<String>,
    pub tanque_final: Option<String>,
    pub km_salida: String,
    pub tanque_salida: Option<String>,
    pub id_reserva: Option<i64>,
    pub created_at: Option<String>,
    /// Vehículo (JOIN con autos): marca + modelo
    pub vehiculo: String,
    /// Pagos registrados contra la renta
    pub pagos: Vec<Pago>,
    /// Inspecciones de la renta
    pub inspecciones: Vec<Inspeccion>,
}

/// Datos de entrada para crear/actualizar (validados por el servicio)
///
/// Contrato TypeScript generado por ts-rs en `src/lib/types/generated/RentaDatos.ts`
/// (Bloque 4 / TAREA 4.3). El frontend usa este tipo para construir el body
/// del command `crear_renta` / `actualizar_renta` (ver `src/lib/api.ts`).
#[derive(Debug, Clone, Default, serde::Deserialize, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export, export_to = "../src/lib/types/generated/")]
pub struct RentaDatos {
    pub placa: Option<String>,
    pub id_cliente: Option<i64>,
    pub nombre_cliente: String,
    pub no_licencia: Option<String>,
    pub nacionalidad: Option<String>,
    pub fecha_recogida: String,
    pub hora_recogida: Option<String>,
    pub ubicacion_recogida: Option<String>,
    pub fecha_retorno: String,
    pub hora_retorno: Option<String>,
    pub ubicacion_retorno: Option<String>,
    pub dias_calculados: i64,
    pub horas_extras: i64,
    pub valor_dia: String,
    pub valor_hora_extra: String,
    pub valor_dia_extra: String,
    pub costo_lavado: String,
    pub costo_silla: String,
    pub costo_retorno: String,
    pub costo_domicilio: String,
    pub costo_cables: String,
    pub costo_inversor: String,
    /// Valor de gasolina a cobrar (cliente entrega/recibe sin tanquear)
    pub valor_gasolina: String,
    pub descuento: String,
    /// Campos calculados por el servicio (subtotal/impuestos/total/saldo)
    pub subtotal: String,
    pub impuestos: String,
    /// ¿Cobra IVA? (checkbox del formulario; el servicio lo aplica al calcular)
    pub cobra_iva: bool,
    /// ¿Tiene comisión? (checkbox del formulario; false = sin comisión)
    pub tiene_comision: bool,
    /// Valor de la comisión a restar del total (neto = total − comisión)
    pub comision: String,
    /// Valor neto (calculado por el servicio: total − comisión)
    pub valor_neto: String,
    pub total: String,
    pub abono: String,
    pub saldo_pendiente: String,
    pub observaciones: Option<String>,
    pub km_salida: String,
    pub tanque_salida: Option<String>,
    pub id_reserva: Option<i64>,
}

/// Datos del cierre de una renta (devolución real y totales)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RentaCierreDatos {
    pub fecha_devolucion_real: Option<String>,
    pub hora_devolucion_real: Option<String>,
    pub km_final: Option<String>,
    pub tanque_final: Option<String>,
    /// Ajustes del cierre (si se recalculan)
    pub dias_calculados: Option<i64>,
    pub horas_extras: Option<i64>,
    pub valor_dia: Option<String>,
    pub valor_hora_extra: Option<String>,
    pub descuento: Option<String>,
    pub observaciones: Option<String>,
}

/// Datos para editar una renta cerrada (corrección de errores de digitación)
/// Solo permite campos financieros que afectan los totales. Los campos de
/// identificación (placa, cliente) y abono no son editables.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RentaCierreEditDatos {
    /// Valor diario de la renta (corrección de digitación)
    pub valor_dia: Option<String>,
    /// Valor hora extra (corrección de digitación)
    pub valor_hora_extra: Option<String>,
    /// Días calculados (corrección de digitación)
    pub dias_calculados: Option<i64>,
    /// Horas extras (corrección de digitación)
    pub horas_extras: Option<i64>,
    /// Descuento aplicado (corrección de digitación)
    pub descuento: Option<String>,
    /// Observaciones sobre la corrección (obligatorio para auditoría)
    pub observaciones: Option<String>,
}

/// Datos para extender una renta activa (agregar horas o días extras)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExtensionDatos {
    /// Tipo de extensión: "horas" o "dias"
    pub tipo: String,
    /// Cantidad de horas o días a agregar
    pub cantidad: i64,
    /// Valor unitario (hora o día extra)
    pub valor: String,
    /// Observaciones sobre la extensión
    pub observaciones: Option<String>,
}

/// Datos de un pago
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PagoDatos {
    pub monto: String,
    pub metodo_pago: String,
    pub concepto: String,
    pub observaciones: Option<String>,
}

/// Datos de una inspección
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct InspeccionDatos {
    pub tipo: String,
    pub kilometraje: String,
    pub nivel_gasolina: String,
    pub limpieza: Option<String>,
    pub tiene_repuesto: bool,
    pub tiene_gato_cruceta: bool,
    pub tiene_kit_carretera: bool,
    pub tiene_documentos: bool,
    pub danos_carroceria: Option<String>,
    pub observaciones: Option<String>,
}

/// rsfbclient solo implementa FromRow para tuplas de hasta 26 elementos,
/// así que el SELECT se divide en dos consultas unidas por id:
/// - SELECT_COLS_A: primeros 26 campos (datos generales y de tarifas)
/// - SELECT_COLS_B: resto (financieros, cierre, salida y vehículo)
pub const SELECT_COLS_A: &str = "\
    r.id, r.placa, r.id_cliente, r.nombre_cliente, r.no_licencia, r.nacionalidad, \
    CAST(r.fecha_recogida AS VARCHAR(10)), CAST(r.hora_recogida AS VARCHAR(13)), r.ubicacion_recogida, \
    CAST(r.fecha_retorno AS VARCHAR(10)), CAST(r.hora_retorno AS VARCHAR(13)), r.ubicacion_retorno, \
    r.dias_calculados, r.horas_extras, \
    CAST(r.valor_dia AS VARCHAR(12)), CAST(r.valor_hora_extra AS VARCHAR(12)), \
    CAST(r.valor_dia_extra AS VARCHAR(12)), CAST(r.costo_lavado AS VARCHAR(12)), \
    CAST(r.costo_silla AS VARCHAR(12)), CAST(r.costo_retorno AS VARCHAR(12)), \
    CAST(r.costo_domicilio AS VARCHAR(12)), CAST(r.costo_cables AS VARCHAR(12)), \
    CAST(r.costo_inversor AS VARCHAR(12)), CAST(r.descuento AS VARCHAR(12)), \
    CAST(r.subtotal AS VARCHAR(12)), CAST(r.impuestos AS VARCHAR(12))";

pub const SELECT_COLS_B: &str = "\
    r.id, \
    CAST(r.total AS VARCHAR(12)), CAST(r.abono AS VARCHAR(12)), CAST(r.saldo_pendiente AS VARCHAR(12)), \
    r.estado, CAST(r.observaciones AS VARCHAR(2000)), \
    CAST(r.fecha_devolucion_real AS VARCHAR(10)), CAST(r.hora_devolucion_real AS VARCHAR(13)), \
    r.km_final, r.tanque_final, CAST(r.km_salida AS VARCHAR(20)), r.tanque_salida, \
    r.id_reserva, CAST(r.created_at AS VARCHAR(30)), \
    COALESCE(a.marca || ' ' || a.modelo, ''), \
    r.no_contrato, r.anio_contrato, \
    r.cobra_iva = 1, \
    CAST(r.valor_gasolina AS VARCHAR(12)), \
    r.tiene_comision = 1, CAST(r.comision AS VARCHAR(12)), CAST(r.valor_neto AS VARCHAR(12))";

/// Fila A (26 columnas) — mantener alineada con `SELECT_COLS_A`
#[allow(clippy::type_complexity)]
pub type RentaRowA = (
    i64,
    Option<String>,
    Option<i64>,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    i64,
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

/// Fila B (22 columnas) — mantener alineada con `SELECT_COLS_B`
#[allow(clippy::type_complexity)]
pub type RentaRowB = (
    i64,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<i64>,
    Option<String>,
    String,
    i64,
    i64,
    bool,
    String,
    bool,
    String,
    String,
);

fn from_rows(a: RentaRowA, b: RentaRowB) -> Renta {
    debug_assert_eq!(a.0, b.0, "renta id mismatch en consulta dividida");
    Renta {
        id: a.0,
        placa: a.1,
        id_cliente: a.2,
        nombre_cliente: a.3,
        no_licencia: a.4,
        nacionalidad: a.5,
        fecha_recogida: a.6,
        hora_recogida: a.7.map(|h| hora_corta(&h)),
        ubicacion_recogida: a.8,
        fecha_retorno: a.9,
        hora_retorno: a.10.map(|h| hora_corta(&h)),
        ubicacion_retorno: a.11,
        dias_calculados: a.12,
        horas_extras: a.13,
        valor_dia: a.14,
        valor_hora_extra: a.15,
        valor_dia_extra: a.16,
        costo_lavado: a.17,
        costo_silla: a.18,
        costo_retorno: a.19,
        costo_domicilio: a.20,
        costo_cables: a.21,
        costo_inversor: a.22,
        descuento: a.23,
        subtotal: a.24,
        impuestos: a.25,
        cobra_iva: b.17,
        tiene_comision: b.19,
        comision: b.20,
        valor_neto: b.21,
        no_contrato: b.15,
        anio_contrato: b.16,
        valor_gasolina: b.18,
        total: b.1,
        abono: b.2,
        saldo_pendiente: b.3,
        estado: b.4,
        observaciones: b.5,
        fecha_devolucion_real: b.6,
        hora_devolucion_real: b.7.map(|h| hora_corta(&h)),
        km_final: b.8,
        tanque_final: b.9,
        km_salida: km_limpio(&b.10),
        tanque_salida: b.11,
        id_reserva: b.12,
        created_at: b.13,
        vehiculo: b.14,
        pagos: Vec::new(),
        inspecciones: Vec::new(),
    }
}

/// Mapea errores de Firebird a AppError (FKs de placa/cliente/reserva).
///
/// Wrapper que delega en `crate::core::repository::map_fb_error_fk` con el
/// mensaje de negocio específico de rentas. Antes esto estaba duplicado en
/// 5+ repositorios (Bloque 4 / TAREA 4.2).
fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    crate::core::repository::map_fb_error_fk(
        e,
        "El cliente, el vehículo o la reserva seleccionada no existe (o está referenciado por otros registros).",
    )
}

pub struct RentaRepository;

impl RentaRepository {
    /// Consulta base con JOIN a autos (placa/marca/modelo).
    /// Ejecuta las dos consultas (A: 26 columnas, B: 15) con los mismos filtros
    /// y une los resultados por id.
    fn consultar(conn: &mut PooledConnection, where_sql: &str, params: &ParamsType) -> Result<Vec<Renta>, AppError> {
        // Soft delete: siempre filtra rentas NO borradas. Si where_sql viene
        // vacío (obtener_todos), se inyecta WHERE r.deleted_at IS NULL. Si
        // viene con WHERE ..., se envuelve en paréntesis para no romper la
        // precedencia de AND/OR (ej. buscar() usa OR entre 3 condiciones).
        let where_deleted = if where_sql.trim().is_empty() {
            "WHERE r.deleted_at IS NULL".to_string()
        } else {
            let s = where_sql.trim();
            let after = s.strip_prefix("WHERE").unwrap_or(s);
            format!("WHERE r.deleted_at IS NULL AND ({after})")
        };
        let base = format!(
            "FROM rentas r LEFT JOIN autos a ON a.placa = r.placa {where_deleted} "
        );
        let sql_a = format!(
            "SELECT {SELECT_COLS_A} {base} ORDER BY r.fecha_recogida DESC, r.id DESC"
        );
        let sql_b = format!(
            "SELECT {SELECT_COLS_B} {base} ORDER BY r.fecha_recogida DESC, r.id DESC"
        );
        // ParamsType no es Clone: se reconstruye clonando el vector de SqlType
        let to_params = |v: &[rsfbclient::SqlType]| ParamsType::Positional(v.to_vec());
        let (rows_a, rows_b) = match params {
            ParamsType::Positional(v) => {
                let a: Vec<RentaRowA> = conn.query(&sql_a, to_params(v))?;
                let b: Vec<RentaRowB> = conn.query(&sql_b, to_params(v))?;
                (a, b)
            }
            // No se usa en este módulo (siempre Positional); defensivo
            ParamsType::Named(_) => {
                return Err(AppError::Generic(
                    "Parámetros nombrados no soportados en consulta de rentas.".into(),
                ));
            }
        };
        Ok(rows_a
            .into_iter()
            .zip(rows_b)
            .map(|(a, b)| from_rows(a, b))
            .collect())
    }

    /// Lista todas las rentas
    pub fn obtener_todos(conn: &mut PooledConnection) -> Result<Vec<Renta>, AppError> {
        let empty = ParamsType::Positional(vec![]);
        Self::consultar(conn, "", &empty)
    }

    /// Busca rentas por placa, cliente o estado (insensible a mayúsculas)
    pub fn buscar(conn: &mut PooledConnection, term: &str) -> Result<Vec<Renta>, AppError> {
        let like = format!("%{}%", term.trim());
        Self::consultar(
            conn,
            "WHERE UPPER(r.nombre_cliente) LIKE UPPER(?) OR UPPER(COALESCE(r.placa, '')) LIKE UPPER(?) OR UPPER(r.estado) LIKE UPPER(?)",
            &ParamsType::Positional(vec![like.clone().into_param(), like.clone().into_param(), like.into_param()]),
        )
    }

    /// Filtra por estado
    pub fn obtener_por_estado(conn: &mut PooledConnection, estado: &str) -> Result<Vec<Renta>, AppError> {
        Self::consultar(
            conn,
            "WHERE r.estado = ?",
            &ParamsType::Positional(vec![estado.to_string().into_param()]),
        )
    }

    /// Filtra por placa
    pub fn obtener_por_placa(conn: &mut PooledConnection, placa: &str) -> Result<Vec<Renta>, AppError> {
        Self::consultar(
            conn,
            "WHERE r.placa = ?",
            &ParamsType::Positional(vec![placa.to_string().into_param()]),
        )
    }

    /// Obtiene una renta por id (con pagos e inspecciones)
    pub fn obtener_por_id(conn: &mut PooledConnection, id: i64) -> Result<Option<Renta>, AppError> {
        let mut rentas = Self::consultar(
            conn,
            "WHERE r.id = ?",
            &ParamsType::Positional(vec![id.into_param()]),
        )?;
        if let Some(renta) = rentas.first_mut() {
            renta.pagos = Self::pagos_de(conn, id)?;
            renta.inspecciones = Self::inspecciones_de(conn, id)?;
        }
        Ok(rentas.into_iter().next())
    }

    /// Pagos de una renta (más recientes primero)
    pub fn pagos_de(conn: &mut PooledConnection, id_renta: i64) -> Result<Vec<Pago>, AppError> {
        let rows: Vec<(i64, i64, String, String, String, String, Option<String>, String)> =
            conn.query(
                "SELECT id, id_renta, CAST(fecha AS VARCHAR(30)), CAST(monto AS VARCHAR(12)), \
                        metodo_pago, concepto, CAST(observaciones AS VARCHAR(2000)), COALESCE(usuario, '') \
                 FROM pagos WHERE id_renta = ? AND deleted_at IS NULL ORDER BY fecha, id",
                ParamsType::Positional(vec![id_renta.into_param()]),
            )?;
        Ok(rows
            .into_iter()
            .map(|r| Pago {
                id: r.0,
                id_renta: r.1,
                fecha: r.2,
                monto: r.3,
                metodo_pago: r.4,
                concepto: r.5,
                observaciones: r.6,
                usuario: (!r.7.trim().is_empty()).then_some(r.7),
            })
            .collect())
    }

    /// Inspecciones de una renta (salida primero)
    pub fn inspecciones_de(conn: &mut PooledConnection, id_renta: i64) -> Result<Vec<Inspeccion>, AppError> {
        let rows: Vec<(i64, i64, String, String, String, String, Option<String>, bool, bool, bool, bool, Option<String>, Option<String>)> =
            conn.query(
                "SELECT id, id_renta, tipo, CAST(fecha AS VARCHAR(30)), CAST(kilometraje AS VARCHAR(20)), \
                        nivel_gasolina, limpieza, tiene_repuesto = 1, tiene_gato_cruceta = 1, \
                        tiene_kit_carretera = 1, tiene_documentos = 1, \
                        CAST(danos_carroceria AS VARCHAR(2000)), CAST(observaciones AS VARCHAR(2000)) \
                 FROM inspecciones WHERE id_renta = ? ORDER BY tipo, id",
                ParamsType::Positional(vec![id_renta.into_param()]),
            )?;
        Ok(rows
            .into_iter()
            .map(|r| Inspeccion {
                id: r.0,
                id_renta: r.1,
                tipo: r.2,
                fecha: r.3,
                kilometraje: km_limpio(&r.4),
                nivel_gasolina: r.5,
                limpieza: r.6,
                tiene_repuesto: r.7,
                tiene_gato_cruceta: r.8,
                tiene_kit_carretera: r.9,
                tiene_documentos: r.10,
                danos_carroceria: r.11,
                observaciones: r.12,
            })
            .collect())
    }

    /// Crea una renta y devuelve el id nuevo (RETURNING evita races con MAX(id)).
    /// Genérica sobre la conexión (`PooledConnection` o la `Transaction` de
    /// `with_transaction`) para poder insertar dentro de una transacción.
    pub fn insertar<C>(conn: &mut C, d: &RentaDatos) -> Result<i64, AppError>
    where
        C: rsfbclient::Execute,
    {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO rentas (\
                    placa, id_cliente, nombre_cliente, no_licencia, nacionalidad, \
                    fecha_recogida, hora_recogida, ubicacion_recogida, fecha_retorno, hora_retorno, \
                    ubicacion_retorno, dias_calculados, horas_extras, \
                    valor_dia, valor_hora_extra, valor_dia_extra, \
                    costo_lavado, costo_silla, costo_retorno, costo_domicilio, costo_cables, costo_inversor, \
                    valor_gasolina, \
                    descuento, subtotal, impuestos, total, abono, saldo_pendiente, \
                    cobra_iva, tiene_comision, comision, valor_neto, estado, observaciones, km_salida, tanque_salida, id_reserva, no_contrato, anio_contrato \
                 ) VALUES (\
                    ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, \
                    ?, ?, \
                    CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), \
                    CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), \
                    CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), \
                    CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), \
                    CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), \
                    CAST(? AS DECIMAL(12,2)), \
                    ?, ?, CAST(? AS DECIMAL(12,2)), CAST(? AS DECIMAL(12,2)), 'Activo', ?, CAST(? AS DOUBLE PRECISION), ?, ?, \
                    (SELECT COALESCE(MAX(no_contrato), 0) + 1 FROM rentas WHERE anio_contrato = EXTRACT(YEAR FROM CURRENT_TIMESTAMP)), \
                    EXTRACT(YEAR FROM CURRENT_TIMESTAMP) \
                 ) RETURNING id",
                params![
                    opt_str(&d.placa),
                    d.id_cliente,
                    d.nombre_cliente.to_string(),
                    opt_str(&d.no_licencia),
                    opt_str(&d.nacionalidad),
                    parse_fecha(&d.fecha_recogida)?,
                    parse_hora_opt(&d.hora_recogida)?,
                    opt_str(&d.ubicacion_recogida),
                    parse_fecha(&d.fecha_retorno)?,
                    parse_hora_opt(&d.hora_retorno)?,
                    opt_str(&d.ubicacion_retorno),
                    d.dias_calculados,
                    d.horas_extras,
                    d.valor_dia.to_string(),
                    d.valor_hora_extra.to_string(),
                    d.valor_dia_extra.to_string(),
                    d.costo_lavado.to_string(),
                    d.costo_silla.to_string(),
                    d.costo_retorno.to_string(),
                    d.costo_domicilio.to_string(),
                    d.costo_cables.to_string(),
                    d.costo_inversor.to_string(),
                    d.valor_gasolina.to_string(),
                    d.descuento.to_string(),
                    d.subtotal.to_string(),
                    d.impuestos.to_string(),
                    d.total.to_string(),
                    d.abono.to_string(),
                    d.saldo_pendiente.to_string(),
                    bool_to_i(d.cobra_iva),
                    bool_to_i(d.tiene_comision),
                    d.comision.to_string(),
                    d.valor_neto.to_string(),
                    opt_str(&d.observaciones),
                    d.km_salida.parse::<f64>().unwrap_or(0.0),
                    opt_str(&d.tanque_salida),
                    d.id_reserva,
                ],
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Actualiza una renta (no toca totales ni estado: los recalcula el servicio)
    pub fn actualizar(conn: &mut PooledConnection, id: i64, d: &RentaDatos) -> Result<(), AppError> {
        conn.execute(
            "UPDATE rentas SET \
                placa = ?, id_cliente = ?, nombre_cliente = ?, no_licencia = ?, nacionalidad = ?, \
                fecha_recogida = ?, hora_recogida = ?, ubicacion_recogida = ?, \
                fecha_retorno = ?, hora_retorno = ?, ubicacion_retorno = ?, \
                dias_calculados = ?, horas_extras = ?, \
                valor_dia = CAST(? AS DECIMAL(12,2)), valor_hora_extra = CAST(? AS DECIMAL(12,2)), \
                valor_dia_extra = CAST(? AS DECIMAL(12,2)), \
                costo_lavado = CAST(? AS DECIMAL(12,2)), costo_silla = CAST(? AS DECIMAL(12,2)), \
                costo_retorno = CAST(? AS DECIMAL(12,2)), costo_domicilio = CAST(? AS DECIMAL(12,2)), \
                costo_cables = CAST(? AS DECIMAL(12,2)), costo_inversor = CAST(? AS DECIMAL(12,2)), \
                valor_gasolina = CAST(? AS DECIMAL(12,2)), \
                descuento = CAST(? AS DECIMAL(12,2)), \
                subtotal = CAST(? AS DECIMAL(12,2)), impuestos = CAST(? AS DECIMAL(12,2)), \
                total = CAST(? AS DECIMAL(12,2)), saldo_pendiente = CAST(? AS DECIMAL(12,2)), \
                cobra_iva = ?, tiene_comision = ?, comision = CAST(? AS DECIMAL(12,2)), valor_neto = CAST(? AS DECIMAL(12,2)), \
                observaciones = ?, km_salida = CAST(? AS DOUBLE PRECISION), tanque_salida = ?, \
                id_reserva = ? \
             WHERE id = ?",
            params![
                opt_str(&d.placa),
                d.id_cliente,
                d.nombre_cliente.to_string(),
                opt_str(&d.no_licencia),
                opt_str(&d.nacionalidad),
                parse_fecha(&d.fecha_recogida)?,
                parse_hora_opt(&d.hora_recogida)?,
                opt_str(&d.ubicacion_recogida),
                parse_fecha(&d.fecha_retorno)?,
                parse_hora_opt(&d.hora_retorno)?,
                opt_str(&d.ubicacion_retorno),
                d.dias_calculados,
                d.horas_extras,
                d.valor_dia.to_string(),
                d.valor_hora_extra.to_string(),
                d.valor_dia_extra.to_string(),
                d.costo_lavado.to_string(),
                d.costo_silla.to_string(),
                d.costo_retorno.to_string(),
                d.costo_domicilio.to_string(),
                d.costo_cables.to_string(),
                d.costo_inversor.to_string(),
                d.valor_gasolina.to_string(),
                d.descuento.to_string(),
                d.subtotal.to_string(),
                d.impuestos.to_string(),
                d.total.to_string(),
                d.saldo_pendiente.to_string(),
                bool_to_i(d.cobra_iva),
                bool_to_i(d.tiene_comision),
                d.comision.to_string(),
                d.valor_neto.to_string(),
                opt_str(&d.observaciones),
                d.km_salida.parse::<f64>().unwrap_or(0.0),
                opt_str(&d.tanque_salida),
                d.id_reserva,
                id,
            ],
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Cierra una renta: estado 'Cerrada', devolución real, km/tanque finales y totales
    pub fn cerrar(
        conn: &mut PooledConnection,
        id: i64,
        d: &RentaCierreDatos,
        subtotal: &str,
        impuestos: &str,
        total: &str,
        saldo: &str,
    ) -> Result<(), AppError> {
        conn.execute(
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
                parse_fecha_opt(&d.fecha_devolucion_real)?,
                parse_hora_opt(&d.hora_devolucion_real)?,
                opt_str(&d.km_final),
                opt_str(&d.tanque_final),
                d.dias_calculados,
                d.horas_extras,
                d.valor_dia.as_deref().map(|s| s.trim().replace(',', ".")),
                d.valor_hora_extra.as_deref().map(|s| s.trim().replace(',', ".")),
                d.descuento.as_deref().map(|s| s.trim().replace(',', ".")),
                subtotal,
                impuestos,
                total,
                saldo,
                opt_str(&d.observaciones),
                id,
            ],
        )?;
        Ok(())
    }

    /// Marca una renta como cancelada
    pub fn cancelar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute(
            "UPDATE rentas SET estado = 'Cancelada' WHERE id = ?",
            (id,),
        )?;
        Ok(())
    }

    /// Soft-delete: marca la renta como borrada (deleted_at) y, en cascada
    /// lógica, sus pagos. Las inspecciones no tienen deleted_at (queda como
    /// TODO si se requiere trazabilidad total) pero dejan de ser accesibles
    /// porque la renta no aparece en los SELECTs (filtrados por deleted_at).
    pub fn eliminar(conn: &mut PooledConnection, id: i64) -> Result<(), AppError> {
        conn.execute(
            "UPDATE rentas SET deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
            (id,),
        )
        .map_err(map_fb_error)?;
        // Soft-delete en cascada de los pagos asociados (la FK original era
        // ON DELETE CASCADE; con soft-delete hay que hacerlo a mano).
        conn.execute(
            "UPDATE pagos SET deleted_at = CURRENT_TIMESTAMP \
             WHERE id_renta = ? AND deleted_at IS NULL",
            (id,),
        )
        .map_err(map_fb_error)?;
        Ok(())
    }

    /// Registra un pago contra la renta y actualiza abono/saldo pendiente
    pub fn insertar_pago(
        conn: &mut PooledConnection,
        id_renta: i64,
        p: &PagoDatos,
        usuario: &str,
        abono_nuevo: &str,
        saldo_nuevo: &str,
    ) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO pagos (id_renta, monto, metodo_pago, concepto, observaciones, usuario) \
                 VALUES (?, CAST(? AS DECIMAL(12,2)), ?, ?, ?, ?) RETURNING id",
                params![
                    id_renta,
                    p.monto.to_string(),
                    p.metodo_pago.to_string(),
                    p.concepto.to_string(),
                    opt_str(&p.observaciones),
                    usuario,
                ],
            )
            .map_err(map_fb_error)?;
        conn.execute(
            "UPDATE rentas SET abono = CAST(? AS DECIMAL(12,2)), saldo_pendiente = CAST(? AS DECIMAL(12,2)) \
             WHERE id = ?",
            (abono_nuevo.to_string(), saldo_nuevo.to_string(), id_renta),
        )?;
        Ok(id)
    }

    /// Inserta una inspección (salida/entrada)
    pub fn insertar_inspeccion(conn: &mut PooledConnection, id_renta: i64, i: &InspeccionDatos) -> Result<i64, AppError> {
        let (id,): (i64,) = conn
            .execute_returnable(
                "INSERT INTO inspecciones (\
                    id_renta, tipo, kilometraje, nivel_gasolina, limpieza, \
                    tiene_repuesto, tiene_gato_cruceta, tiene_kit_carretera, tiene_documentos, \
                    danos_carroceria, observaciones \
                 ) VALUES (?, ?, CAST(? AS DOUBLE PRECISION), ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
                params![
                    id_renta,
                    i.tipo.to_string(),
                    i.kilometraje.to_string(),
                    i.nivel_gasolina.to_string(),
                    opt_str(&i.limpieza),
                    bool_to_i(i.tiene_repuesto),
                    bool_to_i(i.tiene_gato_cruceta),
                    bool_to_i(i.tiene_kit_carretera),
                    bool_to_i(i.tiene_documentos),
                    opt_str(&i.danos_carroceria),
                    opt_str(&i.observaciones),
                ],
            )
            .map_err(map_fb_error)?;
        Ok(id)
    }

    /// Edita campos financieros de una renta CERRADA (corrección de errores de digitación).
    /// Recalcula subtotal/impuestos/total/saldo_pendiente/valor_neto con los nuevos valores.
    /// NO toca abono (se gestiona por pagos) ni estado (permanece 'Cerrada').
    pub fn editar_cerrada(
        conn: &mut PooledConnection,
        id: i64,
        d: &RentaCierreEditDatos,
        subtotal: &str,
        impuestos: &str,
        total: &str,
        saldo: &str,
        valor_neto: &str,
    ) -> Result<(), AppError> {
        conn.execute(
            "UPDATE rentas SET \
                valor_dia = CAST(COALESCE(?, valor_dia) AS DECIMAL(12,2)), \
                valor_hora_extra = CAST(COALESCE(?, valor_hora_extra) AS DECIMAL(12,2)), \
                dias_calculados = COALESCE(?, dias_calculados), \
                horas_extras = COALESCE(?, horas_extras), \
                descuento = CAST(COALESCE(?, descuento) AS DECIMAL(12,2)), \
                subtotal = CAST(? AS DECIMAL(12,2)), \
                impuestos = CAST(? AS DECIMAL(12,2)), \
                total = CAST(? AS DECIMAL(12,2)), \
                saldo_pendiente = CAST(? AS DECIMAL(12,2)), \
                valor_neto = CAST(? AS DECIMAL(12,2)), \
                observaciones = COALESCE(?, observaciones) \
             WHERE id = ?",
            params![
                d.valor_dia.as_deref().map(|s| s.trim().replace(',', ".")),
                d.valor_hora_extra.as_deref().map(|s| s.trim().replace(',', ".")),
                d.dias_calculados,
                d.horas_extras,
                d.descuento.as_deref().map(|s| s.trim().replace(',', ".")),
                subtotal,
                impuestos,
                total,
                saldo,
                valor_neto,
                d.observaciones.as_deref().map(|s| s.trim().to_string()),
                id,
            ],
        )?;
        Ok(())
    }

    /// Rentas activas (para el calendario y el dashboard)
    pub fn activas(conn: &mut PooledConnection) -> Result<Vec<Renta>, AppError> {
        let empty = ParamsType::Positional(vec![]);
        Self::consultar(
            conn,
            "WHERE r.estado = 'Activa' OR r.estado = 'Activo'",
            &empty,
        )
    }

    /// Total de rentas
    pub fn contar(conn: &mut PooledConnection) -> Result<i64, AppError> {
        let count: Option<(i64,)> =
            conn.query_first("SELECT COUNT(*) FROM rentas WHERE deleted_at IS NULL", ())?;
        Ok(count.map(|(c,)| c).unwrap_or(0))
    }

    /// Conteo por estado
    pub fn contar_por_estado(conn: &mut PooledConnection) -> Result<Vec<(String, i64)>, AppError> {
        let rows: Vec<(String, i64)> = conn.query(
            "SELECT estado, COUNT(*) FROM rentas WHERE deleted_at IS NULL GROUP BY estado ORDER BY estado",
            (),
        )?;
        Ok(rows)
    }
}

fn bool_to_i(b: bool) -> i64 {
    if b { 1 } else { 0 }
}

/// Recorta 'HH:MM:SS.0000' (Firebird) a 'HH:MM' para la UI
fn hora_corta(h: &str) -> String {
    h.split(':').take(2).collect::<Vec<_>>().join(":")
}

/// Normaliza el km (DOUBLE PRECISION serializado por Firebird con cola de
/// ceros, p. ej. "42000.000000000000") a su forma compacta ("42000").
/// Conserva los decimales significativos: "12000.5" → "12000.5".
fn km_limpio(v: &str) -> String {
    v.parse::<f64>()
        .map(|n| format!("{n}"))
        .unwrap_or_else(|_| v.trim().to_string())
}


