# Handsoff — Dinamo Rent ERP (Tauri + SvelteKit + Firebird)

> Última actualización: **2026-08-11** · Estado: **todos los módulos operativos, validación verde**

> **Herramientas de monitoreo y test del Agente SIMIT (11-08):** tras la validación E2E, el
> monitoreo del portal quedó consolidado en `scripts/`:
> - **`check-simit.mjs`** (`npm run check:simit`) — ante un **401 sin token** ya no declara el
>   portal caído: corre una **sonda E2E con token real** (PoW de UNA solución, Fase 1) y solo
>   reporta operativo si la consulta responde 200. El flujo quedó cubierto por el test
>   **`scripts/test-check-simit.mjs`** (`npm run test:check-simit`): servidor local que simula el
>   gateway selectivo, verifica 401→sonda E2E→operativo y el token `[a]`, y también el caso
>   negativo (401 con token real → `micro_caido`). Opción nueva **`--multas`**: incluye
>   `microservicio.e2e` (totalPendiente + detalle por multa) en el JSON.
> - **`watch-simit.mjs`** (`npm run watch:simit`) — vigilante periódico que pasa `--multas`,
>   persiste el último total pendiente en `data/simit_watch/ultimo_total.json` y **alerta cuando
>   el total pendiente de la flota cambia** entre corridas (consola + log + `cambioTotal` en el
>   JSON final).
> - **Verificación del jar en el agente real** — convertida en test de integración `#[ignore]`
>   `jar_portal_real_captura_cookies_adc` (`services/simit.rs`): corre siembra + captcha contra el
>   portal real y falla si el jar no captura `aiovg_rand_seed` + `ADC_CONN_*` + `ADC_REQ_*`
>   (en verde: mismas cookies que la sonda Node).
> Suite al cierre: `cargo test --lib` **37 passed + 1 ignored** · `test-check-simit` ✅ ambos
> escenarios. Detalle y referencias en `SIMIT_MIGRACION_PYTHON_RUST.md` §4 y §6.

> **E2E del Agente SIMIT validada contra el portal real (11-08):** el microservicio de consulta
> volvió a estar operativo y se validó el flujo completo sobre la BD dev con un binario de
> desarrollo nuevo (`src-tauri/src/bin/sync_dev.rs`, `cargo run --features dev --bin sync_dev` —
> mismo camino que «Sincronizar ahora», sin Tauri). Resultado: **21/21 placas consultadas, 30
> registros encontrados, 27 comparendos reales insertados** (reporte totalPendiente $21.225.939
> sobre los 30 encontrados; **deuda real en BD tras dedup: $19.212.789** — ver
> `SIMIT_MIGRACION_PYTHON_RUST.md` §4 «Sonda vs BD dev»), 0 errores; **2ª corrida: 0 insertados /
> 30 duplicados** (dedup confirmado); **reporte HTML** en
> `data/informes_simit/simit_*.html` y **export Excel** verificado con
> `scripts/verificar-excel-simit.mjs` (mismo mapeo que el botón de `/comparendos`: 30 filas 1:1,
> total pendiente correcto). 🐛 **2 bugs reales encontrados y corregidos en `simit.rs`**: (1) el
> SIMIT envía `fechaComparendo` en **DD/MM/YYYY** y el mapeo solo aceptaba ISO → **las 30 multas
> reales se descartaban** (primera corrida con el código viejo: 0 insertados); `parsear_fecha_hora`
> ahora normaliza a ISO (tests nuevos). (2) una fecha malformada **abortaba toda la
> sincronización** porque `ya_existe` → `existe_duplicado` llama `parse_fecha` antes del
> pre-check; el pre-check se movió ANTES (intención original: omitir el registro, no abortar).
> 🗑️ **Sonda temporal eliminada** (`scripts/simit-sonda-fase1.mjs`): cumplió su ciclo y quedó
> reemplazada por el propio `scripts/check-simit.mjs`, actualizado el 11-08 — ante un 401 sin
> token corre la sonda E2E con token real y usa el **token de una sola solución** (el gateway
> rechaza el array completo con 401 y acepta `[:1]` con 200; las cookies ADC ya no son el gate de
> hoy — el jar queda como hardening). Verificado en vivo: `check-simit` → **SIMIT operativo**
> (exit 0). Tests: simit 14/14 · `comparendos_integration` 4/4 · `informeExcel` 6/6. Detalle en
> `SIMIT_MIGRACION_PYTHON_RUST.md` §4.

