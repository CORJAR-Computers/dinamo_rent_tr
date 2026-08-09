# Handsoff — Dinamo Rent ERP (Tauri + SvelteKit + Firebird)

> Última actualización: **2026-08-08** · Estado: **todos los módulos operativos, validación verde**

> **Números de contrato secuenciales (08-08):** la renta ahora tiene `no_contrato`, un número
> de contrato **secuencial e independiente del id** de la renta, generado por el generator
> Firebird `GEN_RENTA_NO_CONTRATO` (migración `0003_no_contrato.sql`): el INSERT usa
> `NEXT VALUE FOR` (atómico, sin trigger) y la migración hizo backfill de las rentas existentes
> con `GEN_ID()` (columna `NOT NULL`, índice único `ix_rentas_no_contrato`). Se muestra con
> formato **`C-####`** (padding a 4, p. ej. `C-0042`) en los tres lugares: **listado de rentas**
> (columna «Contrato», con el id interno como subtexto), **orden de renta** (encabezado y pie)
> y **contrato imprimible** (`CONTRATO Nº: C-0042`). Viaja en la API como `noContrato`.

> **Avances del 07-08 al 08-08 (post-handsoff):** `reservas.test.ts` creado, documentos
> imprimibles de Reserva y Comparendo, Informes refactorizado a **rango de fechas**
> (`fecha_inicio`/`fecha_fin`) en vez de mes calendario, y **papel Carta/Letter para todos los
> documentos** (en Colombia el tamaño comercial es Carta, no A4). El contrato de renta pasó a
> ser un **documento independiente** (no embebido en la orden): `ContratoRenta.svelte` reescrito
> con el texto legal real de `Contrato_Dinamo.docx` (14 cláusulas + póliza + firmas) e impresión
> multi-página con el nuevo util `imprimirDocumento()` (clon al body, evita el recorte del
> `position: fixed`). Detalle en las secciones 2 y 3.

---

## 1. Estado general

Proyecto de renta de vehículos: frontend **SvelteKit 5** (`src/`), backend **Tauri/Rust**
(`src-tauri/`) con **Firebird 5** embebido y pool `r2d2` (`rsfbclient`).

| Validación | Resultado |
|---|---|
| Vitest (frontend) | **133/133** en 19 archivos |
| `npm run check` (svelte-check) | **0 errores / 0 warnings** |
| `npm run build` (vite) | ✅ |
| `cargo test` (Rust) | ✅ unit + integraciones por módulo |
| `cargo check --all-targets` | ✅ 0 errores / 0 warnings |

**Regla crítica de rsfbclient:** solo implementa `FromRow` para tuplas de **≤26 elementos**
y `IntoParams` para tuplas de **≤15**. Cualquier SELECT largo debe partirse en dos consultas
(ver `repositories/renta.rs`: `SELECT_COLS_A` + `SELECT_COLS_B`) y los INSERT usar el macro
`params!` (posiciónal, longitud libre).

---

## 2. Módulos implementados (5 pendientes → 5 completos)

### ✅ Rentas (`/rentas`)
- **Backend:** `repositories/renta.rs` (CRUD + pagos + inspecciones, consulta dividida 26+15),
  `services/renta.rs` (totales con impuesto `business.impuesto_porcentaje`, cierre con
  devolución real, pagos/abono/saldo, inspecciones Salida/Entrada, cancelación),
  `commands/renta.rs`, registrado en `lib.rs`. Config: `business.impuesto_porcentaje=19` en `config.ini`.
- **Frontend:** `api.ts` (`rentaApi`), `+page.svelte` (listado con filtros, modal crear/editar con
  calculadora en vivo, modal cierre, modal pago, modal inspección con toggle Salida/Entrada),
  `OrdenRenta.svelte` (documento imprimible **Carta** con desglose completo, pagos, inspecciones).
  La impresión obtiene el detalle con `obtener_renta` (el listado no incluye pagos/inspecciones).
  **Impresión:** el modal de la orden tiene botón «Ver contrato (Carta)» que abre el contrato
  como **documento independiente** (modal propio).
- **Número de contrato:** campo `no_contrato` secuencial e independiente del id
  (generator `GEN_RENTA_NO_CONTRATO`, asignado en el INSERT con `NEXT VALUE FOR`; migración
  `0003_no_contrato.sql` con backfill + índice único). El frontend lo recibe como `noContrato`
  y el contrato imprimible lo muestra en «CONTRATO Nº».
- **Tests:** `tests/rentas_integration.rs` (6, incl. creación con abono inicial → saldo = total − abono
  y `no_contrato` secuencial +1 e independiente del id) · `src/routes/rentas/rentas.test.ts` (11,
  incl. impresión con pagos/inspecciones y apertura del contrato como documento independiente).
