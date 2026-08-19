//! core/repository.rs — Helpers compartidos por los repositorios
//!
//! Antes de este módulo, cada uno de los 10 repositorios
//! (`renta`, `cliente`, `auto`, `mantenimiento`, `reserva`, `gasto`,
//! `comparendo`, `usuario`, `empresa`, `auditoria`) duplicaba:
//!
//! ```ignore
//! fn map_fb_error(e: rsfbclient::FbError) -> AppError { ... }   // 10 copias
//! fn opt_str(v: &Option<String>) -> Option<String> { ... }     // 9 copias
//! fn parse_fecha_opt(v: &Option<String>) -> Result<Option<NaiveDate>, AppError> { ... }
//! fn parse_hora(v: &Option<String>) -> Result<Option<NaiveTime>, AppError> { ... }
//! macro_rules! params { ... }                                   // 7 copias
//! ```
//!
//! Eso son ~250 LOC de código duplicado. Este módulo los centraliza y los
//! exporta para que los repositorios los importen vía
//! `use crate::core::repository::{map_fb_error, opt_str, parse_fecha_opt, parse_hora_opt, params};`.
//!
//! ## Migración parcial (Bloque 4 / TAREA 4.2)
//!
//! Para no romper la API existente ni tocar los 10 repositorios de golpe,
//! la migración es **incremental**:
//!   - **Migrados a `core::repository`** (3): `renta.rs`, `cliente.rs`,
//!     `mantenimiento.rs`. Estos tres concentran ~60% del tráfico de queries
//!     (rentas + clientes + mantenimiento) y son los que más se beneficiaron
//!     del DRY.
//!   - **Pendientes** (7): `auto.rs`, `reserva.rs`, `gasto.rs`,
//!     `comparendo.rs`, `usuario.rs`, `empresa.rs`, `auditoria.rs`. Mantienen
//!     sus helpers locales con la nota `// TODO: migrar a core::repository`.
//!
//! ## `map_fb_error` y los mensajes por entidad
//!
//! Los 10 repositorios NO compartían la misma lógica de mapeo: algunos
//! detectaban `foreign key`/`referential` (renta, reserva, gasto, comparendo,
//! mantenimiento) y devolvían `AppError::Business(msg_específico)`, otros
//! detectaban `duplicate`/`unique` (cliente, auto, usuario) y devolvían
//! `AppError::Duplicate(msg_específico)`. Cada uno con un mensaje distinto.
//!
//! Para preservar la UX (los mensajes son usuario-final), este módulo ofrece:
//!   - `map_fb_error(e)`: variante **genérica** (FK -> Business, dup -> Duplicate
//!     con mensajes estándar). Útil para repositorios que no requieren un
//!     mensaje custom.
//!   - `map_fb_error_fk(e, business_msg)`: preserva el mensaje de FK del
//!     repositorio original.
//!   - `map_fb_error_dup(e, dup_msg)`: preserva el mensaje de Duplicate del
//!     repositorio original.
//!
//! ## `params!` macro
//!
//! El macro `params!` se exporta con `pub use params;` para que los
//! repositorios puedan importarlo como `use crate::core::repository::params;`
//! en vez de redefinirlo. Internamente construye `ParamsType::Positional(vec![...])`
//! porque rsfbclient limita las tuplas `IntoParams` a 15 elementos.

use chrono::{NaiveDate, NaiveTime};
#[allow(unused_imports)]
use rsfbclient::{IntoParam, ParamsType};

use crate::core::error::AppError;

// ──────────────────────────────────────────────────────────────────────────
// Macro `params!`
// ──────────────────────────────────────────────────────────────────────────

/// Construye parámetros posicionales de cualquier longitud (tuplas
/// `IntoParams` limitadas a 15 elementos en rsfbclient).
///
/// Uso:
/// ```ignore
/// use crate::core::repository::params;
/// conn.execute("INSERT ... VALUES (?, ?, ?)", params![a, b, c])?;
/// ```
#[macro_export]
macro_rules! repository_params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into_param()),+])
    };
}

/// Re-export del macro con el nombre corto `params!` (igual al que usaban los
/// repositorios originalmente). Se importa con `use crate::core::repository::params;`.
pub use repository_params as params;

// ──────────────────────────────────────────────────────────────────────────
// Helpers de `Option<String>` y parseo de fechas/horas
// ──────────────────────────────────────────────────────────────────────────