> **Fase 1 del Agente SIMIT (10-08, noche):** se comparó el contrato HTTP del agente Rust contra el
> servicio Python de referencia (**API-Runt-simit**, `D:\Proyectos\API-Runt-simit-main`, con
> `debug_response.json` exitoso) — documento completo: **`SIMIT_MIGRACION_PYTHON_RUST.md`**.
> Hallazgo principal: el agente ureq **no tenía jar de cookies** (feature `cookies` de ureq 2.12.1
> NO está en los defaults; el proyecto usaba solo `json`) y el gateway ADC exige la cookie de
> sesión → el 401 «No se puede definir la política de seguridad». Implementado en `simit.rs`: jar
> persistente (`cookie_store`), **siembra de sesión** con `GET https://www.fcm.org.co/` (una vez
> por proceso + re-siembra ante 401), **token PoW de una sola solución** (como el Python `[:1]`) y
> **reintento con token fresco tras 401** (el token parece de un solo uso). Validación: `cargo
> check --lib` 0/0 · `cargo test --lib` **37/37** (5 tests nuevos: jar compartido entre peticiones,
> siembra deja la cookie en el jar, 401 clasificado como `Unauthorized` con body, recorte del
> token) · clippy sin warnings nuevos. E2E sigue pendiente solo por disponibilidad del micro.
> ⚠️ racha larga del `os error 32` de Defender durante el build (8+ fallos seguidos, incluso con
> target fresco — inusual; ver README §4).