- **Nota:** la tabla `rentas` **no tiene `updated_at`** (solo `created_at`); los UPDATE no lo usan.
  No se debe agregar `updated_at = CURRENT_TIMESTAMP` a UPDATEs de `rentas`.

### ✅ Comparendos (`/comparendos`)
- **Backend:** `repositories/comparendo.rs`, `services/comparendo.rs` (valida placa existente en
  autos, estados Pendiente/Pagado, `marcar_pagado`), `commands/comparendo.rs`.
- **Frontend:** `api.ts` (`comparendoApi`), `+page.svelte` (filtros por estado/placa, CRUD, marcar pagado,
  **modal imprimible** con `OrdenComparendo.svelte`).
- **Tests:** `tests/comparendos_integration.rs` (3) · `src/routes/comparendos/comparendos.test.ts` (8).

### ✅ Alertas (`/alertas`)
- **Sin backend nuevo:** consolida `autoApi.alertas` (vencimientos SOAT/tecno-mecánica/extintor/
  batería), `mantenimientoApi.alertasKm`, rentas activas por vencer (retorno ≤3 días) y comparendos
  pendientes. Filtro "solo críticas" + refrescar.
- **Tests:** `src/routes/alertas/alertas.test.ts` (4).

### ✅ Calendario (`/calendario`)
- **Sin backend nuevo:** `utils/calendario.ts` (funciones puras: `celdasDelMes` con semana
  iniciando lunes, `rangoCubreDia`, `detectarSolapamientos`). Página mensual con chips de renta
  (azul) / reserva (ámbar), conflicto en rojo, panel de detalle por día.
- **Tests:** `src/lib/utils/calendario.test.ts` (7) · `src/routes/calendario/calendario.test.ts` (5).

### ✅ Informes (`/informes`)
- **Backend:** `repositories/informe.rs` (sumas por **rango de fechas** `inicio`/`fin` con
  comparación directa en fechas + consultas por placa para utilidad), `services/informe.rs`
  (balance = pagos + abonos reservas − gastos − mantenimiento − comparendos; **utilidad por
  vehículo** = ingresos − costos por placa, ordenada desc), `commands/informe.rs`
  (`informe_mensual` con `fecha_inicio`/`fecha_fin` requeridos — ya no es por mes/año).
- **Frontend:** `api.ts` (`informeApi.mensual(sessionId, fechaInicio, fechaFin)`), `+page.svelte`
  (selector de **rango de fechas** con Fecha inicio/Fecha fin, tarjetas de balance, desglose por
  categoría con barras, tabla de rentas del rango, **tabla de utilidad por vehículo** con barras
  de rentabilidad y conteo rentables/en pérdida, **Imprimir/PDF** y **exportar `.xlsx` real**
  con SheetJS vía `utils/informeExcel.ts` — título/periodo, encabezados con color, formato de
  moneda `#,##0`, anchos de columna y totales en negrita).
- **Tests:** `tests/informes_integration.rs` (3, incl. utilidad = ingresos − costos y orden desc)
  · `src/routes/informes/informes.test.ts` (6, incl. que envía las fechas seleccionadas al backend).
- **Nota (semántica del reporte):** los abonos de reserva cuentan como ingreso y, si además se
  registran como pago de la renta resultante, se cuentan dos veces (coherente con el balance;
  el esquema no permite evitarlo). Pagos se imputan por su fecha; abonos, por la fecha de
  recogida de la reserva.

### ✅ Reservas — impresión (`OrdenReserva.svelte`)
- **Frontend:** `+page.svelte` de reservas incluye botón imprimir que abre modal con
  `reports/OrdenReserva.svelte` (orden **Carta** con itinerario, tarifas, saldo pendiente,
  firmas). Misma mecánica de impresión que Rentas (`imprimirDocumento()` + `.print-area`).
- **Tests:** `src/routes/reservas/reservas.test.ts` (2) — listado con estado y estado vacío.

### ✅ Contrato de renta (`ContratoRenta.svelte`)
- **Frontend:** `reports/ContratoRenta.svelte` — **documento independiente** de la orden de
  renta, en **papel Carta/Letter**. Texto legal real tomado de `Contrato_Dinamo.docx` (fuente
  de verdad): 14 cláusulas completas (objeto, estado del vehículo, pagos y garantías, plazo,
  obligaciones, compromisoria, penal, terminación, responsabilidad, lucro cesante, factura,
  daños a terceros, cobertura de póliza, fotomultas) + tabla del vehículo + póliza de lucro
  cesante con casillas + firmas ARRENDADOR/ARRENDATARIO + pie con datos de contacto. Se abre
  desde el botón «Ver contrato (Carta)» del modal de impresión de rentas.

