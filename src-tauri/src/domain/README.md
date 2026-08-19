# `domain/` — Capa de dominio (scaffold)

> **Estado**: scaffold documental. No hay código de producción aquí todavía.
> Creado en el **Bloque 4 / TAREA 4.4** como guía para una futura migración
> incremental hacia value objects. **NO** implementar todo de golpe (sería
> un cambio XL que rompería la API existente).

## ¿Por qué una capa de dominio?

Hoy los servicios y repositorios operan sobre **tipos primitivos**:

```rust
// services/renta.rs (estado actual)
pub fn registrar_pago(
    conn: &mut PooledConnection,
    id_renta: i64,                    // ← i64 crudo, podría ser cualquier entero
    usuario: &str,                    // ← &str crudo
    pago: PagoDatos,                  // ← PagoDatos.monto: String (!)
) -> Result<Pago, AppError> {
    let monto = Decimal::from_str(&pago.monto).map_err(...)?;  // ← parseo manual
    if monto <= Decimal::ZERO { ... }                            // ← validación dispersa
    let abono_nuevo = (dec_str(&renta.abono) + monto).round_dp(2);  // ← redondeo manual
    let saldo_nuevo = (saldo_actual - monto).round_dp(2);            // ← redondeo manual
    ...
}
```

Esto causa cuatro clases de bugs:

### 1. Parseo duplicado
`parse_fecha` / `parse_hora` existían en 9+ archivos antes de que el
**Bloque 4 / TAREA 4.2** los centralizara en `core/repository.rs`. La
centralización alivió el síntoma, pero el problema de fondo es que el
tipo `String` **no garantiza** que el contenido sea una fecha válida. Un
`RentaDatos.fecha_recogida: String` puede contener cualquier cosa.

### 2. Redondeo manual
El redondeo a 2 dp se hace a mano con `Decimal::round_dp(2)` en cada
operación financiera. Si se olvida en un sitio, el saldo o el total se
descuadran. El bug puede pasar desapercibido en tests y aparecer en
producción con ciertas combinaciones de importes.

### 3. Contrato FE↔BE frágil
El dinero se serializa como `String` (`"123.45"`) para preservar precisión
decimal, pero el frontend espera un string en formato específico. Sin un
value object `Dinero`, cualquier módulo puede romper el contrato (un
`format!("{:.2}", x)` vs `format!("{}", x)` da resultados distintos).

### 4. Validación dispersa
La placa se valida en `core/validators.rs` pero también en
`services/auto.rs` (al crear) y en el frontend. Un value object `Placa`
validaría una sola vez (en el constructor) y el tipo garantizaría el
invariante en todo el flujo.

## Value objects propuestos

| VO             | Reemplaza                     | Invariante garantizado                          |
|----------------|-------------------------------|-------------------------------------------------|
| `Dinero`       | `String` / `Decimal` suelto   | ≥ 0, 2 dp, formateo consistente                 |
| `Placa`        | `String` / `Option<String>`   | Mayúsculas, sin espacios, formato AAA000/A000AAA |
| `RangoFechas`  | `(NaiveDate, NaiveDate)`      | `inicio <= fin`, no vacío                       |
| `RentaId`      | `i64`                         | > 0 (no es un índice de array)                  |
| `ClienteId`    | `i64`                         | > 0                                             |
| `EstadoRenta`  | `String`                     | Uno de: Activa, Cerrada, Cancelada              |

## Ejemplo de value object `Dinero`

```rust
// src/domain/dinero.rs (NO implementado aún — scaffold)
use rust_decimal::Decimal;
use std::ops::{Add, Sub, Mul};

use crate::core::error::AppError;

/// Dinero en COP (Pesos colombianos). Siempre ≥ 0, siempre con 2 decimales.
///
/// Invariantes garantizados por el constructor:
///   1. `valor >= 0` (no hay dinero negativo; los descuentos se modelan aparte).
///   2. Redondeo a 2 dp aplicado en el constructor (cualquier operación
///      interna preserva la precisión).
///   3. `Display` formatea siempre con 2 decimales ("123.45", nunca "123.4").
///
/// Constructores:
///   - `Dinero::from_str("123.45")` — parsea string del frontend.
///   - `Dinero::from_decimal(d)` — wraps un Decimal ya calculado.
///   - `Dinero::zero()` — para inicializar acumuladores.
///
/// Operaciones (todas devuelven `Dinero`, redondean a 2 dp):
///   - `Dinero + Dinero -> Dinero`
///   - `Dinero - Dinero -> Dinero` (panics si resultado < 0; usar `saturating_sub` para descuentos)
///   - `Dinero * i64 -> Dinero` (ej. precio_dia * dias)
///   - `Dinero * Decimal -> Dinero` (ej. aplicar IVA 19%)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dinero(Decimal);

impl Dinero {
    pub fn from_str(s: &str) -> Result<Self, AppError> {
        let d = Decimal::from_str(s.trim()).map_err(|_| {
            AppError::Validation(format!("Monto inválido: '{s}'"))
        })?;
        Self::from_decimal(d)
    }

    pub fn from_decimal(d: Decimal) -> Result<Self, AppError> {
        if d < Decimal::ZERO {
            return Err(AppError::Validation(
                "El monto no puede ser negativo.".into(),
            ));
        }
        Ok(Dinero(d.round_dp(2)))
    }

    pub fn zero() -> Self {
        Dinero(Decimal::ZERO)
    }

    pub fn to_string(&self) -> String {
        // Siempre 2 dp: "123.00", no "123"
        format!("{:.2}", self.0)
    }

    pub fn as_decimal(&self) -> Decimal {
        self.0
    }
}

impl Add for Dinero {
    type Output = Dinero;
    fn add(self, rhs: Dinero) -> Dinero {
        Dinero((self.0 + rhs.0).round_dp(2))
    }
}

impl Sub for Dinero {
    type Output = Result<Dinero, AppError>;
    fn sub(self, rhs: Dinero) -> Result<Dinero, AppError> {
        let r = self.0 - rhs.0;
        Self::from_decimal(r)
    }
}

impl Mul<i64> for Dinero {
    type Output = Dinero;
    fn mul(self, rhs: i64) -> Dinero {
        Dinero((self.0 * Decimal::from(rhs)).round_dp(2))
    }
}

impl Mul<Decimal> for Dinero {
    type Output = Dinero;
    fn mul(self, rhs: Decimal) -> Dinero {
        Dinero((self.0 * rhs).round_dp(2))
    }
}
```

