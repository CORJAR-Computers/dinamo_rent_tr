//! domain/ — Capa de dominio (scaffold — Bloque 4 / TAREA 4.4)
//!
//! Esta carpeta es un **scaffold** para una futura migración incremental
//! hacia una arquitectura DDD-lite con value objects. **No hay código
//! de producción aquí todavía** — solo documentación (ver `README.md`).
//!
//! ## Por qué existe (y por qué está vacía por ahora)
//!
//! Hoy los servicios (`services/renta.rs`, etc.) operan directamente sobre
//! tipos primitivos:
//!   - `String` para representar placas, dinero, fechas y horas.
//!   - `Decimal` para importes, pero sin encapsular redondeo ni signo.
//!   - `Option<String>` para fechas opcionales, parseadas en cada repo.
//!
//! Esto causa:
//!   1. **Bug de parseo duplicado**: `parse_fecha` / `parse_hora` existen en
//!      9+ archivos (Bloque 4 / TAREA 4.2 las centralizó en
//!      `core::repository`, pero el problema de fondo es que el tipo no
//!      garantiza invariantes).
//!   2. **Bug de redondeo**: el redondeo a 2 dp se hace a mano con
//!      `Decimal::round_dp(2)` en cada operación. Si se olvida en un sitio,
//!      el saldo o el total se descuadran.
//!   3. **Bug de formato**: el dinero se serializa como `String` ("123.45")
//!      pero el frontend espera un string en formato específico. Sin un VO
//!      `Dinero`, cualquier módulo puede romper el contrato.
//!   4. **Bug de validación dispersa**: la placa se valida en `validators.rs`
//!      pero también en `services/auto.rs` y en el frontend. Un VO `Placa`
//!      validaría una sola vez (en el constructor).
//!
//! ## Plan de migración incremental
//!
//! Ver `README.md` en este directorio para el detalle completo y ejemplos
//! de value objects (`Dinero`, `Placa`, `RangoFechas`).
//!
//! ## Nota sobre `core/` vs `domain/`
//!
//! `core/` contiene utilidades **transversales** (crypto, db, error, rbac,
//! config) que no son específicas del negocio de rentas de vehículos.
//! `domain/` contendrá tipos **del negocio** (Dinero, Placa, RentaId,
//! ClienteId, RangoFechas, EstadoRenta). La separación sigue el patrón
//! "Hexagonal Architecture": `core/` = infraestructura compartida,
//! `domain/` = modelo del negocio, `services/` = casos de uso.