### ✅ Impresión unificada (`utils/imprimir.ts` + `app.css`)
- **Bug corregido:** dos `.print-area` anidados (orden + contrato) con `position: fixed` en
  `@media print` → el contrato interno ganaba y solo se imprimía el contrato (y en 1 página
  recortada).
- **Solución:** `imprimirDocumento()` clona el `.print-area` a un contenedor raíz del `<body>`
  (`.print-clone`) fuera de la cadena de overflow del Modal, activa `body.printing-clone` y
  llama a `window.print()`. El clon fluye en posición estática → el navegador pagina de forma
  natural (contrato = 4 páginas Carta, orden = 2). Espera `document.fonts.ready` e imágenes
  antes de imprimir y limpia con `afterprint`.
- **Bug corregido en revisión (página 1 en blanco):** el clon se insertaba con `appendChild`
  (al final del `<body>`), y el layout de la app (sidebar + main) oculto con
  `visibility: hidden` —que **sí ocupa espacio**— consumía la primera hoja del PDF, dejándola
  en blanco y desplazando el documento a la página 2. Ahora el clon se inserta con
  `prepend` (primer hijo) y el CSS oculta el resto del layout con `display: none`
  (`body.printing-clone > *:not(.print-clone)`), no solo con `visibility`.
- **CSS:** `@page { size: letter }` (Carta, margen 0) + reset de contenedores con scroll propio
  (`max-height: none; overflow: visible`) para que tablas largas (p. ej. informes) no se
  recorten. Páginas migradas: rentas, reservas, comparendos, informes.

---

## 3. Pendiente / mejoras sugeridas

- [ ] **Configurar `business.impuesto_porcentaje`** en el `config.ini` real de producción (dev usa 19).
- [ ] **Auditar índices** de `mantenimiento_vehiculos` y `informes` para los filtros por rango de
      fechas (pagos.fecha, reservas.fecha_recogida, gastos.fecha, comparendos.fecha_infraccion).
- [ ] **Revisión visual en Tauri**: la **orden de reserva, notificación de comparendo y orden
      de renta + contrato** ya se revisaron en navegador (dev server + mock de Tauri) con
      capturas en `static/preview-shots/*.png` y audit de layout (0 desbordes, 0 imágenes rotas).
      Además se verificó la **impresión PDF real** con Chrome headless: orden de renta en
      **Carta 612×792 (2 páginas)** y contrato en **Carta (4 páginas)** con el texto legal
      completo, revisado página por página (render a 120 dpi en `static/preview-shots/`:
      `contrato-real-pag1..4.png`) sin desbordes, texto cortado ni imágenes rotas.
      Quedan pendientes el **modal de inspección** de rentas y el **calendario**.

---

## 4. Convenciones a respetar

1. **Patrón de módulo:** `repositories/X.rs` (queries Firebird con `CAST` a VARCHAR para montos/fechas)
   → `services/X.rs` (validación `validate_no_xss`, montos `Decimal`, estados en config) →
   `commands/X.rs` (thin wrapper: `require_session` → `conn` → servicio → `to_payload`).
   Registrar módulo en los tres `mod.rs` y comandos en `lib.rs` (`generate_handler!`).
2. **Tipos en tuplas:** `SelectCols` constantes alineadas con la tupla `Row` (comentario
   "mantener alineada"). Nunca tuplas >26 (FROMROW) ni >15 en `IntoParams` directo → usar `params!`.
3. **Montos como String** (`DECIMAL` → `CAST(... AS VARCHAR(12))`); el frontend formatea con
   `formatCOP`. El backend es la fuente de verdad de totales.
4. **RBAC:** comandos con `require_session` (todos) y `require_usuario_admin` (usuarios/auditoría).
   Guards frontend centralizados en `utils/guards.ts` (`validarSesion`, `guardSesion`, `haySesion`, `guardRole`).
5. **PII:** clientes se descifran en el backend con `db_encryption_key` (`pii_key` en caliente).
6. **Errores:** `AppError` → `to_payload()` → `{kind, message, detail}`; el frontend lanza `ApiError`.
7. **Tests:** integración Rust contra `data/dinamo_rent_v3.fdb` (serial, autos/clientes reales de
   solo lectura, limpieza de registros temporales). Frontend con mock de Tauri
   (`src/test/tauri.ts` + `register`), `session.setSession` en `beforeEach`.