### Uso (hipotético) en `services/renta.rs::registrar_pago`

```rust
// ANTES (estado actual):
let monto = Decimal::from_str(&pago.monto).map_err(...)?;
if monto <= Decimal::ZERO { return Err(...); }
let abono_nuevo = (dec_str(&renta.abono) + monto).round_dp(2);
let saldo_nuevo = (saldo_actual - monto).round_dp(2);

// DESPUÉS (con value objects):
let monto = Dinero::from_str(&pago.monto)?;
let abono_actual = Dinero::from_str(&renta.abono)?;
let saldo_actual = Dinero::from_str(&renta.saldo_pendiente)?;
if monto > saldo_actual {
    return Err(AppError::Business(format!(
        "El pago ({}) supera el saldo pendiente ({}).",
        monto.to_string(), saldo_actual.to_string()
    )));
}
let abono_nuevo = abono_actual + monto;       // redondeo automático
let saldo_nuevo = (saldo_actual - monto?)?;   // saturating, no negativo
```

## Cómo migrar incrementalmente

**NO intentar migrar todo de golpe** — sería un cambio XL que rompería
la API existente y requeriría tocar ~30 archivos simultáneamente.

### Fase 1 — `Dinero` en `services/renta.rs` (1-2 días)

1. Crear `src/domain/dinero.rs` con el VO del ejemplo de arriba.
2. Registrar en `src/domain/mod.rs` (`pub mod dinero;`).
3. En `services/renta.rs::registrar_pago`, reemplazar `pago.monto: String`
   por `pago.monto: Dinero` (requiere `Deserialize` para `Dinero`).
4. Migrar `cerrar` y `editar_cerrada` al mismo patrón.
5. Tests existentes (`tests/renta.rs`) deben pasar sin cambios — el
   contract FE↔BE (`String` en JSON) se preserva con `serde` custom.

### Fase 2 — `Placa` en `repositories/auto.rs` (medio día)

1. Crear `src/domain/placa.rs` con VO `Placa` (mayúsculas, sin espacios).
2. Reemplazar `placa: String` por `placa: Placa` en `Auto`, `AutoDatos`.
3. Los repositorios usan `placa.as_str()` para las queries SQL.
4. Eliminar la validación dispersa en `services/auto.rs` y `validators.rs`.

### Fase 3 — `RangoFechas` en `services/renta.rs::calcular_dias_horas` (medio día)

1. Crear `src/domain/rango_fechas.rs` con VO `RangoFechas { inicio, fin }`.
2. El constructor valida `inicio <= fin`.
3. `calcular_dias_horas` toma `&RangoFechas` en vez de `(fecha_recogida, fecha_retorno)`.
4. Eliminar `parse_fecha_ref` / `parse_hora_ref` de `services/renta.rs`.

### Fase 4 — Migración completa (~1 semana)

- `RentaId`, `ClienteId`, `AutoId` como newtypes sobre `i64`.
- `EstadoRenta` como enum con variantes `Activa`, `Cerrada`, `Cancelada`.
- `Renta` (el struct de respuesta) usa VOs en sus campos.
- `ts-rs` (Bloque 4 / TAREA 4.3) genera el contrato TypeScript a partir
  de los VOs automáticamente.

## Referencias

- Eric Evans, *Domain-Driven Design* (2003), cap. 5 "A Model Expressed in Software".
- Vaughn Vernon, *Implementing Domain-Driven Design* (2013), cap. 5 "Entities" y cap. 6 "Value Objects".
- Rust newtype pattern: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
- `rust_decimal` crate: https://docs.rs/rust_decimal
- Discusión previa en este repo: worklog Task UI-Redesign §3 (issue de redondeo
  en `cerrar_renta` que motivó este diseño).