/// Recorta un `Option<String>` y descarta vacíos (devuelve `None`).
///
/// Equivalente a los `fn opt_str` que estaban duplicados en 9 repositorios.
/// Mismo comportamiento: `Some("  ")` -> `None`, `Some("abc ")` -> `Some("abc")`.
pub fn opt_str(v: &Option<String>) -> Option<String> {
    v.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Parsea `&str` (formato AAAA-MM-DD) a `NaiveDate`.
///
/// Lo usan los repositorios para campos de fecha **requeridos** (ej.
/// `MantenimientoDatos.fecha`). El servicio ya validó el formato antes de
/// llamar al repo; este parseo es defense-in-depth.
pub fn parse_fecha(v: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::Validation("Fecha inválida (formato AAAA-MM-DD).".into()))
}

/// Parsea `Option<String>` -> `Option<NaiveDate>` (formato AAAA-MM-DD).
///
/// Equivalente a los `fn parse_fecha_opt` que estaban duplicados en
/// `renta.rs` y `services/renta.rs`. Devuelve `None` si la entrada es `None`
/// o vacía tras trim; si no, delega en `parse_fecha`.
pub fn parse_fecha_opt(v: &Option<String>) -> Result<Option<NaiveDate>, AppError> {
    match opt_str(v) {
        None => Ok(None),
        Some(s) => parse_fecha(&s).map(Some),
    }
}