> **Revisión visual en la app real (10-08, noche):** se lanzó el binario Tauri (debug) con
> WebView2 remote-debugging y se revisaron los pendientes vía CDP (script
> `scripts/review-pendientes.mjs`, capturas en `static/preview-shots/revision-*.png`):
> (1) ✅ **modal de inspección de rentas** (Salida y Entrada) abre y conmuta bien (renta #18);
> (2) ✅ **calendario** carga sin errores; (3) ✅ **panel Agente SIMIT** operativo con estado real
> (última corrida 10-08 08:46, reporte HTML generado, errores 401 por placa — portal caído,
> esperado). Audit: 0 desbordes de layout, 0 imágenes rotas, 0 errores de consola.
> 🐛 **bug de zona horaria encontrado y corregido**: `formatDate`/`formatDateTime` usaban
> `new Date('YYYY-MM-DD')`, que interpreta la fecha como medianoche UTC y en Bogotá (UTC-5)
> retrocedía un día («9 de ago» en vez de «10 de ago») → afectaba al panel SIMIT y a cualquier
> fecha ISO de la app. Fix: helper `parseDate` en `src/lib/utils/format.ts` que construye las
> fechas sin hora como **hora local** (tests: +2 de regresión en `format.test.ts`, invariante
> independiente de la zona horaria del entorno; suite 192/192, `check` 0/0, `lint` 0).

> **Reintento SIMIT (10-08, tarde):** se recreó el test temporal y se re-verificó con el código ya
> corregido (headers Origin/Referer). El **microservicio de consulta sigue caído**: captcha PoW OK,
> consulta 401 de gateway en todas las placas, página principal 503, `simit2.fcm.org.co` 503 y
> `smbackoffice.fcm.org.co` inalcanzable → toda la infra SIMIT está caída salvo el captcha. BONUS:
> el preflight OPTIONS del endpoint expone un **WADL** (Jersey 2.32) que confirma el contrato del
> microservicio: recurso único `estadocuenta/consulta` POST JSON (método `findEstadoCuenta`), sin
> parámetros de auth en la API → el 401 viene de un gateway de seguridad EXTERNO que no logra
> «definir la política de seguridad» (coherente con el 503 general). La E2E sigue pendiente solo
> por disponibilidad del servidor.

> **Validación SIMIT contra el portal real (10-08, mañana):** por primera vez se probó el flujo HTTP
> real (el captcha qxcaptcha volvió a responder; el microservicio de consulta NO). Resultados:
> (1) ✅ el **captcha PoW se resuelve** contra el servidor real — el riesgo de **TLS fingerprinting NO
> se materializó** (probado con ureq/rustls, Node/undici y curl); (2) ✅ el **formato del token es 1:1
> con la referencia** manavarrp/SimitConsulta (`HashHelper.cs` + `captcha.ts`: orden
> question/time/nonce, array JSON, epoch UTC — verificado con test temporal y replicado en Node);
> (3) 🐛 **bug de contrato encontrado y corregido**: faltaban los headers `Origin: https://www.fcm.org.co`
> y `Referer: https://www.fcm.org.co/` (+ `Accept`, `Accept-Language`) que la referencia documenta como
> obligatorios («Sin Origin y Referer el servidor rechaza la petición») → añadidos en
> `services/simit.rs` (`con_headers_browser()`, aplicado a captcha y consulta; 9/9 tests unitarios
> verdes); (4) ❌ el **microservicio de consulta sigue caído**: página principal `503
> Server-unavailable!` y el endpoint de consulta responde `401 {"codigo":5,"descripcion":"Autenticación
> fallida: Acceso denegado. No se puede definir la política de seguridad."}` a **CUALQUIER** petición
> (con token válido de Rust, de Node/undici, o sin token) → fallo del gateway/auth, NO del contrato.
> La verificación end-to-end (insertar/dedup, sync de estado, reporte HTML, Excel) sigue pendiente
> hasta que el microservicio vuelva. Nota del gateway: expone auth por headers `token`/`ticket`
> (CORS `Access-Control-Allow-Headers`) y setea cookies ADC (`ADC_CONN_*`/`ADC_REQ_*`) — posible
> requisito adicional cuando el servicio esté arriba. Ver §3 (tarea PRIMERO en curso).

> **Agente SIMIT (09-08, tarde):** nuevo agente que consulta los comparendos/multas de toda la
> flota en el portal del SIMIT cada 2 h (configurable en `[simit]` de `config.ini`) mientras la
> app esté abierta. Resuelve el **captcha Proof-of-Work** de `qxcaptcha.fcm.org.co` (nonces
> primos cuyo SHA256 de `{"question":q,"time":t,"nonce":n}` empiece con `0000`, ×dificultad) y
> consulta el microservicio interno de `consultasimit.fcm.org.co` (contrato reconstruido por
> ingeniería inversa, ver referencia `manavarrp/SimitConsulta`). Los comparendos **nuevos** se
> insertan como Pendiente en `comparendos` deduplicando por el **número oficial** (columna nueva
> `numero_comparendo`, migración **0015**) o placa+fecha+monto; si el SIMIT reporta pagado un
> comparendo ya registrado, la BD **converge a Pagado**; al final genera un **reporte HTML
> imprimible** en `data/informes_simit/simit_*.html` y emite el evento `simit-sync-complete`.
> UI: panel «Agente SIMIT» en `/comparendos` (estado, última corrida, «Sincronizar ahora» y
> «Descargar Excel»). Detalle en §2. **Advertencia para mañana: el portal SIMIT estuvo caído
> («Server-unavailable») durante todo el desarrollo → el flujo HTTP real NO se probó de punta
> a punta**; el contrato se implementó fiel al código de referencia y hay riesgo de TLS
> fingerprinting en el captcha (ver §3, primera tarea pendiente).

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

> **Consolidación de índices y migraciones (09-08):** serie de migraciones **0001-0014**
> (0001-0004 idempotentes, 0005-0009 auto-reparables, 0010-0013 de consolidación de índices:
> dedup `IX_`/`IDX_`, redundantes con las FKs, subsumidos por compuestos y el último
> `IX_AUDITORIA_USUARIO`, y **0014 de limpieza de tablas residuales de test** con guard de
> esquema exacto + `RDB$RELATION_TYPE = 0`, ver §5.2). Resultado verificado sobre la BD dev: **0 índices duplicados, 0
> subsumidos** (esquema canónico en README §Migraciones), `SET PLAN ON` sin full scans nuevos
> (placa → FK `RDB$FOREIGN38/34/36`; estado → compuestos) y `gstat -i` con **−17% de páginas de
> índice**. Además, el `os error 32` de cargo quedó resuelto con exclusiones de Windows
> (Defender + índice; ver README) y el ciclo completo `cargo test` + `npm run tauri dev` corre
> verde. Ciclo de trabajo de migraciones en la sección 5.

---

## 1. Estado general

Proyecto de renta de vehículos: frontend **SvelteKit 5** (`src/`), backend **Tauri/Rust**
(`src-tauri/`) con **Firebird 5** embebido y pool `r2d2` (`rsfbclient`).

| Validación | Resultado |
|---|---|
| Vitest (frontend) | **190/190** en 25 archivos |
| `npm run check` (svelte-check) | **0 errores / 0 warnings** |
| `npm run build` (vite) | ✅ |
| `cargo test` (Rust) | ✅ unit (32, incl. 9 del Agente SIMIT) + integraciones por módulo (comparendos ahora 4) |
| `cargo check --tests` | ✅ 0 errores |
| `cargo clippy --lib` | ✅ código nuevo limpio; quedan 6 warnings pre-existentes (migrations.rs ×2, informe.rs ×1, renta.rs ×2, services/renta.rs ×1) |
| `npm run lint` | ✅ **0 problemas** (config corregida el 10-08 — ver §3) |

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
- **Tests:** `tests/comparendos_integration.rs` (4) · `src/routes/comparendos/comparendos.test.ts` (8).

### ✅ Agente SIMIT (comparendos automáticos por placa)
- **Backend:** `services/simit.rs` —
  - `resolver_captcha()`: `POST https://qxcaptcha.fcm.org.co/api.php` (form `endpoint=question`)
    → `{error, data:{question, recommended_difficulty}}`; PoW: `difficulty` nonces primos cuyo
    SHA256 hex de `{"question":q,"time":t,"nonce":n}` empiece con `0000` → token = array JSON
    de los objetos de verificación. Dependencia HTTP nueva: **`ureq` 2.12** (features `json`).
  - `consultar_placa(placa)`: `POST https://consultasimit.fcm.org.co/simit/microservices/
    estado-cuenta-simit/estadocuenta/consulta` con `{"filtro":placa,"reCaptchaDTO":{"response":
    token,"consumidor":"1"}}` → `multas[]` (comparendo, numeroComparendo, valorPagar,
    estadoComparendo, fechaComparendo, organismoTransito, infracciones[]). Se conservan
    comparendos y multas; estado mapeado (PAGA*/COBR* → «Pagado»; resto → «Pendiente»).
  - `sincronizar()`: lista `AutoRepository::placas_activas` (**excluye Vendido/Baja**), consulta
    cada placa con `simit.polite_delay_ms` (2,5 s) de espera, **deduplica** por
    `numero_comparendo` (fallback placa+fecha+monto, solo registros activos) y **sincroniza
    estado**: si el SIMIT reporta pagado un número ya registrado → `marcar_pagado_por_numero`.
    Las observaciones llevan trazabilidad (`Importado SIMIT (Comparendo|Multa) · N° … · organismo · código · descripción`).
  - Reporte: **HTML imprimible** en `data_dir/simit_report_dir/simit_AAAAMMDD_HHMM.html` con
    tarjetas de resumen, tabla (🆕 = nuevo en la BD) y errores por placa. `total_pendiente` =
    suma de **todos** los registros pendientes encontrados (no solo los nuevos).
  - Scheduler: hilo de fondo lanzado en `setup()` de `lib.rs` — la **primera corrida** espera
    `simit.start_delay_minutes` (default **10 min**; 0 = inmediata) para no competir con el
    arranque de la app, y después corre cada `simit.interval_hours` (2 h). **Chequeo DNS previo**
    a cada corrida (`portal_simit_accesible()`, espejo de `scripts/check-simit.mjs`): si los
    subdominios del portal no resuelven (SIMIT caído), la corrida se omite al instante y la
    siguiente consulta es en el ciclo normal (evita timeouts de 30 s por placa). Reintento a los
    60 s solo si falla a nivel de BD; errores por placa (SIMIT caído) se registran y la
    siguiente corrida es en el ciclo normal. Emite `simit-sync-complete` con el
    `ResultadoSincronizacion` serializable.
  - Concurrencia: claim **atómico** (`AtomicBool::compare_exchange`, `claimar()/liberar()`) para
    que la corrida programada y la manual nunca se solapen.
- **Comandos:** `commands/simit.rs` — `simit_sync_status` (estado en memoria: habilitado,
  intervalHours, ejecutando, última sincronización + resultado/errores) y `simit_sync_now`
  (async + `spawn_blocking`, las operaciones de BD son síncronas). Ambos pasan por `run_sync`,
  que incluye el **fast-fail DNS** (`portal_simit_accesible`): si el portal no resuelve,
  «Sincronizar ahora» falla al instante con un mensaje claro en vez de gastar timeouts de 30 s
  por placa. Estado manejado por Tauri vía `EstadoAgenteSimitManaged` (no amplía `AppState` →
  no toca los tests de integración).
- **Migración `0015_comparendo_numero_simit.sql`:** `comparendos.numero_comparendo VARCHAR(30)`
  + índice `IX_COMPARENDOS_NUMERO` (guards RDB$, idempotente). **YA aplicada a la BD dev**
  (vía test temporal `aplicar_migraciones_dev_temporal.rs`, eliminado después).
- **Config `[simit]`** (defaults en `core/config.rs` + `data/config.ini.example`): `enabled=true`,
  `interval_hours=2`, `polite_delay_ms=2500`, `report_dir=informes_simit`,
  `start_delay_minutes=10` (retraso de la primera corrida tras el arranque; 0 = inmediata).
- **Frontend:** `api.ts` (`simitApi` + tipos `RegistroSimit`/`ResultadoSincronizacion`/`InfoAgenteSimit`,
  con `startDelayMinutes` y `proximaSincronizacion`), `+page.svelte` de comparendos: panel con
  estado, última y **próxima corrida** (primera corrida tras `startDelayMinutes`), botón
  «Sincronizar ahora», «Descargar Excel» (reusa `exceljs`, columna «Nuevo»), escucha de `simit-sync-complete` con
  cleanup (`UnlistenFn`), y el formulario conserva `numeroComparendo` al editar (no rompe la deduplicación).
- **Tests:** 9 unit en `services/simit.rs` (sha256, primos, formato del JSON de verificación,
  PoW → nonces válidos crecientes, mapeo de estados, parseo de fechas/horas incl. ISO con `T`,
  mapeo de la respuesta completa, observaciones, escape HTML) · `comparendos_integration.rs` +1
  (round-trip de `numero_comparendo`, dedup por número y placa+fecha+monto, sync de estado,
  soft-delete excluye del dedup) · frontend: factories de `comparendos.test.ts` y
  `alertas.test.ts` actualizados con `numeroComparendo`.
- **Validación real (10-08 al 11-08, ver notas al inicio):** el 10-08 se probó el flujo HTTP por
  primera vez — ✅ captcha PoW aceptado (sin TLS fingerprinting), ✅ token 1:1 con la referencia,
  🐛 headers `Origin`/`Referer` añadidos (`con_headers_browser()`), ❌ micro caído ese día (401 a
  toda petición + 503). El **11-08 el micro volvió** y la E2E quedó **validada**: 21/21 placas,
  27 comparendos reales insertados (dedup en 2ª corrida: 0 nuevos / 30 duplicados), reporte HTML
  + Excel verificados; 🐛 **2 bugs corregidos** (fechas `fechaComparendo` en **DD/MM/YYYY** — las
  multas se descartaban — y orden del pre-check de fecha que abortaba la sincronización). Ver §3.

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

- [x] **PRIMERO — Probar el Agente SIMIT contra el portal real.** *HECHO el 11-08 — E2E validada
      contra el portal real: 27 comparendos insertados (BD dev), dedup en 2ª corrida, reporte HTML +
      Excel verificados, 2 bugs corregidos (fechas DD/MM/YYYY y orden del pre-check) — resumen en la
      nota de portada y detalle en `SIMIT_MIGRACION_PYTHON_RUST.md` §4.      Herramientas:
      `cargo run --features dev --bin sync_dev` (sincronización E2E sin Tauri, dump JSON en
      `data/simit_watch/sync_result.json`), `node scripts/verificar-excel-simit.mjs` (valida el
      export Excel contra el resultado) y el monitoreo/test de hoy (`check-simit` con sonda E2E y
      `--multas`, `watch-simit` con alerta de total, `test-check-simit`, test `#[ignore]` del jar) —
      resumen en la nota de portada.* Historial: el 10-08 se probó el flujo HTTP real por
      primera vez (el captcha qxcaptcha volvió a estar arriba):
      (1) ✅ **captcha PoW aceptado** — el riesgo de TLS fingerprinting NO se materializó (probado
      con ureq/rustls, Node/undici y curl); (2) ✅ **token 1:1 con la referencia** (HashHelper.cs /
      captcha.ts de manavarrp/SimitConsulta) y **bug de contrato corregido** (faltaban
      `Origin`/`Referer`/`Accept`/`Accept-Language` → `con_headers_browser()` en services/simit.rs);
      (3) ❌ el 10-08 el microservicio seguía caído (401 de gateway a CUALQUIER petición + 503 en la
      página principal; WADL del endpoint confirma el contrato POST JSON sin auth en la API). El
      11-08 el micro volvió: el 401 pasó a distinguir token válido (200) de inválido/sin token
      (401), y la sonda temporal `simit-sonda-fase1.mjs` (eliminada) + `check-simit.mjs`
      (actualizado: 401 sin token → sonda E2E con token real de UNA solución) confirmaron el flujo.
      Queda pendiente solo el repaso visual del panel del Agente SIMIT en la app real.
- [x] **Arreglar `npm run lint` (hecho 10-08):** `eslint.config.js` corregido — (1) el plugin
      `@typescript-eslint` se declara en TODOS los bloques que usan sus reglas (custom y tests;
      flat config scoping — causa del «could not find plugin»); (2) globals de navegador vía el
      paquete `globals` (añadido a devDependencies) para `.ts`/`.svelte` y globals node para
      `src/test/**`; (3) `no-undef` off en TS/Svelte (svelte-check cubre la detección; sin eso,
      las runes `$state`/`$effect` y el genérico `T` de DataTable daban no-undef); (4)
      `no-unused-vars` del core off (el de `@typescript-eslint` ya lo cubre respetando `^_`);
      (5) `.eslintignore` eliminado (deprecado; patrones ya en `ignores`). Deuda limpiada:
      vars de suscripción de `$effect` renombradas a `_est`/`_plac`/… (convención ya usada en
      rentas), `catch (e)` → `catch {}`, y muertos eliminados (import `onMount` de Modal,
      props `cliente`/`auto` de OrdenRenta + su caller en rentas, deriveds sin uso en autos,
      arg `r` de informeExcel). Validación: `npm run lint` 0 problemas, `npm run check` 0/0,
      `npm run test` 190/190, `npm run build` ✅ — el pre-commit (`bun run lint`) ya no bloquea.
- [x] **Configurar `business.impuesto_porcentaje`** en el `config.ini` real de producción (dev usa 19).
      *Hecho (10-08): el config real (`%APPDATA%\com.corjar.dinamorent\config.ini`) ya trae
      `impuesto_porcentaje = 19` en `[business]` (auto-generado con los defaults) y la app lo lee al
      arrancar. Para CAMBIAR la tasa en producción: editar esa clave en `[business]` del config.ini y
      reiniciar la app (sin rebuild; `AppConfig::save()` preserva la clave — no la pisa). Se documentó
      también en `data/config.ini.example`.*
- [x] **Auditar índices** de `mantenimiento_vehiculos` y `informes` para los filtros por rango de
      fechas (pagos.fecha, reservas.fecha_recogida, gastos.fecha, comparendos.fecha_infraccion).
      *Hecho en 0010-0013 (dedup + consolidación; los `IDX_*_FECHA` de 0002 se conservan porque no
      están subsumidos) — ver sección 5 y README §Migraciones.*
- [ ] **Revisión visual en Tauri**: la **orden de reserva, notificación de comparendo y orden
      de renta + contrato** ya se revisaron en navegador (dev server + mock de Tauri) con
      capturas en `static/preview-shots/*.png` y audit de layout (0 desbordes, 0 imágenes rotas).
      Además se verificó la **impresión PDF real** con Chrome headless: orden de renta en
      **Carta 612×792 (2 páginas)** y contrato en **Carta (4 páginas)** con el texto legal
      completo, revisado página por página (render a 120 dpi en `static/preview-shots/`:
      `contrato-real-pag1..4.png`) sin desbordes, texto cortado ni imágenes rotas.
      Quedan pendientes el **modal de inspección** de rentas, el **calendario** y el **panel del
      Agente SIMIT** en la app real (ver primera tarea de §3).

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

---

## 5. Migraciones de base de datos — ciclo de trabajo

El esquema Firebird se gestiona con un runner propio (`src-tauri/src/core/migrations.rs`) que
aplica en orden los scripts de `src-tauri/migrations/` no ejecutados y registra cada versión en
`schema_migrations`. Serie actual: **0001-0015** (propósito de cada una y esquema canónico de
índices en el README §Migraciones). La **0015** (columna `comparendos.numero_comparendo` +
índice) da soporte a la deduplicación del Agente SIMIT; ya está aplicada a la BD dev.

### 5.1 Cómo añadir una migración nueva (000N)

1. Crear `src-tauri/migrations/000N_descripcion.sql` — número siguiente con padding a 4 dígitos
   y sufijo descriptivo (`0004_no_contrato_anual.sql`, `0013_consolidar_indices_auditoria.sql`).
2. **Nunca DDL "pelado"**: cada objeto va dentro de un `EXECUTE BLOCK` con guard contra el
   catálogo (patrón en 5.2). Es obligatorio por el diseño del runner: cada sentencia se ejecuta
   en **autocommit** y, si una migración falla a mitad, su versión **NO se registra** y el
   siguiente arranque la reintenta — los guards omiten lo ya creado y crean lo que falta, así que
   las instalaciones a medias se auto-reparan solas.
3. Si la migración **deja de crear** un objeto que otra creaba (p. ej. 0012 quitó de 0001 los
   índices que después dropea), edita también la migración creadora para que las instalaciones
   nuevas nunca lo creen — y actualiza los tests (5.3).
4. Encabezado de comentario: propósito, tablas/columnas afectadas y por qué es idempotente.
5. Validar con `cargo test --test migraciones_integration` (5.3) y, al final, arranque real
   (`npm run tauri dev`) para que se aplique a la BD dev.

### 5.2 Patrón EXECUTE BLOCK + guard

```sql
EXECUTE BLOCK
AS
BEGIN
  IF (NOT EXISTS(SELECT 1 FROM RDB$RELATION_FIELDS
                 WHERE RDB$RELATION_NAME = 'X' AND RDB$FIELD_NAME = 'COL')) THEN
    EXECUTE STATEMENT 'ALTER TABLE x ADD col ...';
END;
```

Guards por objeto (catálogo Firebird):

| Objeto | Catálogo |
|---|---|
| Tabla (crear) | `RDB$RELATIONS.RDB$RELATION_NAME` |
| Tabla (DROP residual de test) | `RDB$RELATIONS` + `RDB$SYSTEM_FLAG = 0` + `RDB$RELATION_TYPE = 0` + esquema exacto vía `RDB$RELATION_FIELDS` |
| Columna | `RDB$RELATION_FIELDS` (relation+field); NOT NULL → `RDB$NULL_FLAG = 1` |
| Índice | `RDB$INDICES.RDB$INDEX_NAME` |
| Índice por columnas | `RDB$INDEX_SEGMENTS` (`FIELD_NAME` + `FIELD_POSITION`) |
| Constraint CHECK | `RDB$RELATION_CONSTRAINTS.RDB$CONSTRAINT_NAME` |
| Generator | `RDB$GENERATORS.RDB$GENERATOR_NAME` |
| Trigger | `RECREATE TRIGGER` (crea o recrea, sin guard) |

**DROP condicional (consolidación de índices, patrón 0011-0013):** solo se elimina un índice si
queda OTRO índice no-sistema de la misma tabla que cubra la columna como **primer segmento**
(`RDB$FIELD_POSITION = 0`) y esté **activo** (`COALESCE(RDB$INDEX_INACTIVE, 0) = 0`) — nunca se
deja una búsqueda sin índice.

**DROP de tabla residual (patrón 0014):** solo se elimina una tabla de test si es **tabla real
no-sistema** (`RDB$SYSTEM_FLAG = 0` **y** `RDB$RELATION_TYPE = 0` — las vistas también viven en
`RDB$RELATIONS` y un `DROP TABLE` sobre una fallaría) y coincide **exactamente** con el esquema
residual esperado (conteo de columnas + `EXISTS` por nombre, recuperado del catálogo original
vía git HEAD). Una tabla futura con el mismo nombre pero otro esquema **NO** se dropea
(seguridad ante colisión; el tradeoff residual — nombre + esquema idénticos — está documentado
en la cabecera de 0014).

Reglas de oro:
- Comillas simples de los literales **duplicadas** (`''...''`) por ir dentro del literal de
  `EXECUTE STATEMENT`.
- **NUNCA `--` dentro de un literal**: `split_sql_statements` recorta los comentarios de cada
  línea sin mirar si están dentro de una cadena → truncaría la sentencia en silencio.
- Triggers con varias sentencias no caben (el splitter respeta bloques BEGIN...END pero no `;`
  internos): cuerpo de UNA sentencia (ver 0007).
- **`isql` NO ejecuta `EXECUTE BLOCK`** (error -104, limitación de la herramienta): validar
  siempre por el runner, nunca por isql.

### 5.3 Validación

```bash
cd src-tauri && cargo test --test migraciones_integration
```

Los **8 tests** corren contra una **copia temporal** de la BD dev (la real nunca se toca) y contra
una BD nueva vacía: versiones registradas (incluida la nueva) + 2ª ejecución no-op (idempotencia),
instalación fresca desde cero, auto-reparación de instalaciones a medias (crash en 0001/0003-0004),
`has_initial_schema` (exige las 4 tablas núcleo + `pagos`) y la **seguridad ante colisión de
0014** (tablas creadas antes de migrar: el esquema residual exacto se dropea, una tabla con el
mismo nombre pero esquema distinto sobrevive).

Al añadir una migración hay que actualizar en `migraciones_integration.rs`: el conteo
`versiones_aplicadas(&pool).len()` (actual: **15**), las listas de versiones de los tests de
registro y las assertions de columnas/índices afectados (copia dev y BD fresca). Después, la
suite completa (`cargo test`) y `npm run tauri dev` (aplica la migración a la BD dev real en el
arranque; verificar el catálogo al cerrar).

> **Nota 09-08:** la 0015 se aplicó a la BD dev con un test temporal de una sola ejecución
> (`tests/aplicar_migraciones_dev_temporal.rs`, creado, ejecutado con `cargo test --test ...` y
> **eliminado**). Es el atajo equivalente a `npm run tauri dev` cuando solo se necesita migrar.
