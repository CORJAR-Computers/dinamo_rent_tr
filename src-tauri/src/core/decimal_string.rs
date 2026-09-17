//! Deserialización tolerante de montos monetarios (`String` | número JSON).
//!
//! Todos los montos del sistema se almacenan y transmiten como `String`
//! (DECIMAL de Firebird serializado a texto, sin errores de coma flotante).
//! El riesgo: si la UI envía un número JSON donde el backend espera `String`,
//! serde rechaza el comando completo con
//! `invalid type: integer `150000`, expected a string` y la operación de
//! cobro se bloquea (incidente del 2026-09-16 con `extender_renta`).
//!
//! Con `#[serde(deserialize_with = "crate::core::decimal_string::decimal_string")]`
//! los campos monetarios aceptan AMBAS formas: el string canónico y el número
//! JSON (entero o decimal, sin pérdida de precisión para enteros de hasta 64
//! bits), dejando la validación de negocio (monto > 0, dígitos, etc.) al
//! servicio, que es quien produce mensajes útiles para el usuario.
//!
//! Nota: este deserializador es una **red de seguridad de contrato**, no una
//! invitación a enviar números desde la UI: el tipo canónico sigue siendo
//! `string` (ver `src/lib/api/*.ts` y los `.ts` generados por ts-rs).

use serde::de::{self, Deserializer, Visitor};
use std::fmt;

/// Visitor que acepta string, número o null y produce `Option<String>`.
/// `None` solo para `null` (o campo ausente vía `#[serde(default)]`).
struct MontoVisitor;

impl<'de> Visitor<'de> for MontoVisitor {
    type Value = Option<String>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("un monto monetario: string o número JSON")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Some(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Some(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(v.to_string()))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Some(v.to_string()))
    }

    /// Los decimales JSON llegan como f64. `Display` produce la representación
    /// más corta que redondea (p. ej. `150000.5`), suficiente para montos que
    /// la UI envía con 2 decimales como máximo.
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        if v.is_finite() {
            Ok(Some(format!("{v}")))
        } else {
            Err(de::Error::custom(format!("número no finito: {v}")))
        }
    }

    /// `null` explícito (solo alcanzable vía `deserialize_any` de serde_json).
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(self)
    }
}

/// Acepta `"150000"`, `150000` o `150000.5`; `null` → `String::new()` para
/// campos `String` NOT NULL (la validación de negocio decide si es válido).
pub fn decimal_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(deserializer
        .deserialize_any(MontoVisitor)?
        .unwrap_or_default())
}

/// Igual que [`decimal_string`] pero para campos `Option<String>`:
/// `null`/ausente → `None`, número/string → `Some(...)`.
pub fn option_decimal_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(MontoVisitor)
}

#[cfg(test)]
mod tests {
    use super::{decimal_string, option_decimal_string};
    use serde::Deserialize;

    #[derive(Debug, Default, Deserialize)]
    #[serde(default)]
    struct Dto {
        #[serde(deserialize_with = "decimal_string")]
        monto: String,
        #[serde(deserialize_with = "option_decimal_string")]
        descuento: Option<String>,
    }

    fn parse(json: &str) -> Dto {
        serde_json::from_str(json).expect("el DTO debería deserializar")
    }

    #[test]
    fn acepta_string_canonico() {
        let d = parse(r#"{"monto": "150000", "descuento": "0"}"#);
        assert_eq!(d.monto, "150000");
        assert_eq!(d.descuento.as_deref(), Some("0"));
    }

    #[test]
    fn acepta_entero_json_como_string() {
        // Reproduce el incidente de extender_renta: 150000 sin comillas.
        let d = parse(r#"{"monto": 150000}"#);
        assert_eq!(d.monto, "150000");
    }

    #[test]
    fn acepta_decimal_json_como_string() {
        let d = parse(r#"{"monto": 150000.5}"#);
        assert_eq!(d.monto, "150000.5");
    }

    #[test]
    fn entero_grande_sin_perdida_de_precision() {
        // 2^53 + 1 (el clásico valor que JS corrompería como float).
        let d = parse(r#"{"monto": 9007199254740993}"#);
        assert_eq!(d.monto, "9007199254740993");
    }

    #[test]
    fn nulo_en_campo_string_queda_vacio() {
        let d = parse(r#"{"monto": null}"#);
        assert_eq!(d.monto, "");
    }

    #[test]
    fn campo_ausente_usa_default() {
        let d = parse("{}");
        assert_eq!(d.monto, "");
        assert_eq!(d.descuento, None);
    }

    #[test]
    fn nulo_en_option_es_none() {
        let d = parse(r#"{"monto": "100", "descuento": null}"#);
        assert_eq!(d.descuento, None);
    }

    #[test]
    fn numero_en_option_es_some() {
        let d = parse(r#"{"monto": "100", "descuento": 5000}"#);
        assert_eq!(d.descuento.as_deref(), Some("5000"));
    }

    #[test]
    fn enteros_negativos_llegan_al_servicio_para_validarse() {
        // La red de seguridad NO valida negocio: −1 llega como "-1" y el
        // servicio responde con su mensaje de validación.
        let d = parse(r#"{"monto": -1}"#);
        assert_eq!(d.monto, "-1");
    }

    #[test]
    fn tipos_no_numericos_siguen_fallando() {
        let err = serde_json::from_str::<Dto>(r#"{"monto": true}"#)
            .expect_err("un booleano no es un monto");
        assert!(
            err.to_string().contains("monto monetario"),
            "mensaje inesperado: {err}"
        );
    }
}