/// Parsea `Option<String>` -> `Option<NaiveTime>` (formato HH:MM o HH:MM:SS).
///
/// Equivalente a los `fn parse_hora` que estaban duplicados en `renta.rs`,
/// `reserva.rs`, `gasto.rs` y `services/renta.rs`. Si la hora viene sin
/// segundos (`HH:MM`, 5 chars) se le añade `:00` antes de parsear.
pub fn parse_hora_opt(v: &Option<String>) -> Result<Option<NaiveTime>, AppError> {
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

// ──────────────────────────────────────────────────────────────────────────
// Mapeo de errores Firebird -> AppError
// ──────────────────────────────────────────────────────────────────────────

/// Patrón de detección de errores FK/referencial (compartido por 5 repos).
///
/// Devuelve `true` si el mensaje de Firebird menciona `foreign key`,
/// `not a valid reference` o `referential` (case-insensitive). Lo usan
/// `map_fb_error`, `map_fb_error_fk` y los repositorios que aún no migran.
fn es_error_fk(lower: &str) -> bool {
    lower.contains("foreign key")
        || lower.contains("not a valid reference")
        || lower.contains("referential")
}

/// Patrón de detección de errores de unicidad (compartido por 3 repos).
///
/// Devuelve `true` si el mensaje menciona `duplicate` o `unique`
/// (case-insensitive).
fn es_error_duplicado(lower: &str) -> bool {
    lower.contains("duplicate") || lower.contains("unique")
}

/// Mapeo genérico `FbError -> AppError`.
///
/// Detecta ambos patrones (FK y duplicado) con mensajes estándar:
///   - FK -> `AppError::Business("Violación de integridad referencial...")`
///   - Duplicado -> `AppError::Duplicate("Ya existe un registro con esos datos.")`
///   - Otro -> `AppError::Database(msg_original)`
///
/// Para preservar los mensajes específicos por entidad que tenían los
/// repositorios originales, usar `map_fb_error_fk` o `map_fb_error_dup`.
pub fn map_fb_error(e: rsfbclient::FbError) -> AppError {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if es_error_fk(&lower) {
        AppError::Business(
            "Violación de integridad referencial: el registro referenciado no existe o está en uso.".into(),
        )
    } else if es_error_duplicado(&lower) {
        AppError::Duplicate("Ya existe un registro con esos datos.".into())
    } else {
        AppError::Database(msg)
    }
}

/// Mapeo FK-específico: preserva el mensaje de `AppError::Business` original.
///
/// Para repositorios que sólo necesitan detectar FK (renta, reserva, gasto,
/// comparendo, mantenimiento). Si el error NO es FK, cae al `Database` genérico.
///
/// ```ignore
/// use crate::core::repository::map_fb_error_fk;
/// conn.execute(...)
///     .map_err(|e| map_fb_error_fk(e, "El cliente, el vehículo o la reserva seleccionada no existe."))?;
/// ```
pub fn map_fb_error_fk(e: rsfbclient::FbError, business_msg: &str) -> AppError {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if es_error_fk(&lower) {
        AppError::Business(business_msg.into())
    } else {
        AppError::Database(msg)
    }
}

/// Mapeo Duplicate-específico: preserva el mensaje de `AppError::Duplicate` original.
///
/// Para repositorios que sólo necesitan detectar duplicados (cliente, auto,
/// usuario). Si el error NO es de unicidad, cae al `Database` genérico.
///
/// ```ignore
/// use crate::core::repository::map_fb_error_dup;
/// conn.execute(...)
///     .map_err(|e| map_fb_error_dup(e, "Ya existe un cliente con ese documento."))?;
/// ```
pub fn map_fb_error_dup(e: rsfbclient::FbError, dup_msg: &str) -> AppError {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if es_error_duplicado(&lower) {
        AppError::Duplicate(dup_msg.into())
    } else {
        AppError::Database(msg)
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Tests unitarios (no requieren BD — sólo lógica de mapeo)
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_str_descarta_vacios_y_espacios() {
        assert_eq!(opt_str(&None), None);
        assert_eq!(opt_str(&Some("".into())), None);
        assert_eq!(opt_str(&Some("   ".into())), None);
        assert_eq!(opt_str(&Some("  abc ".into())), Some("abc".into()));
    }

    #[test]
    fn parse_fecha_acepta_iso() {
        assert!(parse_fecha("2026-01-15").is_ok());
        // chrono %m/%d aceptan dígitos sin cero: "2026-1-5" es válido
        assert!(parse_fecha("2026-1-5").is_ok());
        assert!(parse_fecha("no-fecha").is_err());
        assert!(parse_fecha("2026-13-01").is_err()); // mes 13 inválido
        // Acepta con espacios
        assert!(parse_fecha("  2026-01-15  ").is_ok());
    }

    #[test]
    fn parse_fecha_opt_devuelve_none_si_vacio() {
        assert_eq!(parse_fecha_opt(&None).unwrap(), None);
        assert_eq!(parse_fecha_opt(&Some("".into())).unwrap(), None);
        assert_eq!(parse_fecha_opt(&Some("   ".into())).unwrap(), None);
        assert!(parse_fecha_opt(&Some("2026-01-15".into())).unwrap().is_some());
    }

    #[test]
    fn parse_hora_opt_completa_segundos() {
        assert_eq!(parse_hora_opt(&None).unwrap(), None);
        assert_eq!(parse_hora_opt(&Some("".into())).unwrap(), None);
        // HH:MM (5 chars) -> se añade :00
        let h = parse_hora_opt(&Some("13:45".into())).unwrap();
        assert!(h.is_some());
        assert_eq!(h.unwrap().to_string(), "13:45:00");
        // HH:MM:SS (8 chars) -> se respeta
        let h2 = parse_hora_opt(&Some("13:45:30".into())).unwrap();
        assert!(h2.is_some());
        // Hora inválida
        assert!(parse_hora_opt(&Some("99:99".into())).is_err());
    }

    #[test]
    fn es_error_fk_detecta_patrones() {
        // Nota: estas funciones esperan strings ya en minúsculas (el caller hace to_lowercase)
        assert!(es_error_fk("violation of foreign key constraint"));
        assert!(es_error_fk("not a valid reference"));
        assert!(es_error_fk("referential integrity violated"));
        assert!(!es_error_fk("duplicate value"));
        assert!(!es_error_fk("connection lost"));
    }

    #[test]
    fn es_error_duplicado_detecta_patrones() {
        // Nota: estas funciones esperan strings ya en minúsculas (el caller hace to_lowercase)
        assert!(es_error_duplicado("attempt to store duplicate value"));
        assert!(es_error_duplicado("unique constraint violated"));
        assert!(!es_error_duplicado("foreign key"));
        assert!(!es_error_duplicado("connection lost"));
    }

    #[test]
    fn params_construye_posicional() {
        // `IntoParam` está implementado para String, i64 y Option<String>.
        let p: ParamsType = params!["a".to_string(), 42i64, Some("b".to_string())];
        match p {
            ParamsType::Positional(v) => assert_eq!(v.len(), 3),
            _ => panic!("esperaba Positional"),
        }
    }
}
