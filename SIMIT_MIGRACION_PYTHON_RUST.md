# SIMIT — Contrato Python vs Rust y plan de migración por fases

> Documento de referencia del Agente SIMIT (Handsoff §2 · §3). Compara el contrato HTTP del
> servicio Python de referencia (**API-Runt-simit**, repo `D:\Proyectos\API-Runt-simit-main`,
> autor @Milo443) contra el agente Rust (`src-tauri/src/services/simit.rs`) y detalla el plan de
> migración en fases para resolver el 401 del gateway de consulta.

---

## 1. Contexto y objetivo

- El Agente SIMIT de Dinamo Rent consulta comparendos/multas por placa contra
  `consultasimit.fcm.org.co` (microservicio del FCM) resolviendo el captcha Proof-of-Work de
  `qxcaptcha.fcm.org.co`.
- **Síntoma**: el endpoint de consulta respondía `401 {"codigo":5,"descripcion":"Autenticación
  fallida: Acceso denegado. No se puede definir la política de seguridad."}` a toda petición
  (con token válido de Rust/Node o sin token) mientras el captcha PoW funcionaba. *Ese
  comportamiento era del 10-08 (servicio caído): desde el 11-08 el 401 solo ocurre con token
  inválido/ausente — ver §3 y §4.*
- **Estado al 11-08**: el microservicio **volvió a estar operativo** y el 401 pasó a ser
  **selectivo por token** — un token PoW válido de **una sola solución** responde **200** (la E2E
  insertó 27 comparendos reales, ver §4) y el token ausente/inválido responde **401**
  (comportamiento normal del gateway, no caída). `scripts/check-simit.mjs` confirma el 401 sin
  token corriendo una sonda E2E con token real (resultado 11-08: **SIMIT operativo**, exit 0).
- El repo de referencia contiene **dos** implementaciones SIMIT:
  1. `app/procesos/simit/service.py` — FastAPI + **httpx** (mismo contrato que Rust), con
     evidencia de éxito: `debug_response.json` (UTF-16LE) muestra `simit.exito: true` en 2.62 s.
  2. `test_simit.py` — **Selenium + Chrome headless**: carga la SPA real, deja que el propio sitio
     resuelva y acumule tokens en `sessionStorage.whcQuestions` (cola de 5, confirmado por
     `wehatecaptcha.js`) y consulta con `fetch` nativo de la página (plan B, ver Fase 2).
- **Objetivo de la Fase 1**: replicar en Rust el contrato HTTP que funcionó en Python.

---

## 2. Comparación contrato-a-contrato

| Aspecto | Python (`service.py`) | Rust (`simit.rs`) | Diferencia |
|---|---|---|---|
| Captcha endpoint | `POST qxcaptcha.fcm.org.co/api.php` form `endpoint=question` | idéntico | — |
| Headers captcha | UA, `Origin`, `Referer`, `Accept` | UA, `Origin`, `Referer`, `Accept`, `Accept-Language` | — |
| PoW | nonces primos con SHA256 `0000`, `int(time.time())` (epoch) | idéntico (`resolver_pow`, `Local::now().timestamp()`) | — |
| **Token PoW enviado** | `json.dumps(pow_solutions[:1])` → **1 sola solución** | array completo de `difficulty` soluciones | ⚠️ **Fase 1**: Rust ahora envía 1 |
| **Cookie jar** | `httpx.AsyncClient` persistente + `cookies=captcha_cookies` explícito; mismo client compartido entre SIMIT/RUNT en `consulta_integral` | ureq **sin** feature `cookies` → sin jar | ⚠️ **Fase 1**: feature + `CookieStore` |
| Siembra de sesión del sitio | no hace GET al sitio (depende del jar) | no hacía GET al sitio | **Fase 1**: `GET https://www.fcm.org.co/` |
| Headers de consulta | `Sec-Fetch-*`, `Priority` + UA/Origin/Referer | UA/Origin/Referer (omite `Sec-Fetch-*`) | menor (ver §3, hipótesis 4) |
| Reintentos | 3 intentos, 1.5 s fijo | backoff exponencial + circuit breaker (5 fallos/5 min) | — |
| Detección de 401 | `raise HTTPException(401)` | `ureq::Error::Status(401, resp)` → error genérico | **Fase 1**: `ErrorConsulta::Unauthorized` + re-siembra/reintento |
| Parseo de respuesta | `comparendos` + `resoluciones` + `multas` + `acuerdosPago` | solo `multas` (con flag `comparendo`) | formato distinto según respuesta; no es la causa del 401 |

### 2.1 Hallazgo principal — el jar de cookies no existía en Rust

Verificado en el código fuente de ureq 2.12.1 (registry local):

- Features **default** de ureq 2.12.1: `["tls", "gzip"]` — **`cookies` NO está por defecto**.
  El proyecto declaraba `ureq = { version = "2.12", features = ["json"] }` → todo el manejo de
  cookies estaba **compilado out**.
- Con el feature activo, `AgentBuilder::build()` crea un `CookieTin` (`RwLock<CookieStore>`)
  **persistente por agente** (`agent.rs:299`), las cookies de cada respuesta se guardan en el jar
  (`unit.rs:589` — incluye respuestas con status ≥ 400) y se reenvían automáticamente en la
  siguiente petición al mismo dominio (`unit.rs:325`).
- El gateway ADC del SIMIT emite cookies de sesión (`ADC_CONN_*`/`ADC_REQ_*` — observadas en el
  preflight OPTIONS del endpoint el 10-08, ver Handsoff). Sin ellas el gateway no puede «definir la
  política de seguridad» → 401. *(Hipótesis original; matizada abajo: desde el 11-08 la consulta
  funciona con y sin esas cookies — no eran el gate.)* El `httpx` de Python conservaba esas
  cookies; el ureq de Rust las descartaba.
- **Confirmado contra el portal real (11-08)**: durante la validación E2E el jar de la Fase 1
  capturó las cookies reales del flujo — `aiovg_rand_seed` (del `GET https://www.fcm.org.co/` de
  siembra) y `ADC_CONN_*`/`ADC_REQ_*` (de la respuesta del captcha) — confirmando que el gateway
  **sí las emite** y que el `CookieStore` persistente **las conserva** tal como se diseñó en la
  Fase 1. Quedó convertido en test de integración `#[ignore]` (no corre en la suite normal):
  `jar_portal_real_captura_cookies_adc` en `services/simit.rs`, ejecutable con
  `cargo test --lib jar_portal_real_captura_cookies_adc -- --ignored --nocapture` — falla si
  falta cualquiera de las tres cookies en el jar. Matiz de hoy: la consulta funciona **con y sin**
  esas cookies (no son el gate actual; el gate es el formato del token, ver §3), así que el jar
  queda como **hardening** fiel al flujo Python probado, listo por si el gateway las exige en el
  futuro.

---

## 3. Hipótesis del 401 (orden de probabilidad)

1. **Falta la cookie de sesión del gateway ADC** — la Fase 1 la cubre (jar + siembra + re-siembra
   ante 401). *Actualizado 11-08: con el micro arriba la consulta funcionó **con y sin** cookies
   ADC → ya no eran el gate; el jar queda como hardening fiel al flujo Python probado.*
2. **Token con demasiadas soluciones** — el Python envía `[:1]`; la SPA el array completo.
   *Actualizado 11-08: verificado en vivo — el gateway **rechaza el array completo (401) y acepta
   `[:1]` (200)**. Es el gate real de hoy; por eso la Fase 1 y `check-simit` envían una sola
   solución.*
3. **Gateway caído de verdad** — el 10-08 el portal entero daba 503 y el 401 era idéntico con o sin
   token (firma de gateway bloqueando todo). *Actualizado 11-08: el micro volvió; el 401 sin token
   pasó a ser el comportamiento normal del gateway (valida captchas) y `scripts/check-simit.mjs`
   lo confirma con sonda E2E de token real (clasifica `down`/`gateway`/`up`).*
4. **`Sec-Fetch-*`/`Priority` requeridos** — improbable: el captcha funciona sin ellos y la
   referencia (manavarrp/SimitConsulta) advierte que `Sec-Fetch-*` hace que el servidor bloquee la
   conexión. Rust los omite deliberadamente.

---

## 4. Plan de migración en fases

### Fase 1 — Sesión HTTP como navegador ✅ IMPLEMENTADA Y VALIDADA E2E (10-08 → 11-08)

Cambios en `src-tauri/Cargo.toml` y `src-tauri/src/services/simit.rs`:

1. **Jar persistente**: `ureq features = ["json", "cookies"]` + `.cookie_store(CookieStore::default())`
   (dep directa `cookie_store = "0.21"`, mismo crate que usa ureq internamente).
2. **Siembra de sesión**: `sembrar_cookies_sitio_con()` → `GET https://www.fcm.org.co/` con headers
   de navegador (sin `Origin`/`Referer`, como una navegación real), hasta 2 redirecciones,
   best-effort (no aborta la sincronización). `asegurar_sesion_sitio()` una vez por proceso
   (`AtomicBool`); `resembrar_cookies_sitio()` forzada ante 401.
3. **Token de una solución**: `solo_primera_solucion()` recorta `[a,b]` → `[a]` (como
   `pow_solutions[:1]` del Python).
4. **401 → re-siembra + reintento**: `enviar_consulta()` distingue el 401
   (`ErrorConsulta::Unauthorized` conservando el body del SIMIT para mensajes claros en la UI) y
   `consultar_placa()` re-siembra la sesión y repite captcha + consulta **una vez** con token fresco
   (el token PoW parece ser de un solo uso).

**Flujo de la Fase 1 (secuencia)** — validado contra el portal real el 11-08:

```text
         Agente (simit.rs)            Sitio FCM          Captcha qxcaptcha        Micro consultasimit
         sync_dev / "Sincronizar"     www.fcm.org.co     fcm.org.co              fcm.org.co
                │                         │                     │                      │
   (1) Siembra de sesión — 1ª vez por proceso; headers de navegador, SIN Origin/Referer
                │  GET /                │                     │                      │
                │──────────────────────>│                     │                      │
                │  200 · Set-Cookie: aiovg_rand_seed          │                      │
                │<──────────────────────┘                     │                      │
                │  → jar (CookieStore persistente)            │                      │
                │                         │                     │                      │
   (2) Captcha Proof-of-Work
                │  POST api.php (endpoint=question)           │                      │
                │────────────────────────────────────────────>│                      │
                │  200 {question, recommended_difficulty}     │                      │
                │   + Set-Cookie: ADC_CONN_* · ADC_REQ_*  ◄── cookies ADC            │
                │<────────────────────────────────────────────┘                      │
                │  → jar                                     │                      │
                │                         │                     │                      │
   (3) PoW resuelto — nonces primos con SHA256 "0000" ×dificultad (2)
                │  token = [a]  ← UNA solución (solo_primera_solucion, como Python [:1])
                │                         │                     │                      │
   (4) Consulta de estado de cuenta
                │  POST /estadocuenta/consulta  {filtro, reCaptchaDTO: {response: token}}
                │───────────────────────────────────────────────────────────────────>│
                │    · token ausente/inválido → 401 (gateway: normal, no es caída)   │
                │    · token válido          → 200 {multas[]}                        │
                │<───────────────────────────────────────────────────────────────────┘
                │  (si 401 con token válido → re-siembra + reintento 1 vez con token fresco)
                │                         │                     │                      │
   (5) Deduplicación + insert — BD Firebird dev (data/dinamo_rent_v3.fdb)
                │  por registro:  ¿ya_existe?  (numero_comparendo | placa+fecha+monto)
                │    · no existe → INSERT comparendos (Pendiente, observaciones de trazabilidad)
                │    · existe    → duplicados++ (no se reinserta)
                │    · existe y SIMIT reporta "Pagado" → marcar_pagado_por_numero (sync de estado)
                │                         │                     │                      │
   (6) Cierre — reporte HTML (data/informes_simit/simit_*.html) · dump JSON
                │  (data/simit_watch/sync_result.json) → Excel (scripts/verificar-excel-simit.mjs)
                │                         │                     │                      │
```

**Validación** (snapshot del 10-08): `cargo check --lib` 0/0 · `cargo test --lib` **37/37**
(32 previos + 5 nuevos) · `cargo clippy --lib` sin warnings nuevos (8 totales: los 6 documentados
+ 2 pre-existentes en `simit.rs`). Tests nuevos con mini servidor HTTP local: jar compartido entre
peticiones, la siembra deja la cookie (`ADC_CONN`) en el jar, el 401 se clasifica como
`Unauthorized` con body, y el recorte del token. *Al cierre del 11-08 la suite `--lib` quedó en
**37 passed + 1 `#[ignore]`** (portal real) — módulo simit 14/14, `comparendos_integration` 4/4,
`informeExcel` 6/6.*

**Validación E2E contra el portal real (11-08)** — el microservicio volvió a estar operativo y se
validó el flujo completo sobre la BD dev con un binario de desarrollo nuevo
(`cargo run --features dev --bin sync_dev`, `src-tauri/src/bin/sync_dev.rs` — mismo camino que
«Sincronizar ahora», sin Tauri; dump JSON en `data/simit_watch/sync_result.json`):

- **Insert**: 21/21 placas consultadas, 30 registros encontrados, **27 comparendos reales
  insertados**, 0 errores. Reporte `totalPendiente` = **$21.225.939** (suma de los 30
  encontrados, pre-dedup); **deuda real en la BD tras dedup: $19.212.789** (27 registros) —
  detalle en «Sonda vs BD dev» abajo.
- **Dedup**: 2ª corrida → 0 insertados / 30 duplicados (la BD no creció: 27 total).
- **Sync de estado**: no se pudo disparar la transición Pendiente→Pagado (el SIMIT reporta los 30
  registros como Pendiente); el branch (`ya_existe` + `marcar_pagado_por_numero`) queda cubierto
  por `tests/comparendos_integration.rs` (4/4).
- **Reportes**: HTML en `data/informes_simit/simit_*.html` (30 filas + tarjetas de resumen) y
  export Excel verificado con `scripts/verificar-excel-simit.mjs` (mismo mapeo que el botón de
  `/comparendos`: 30 filas 1:1 con el resultado, montos numéricos, total pendiente correcto).

🐛 **2 bugs reales encontrados y corregidos** (ambos en `simit.rs`):
1. **El SIMIT envía `fechaComparendo` en DD/MM/YYYY** — el mapeo solo aceptaba ISO, así que las
   30 multas reales se descartaban (primera corrida con el código viejo: 0 insertados).
   `parsear_fecha_hora` ahora normaliza DD/MM/YYYY → ISO (tests nuevos: `27/01/2025` →
   `2025-01-27`, `02/02/2026 14:30:00` → `2026-02-02`/`14:30`).
2. **Una fecha malformada abortaba toda la sincronización** — `ya_existe` → `existe_duplicado`
   llama `parse_fecha` ANTES del pre-check de fecha (que estaba después); el pre-check se movió
   antes, cumpliendo la intención documentada («se omite el registro, no aborta la placa»).

🗑️ **Sonda temporal eliminada** (`scripts/simit-sonda-fase1.mjs`, 11-08): cumplió su ciclo. Lo
que dejó probado: (a) el gateway **acepta el token de UNA solución y rechaza el array completo**
(la sonda con `[a]` daba 200 mientras `check-simit` con `[a,b]` daba 401 en el mismo minuto);
(b) las cookies ADC (siembra + jar) **ya no son el gate de hoy** (la consulta funciona con y sin
ellas — el jar queda como hardening fiel al flujo Python). Su flujo quedó absorbido por
`scripts/check-simit.mjs` (actualizado el 11-08): ante un 401 sin token corre la sonda E2E con
token real y usa el token de una sola solución (verificado: **SIMIT operativo**, exit 0); el
comportamiento quedó cubierto por el test `scripts/test-check-simit.mjs`.

**Sonda vs BD dev (11-08)** — relación entre el total de la sonda y la deuda registrada:

| Fuente | Alcance | Registros | Total pendiente |
|---|---|---|---|
| `check-simit --multas` (sonda E2E) | 1 placa de prueba (**AAA000**, NO está en `autos`) | 176 multas | $101.594.688 |
| Reporte de sincronización (`totalPendiente`) | 30 registros **encontrados** (pre-dedup) | 30 | $21.225.939 |
| **BD dev — suma real (`sync_dev --solo-total`)** | 27 comparendos de **21 placas** (dedup) | 27 | **$19.212.789** |

- La sonda es un **chequeo de contrato/liveness**, no del total de la flota: consulta una única
  placa de prueba que no pertenece a la flota (no está en `autos` → el agente nunca la consulta
  vía `placas_activas`). Su total ($101,6M en AAA000) no es comparable con la deuda de la flota;
  sirve para confirmar token válido, endpoint 200 y parseo de multas.
- El `totalPendiente` del reporte **sobrecuenta la deuda registrada**: suma los 30 encontrados,
  incluidos los 3 deduplicados no insertados (1 por placa+fecha+monto — `JWW149 04/08/2026` ×2 —
  y 2 por número oficial; **$2.013.150** entre los tres). La fuente de verdad de la deuda de la
  flota es la **BD dev: $19.212.789 en 27 comparendos**.
- Lectura sin efectos: `cargo run --features dev --bin sync_dev -- --solo-total` (no toca el
  portal ni escribe en la BD).

**Atribución persistente del import (11-08):** además del flujo HTTP, el agente resuelve qué
renta cubría el vehículo el día de la infracción (`ComparendoRepository::renta_del_dia`, misma
lógica que el cruce comparendos↔rentas: rango `[fecha_recogida, devolución real o retorno]`,
sin Canceladas) y persiste `id_renta`/`id_cliente` en cada comparendo nuevo; la alta manual
hace lo mismo. La migración **0016** (`atribucion_comparendo_renta.sql`, DML idempotente)
backfillea los comparendos existentes sin vínculo (test en `migraciones_integration`: inserta
renta + comparendo sin atribución en una copia de la BD dev y verifica que 0016 los vincula).
Con los datos dev actuales el backfill dejó `atribuidos=0` — las 7 rentas activas no cubren
las fechas de las 27 multas (hecho de los datos, no un fallo). Detalle y validación en
`Handsoff.md` (nota de portada y §2).

### Fase 1.5 — Robustez de sesión (sugerida, no implementada)

- Re-siembra **periódica** según el TTL de la cookie ADC (no solo ante 401) — el TTL real se
  desconoce; si la sincronización dura minutos (varias placas × 2.5 s), una cookie corta podría
  caducar a mitad de corrida.
- Config en `[simit]` (`sembrar_sitio: bool`, `re_siembra_minutos`).
- Opcional: `CookieStore` compartido explícitamente (hoy es el del agente global `AGENTE`).

### Fase 2 — Plan B: sesión de navegador real (solo si la Fase 1 no basta)

Port de `test_simit.py` a la app Tauri reutilizando el **WebView2 que ya usa la app**:

- Ventana `WebviewWindow` oculta con la SPA `https://www.fcm.org.co/simit/#/estado-cuenta`.
- Inyección de JS (eval): el sitio resuelve los captchas y acumula los tokens en
  `sessionStorage.whcQuestions` (cola de 5, ver `wehatecaptcha.js`); el agente hace el `fetch` del
  endpoint de consulta **desde dentro de la página** → cookies del navegador + TLS real + SameSite,
  incluido el handshake completo del WAF ADC.
- **Ventajas**: elimina TLS fingerprinting, cookies y «política de seguridad» por construcción;
  es el enfoque que evita el problema en lugar de replicarlo.
- **Coste**: ventana oculta + inyección + ciclo de vida/cleanup; dependencia del JS del portal
  (frágil ante cambios de la SPA); webview cargando un sitio remoto (revisar CSP y capabilities de
  Tauri); el token del PoW lo resuelve el propio sitio (menos control).
- **Alternativa más pesada**: binario sidecar con Chrome headless (Selenium) — solo si el WebView2
  no sirve.

### Fase 3 — Estratégica (opcional)

- Convenios oficiales / API para organismos: RUNT es el canal oficial de consulta para entidades.
- Nota legal: scraping del portal público del FCM — revisar términos de uso antes de depender de
  ello en producción.

---

## 5. Riesgos y notas

- **Portal intermitente**: 503 «Server-unavailable!» frecuentes; el circuit breaker abre tras 5
  fallos (5 min). La E2E depende de la disponibilidad, no del contrato.
- **Contrato vivo**: el portal puede cambiar el PoW, los headers o el JSON; `wehatecaptcha.js`
  (transcripción del comportamiento de la SPA) es la referencia viva del flujo de tokens.
- **TLS fingerprinting descartado** (verificado 10-08 con ureq/rustls, Node/undici y curl).
- El 401 del 10-08 era **idéntico con y sin token** → el gateway estaba bloqueando todo (servicio
  caído). *Actualizado 11-08: con el micro arriba el 401 pasó a distinguir token válido (200) de
  inválido/ausente (401) — comportamiento normal del gateway; la firma «401 uniforme» sigue siendo
  el indicador de caída real.*
- ⚠️ **Windows `os error 32`**: durante el build de la Fase 1 hubo una racha larga (8+ fallos
  seguidos, incluso con `target` fresco) del lock de Defender/índice sobre `target/` — inusual
  frente al patrón transitorio del README §4; amainó solo. Si reaparece: loop de reintentos del
  README o verificar exclusiones.

---

## 6. Referencias

- **API-Runt-simit** (`D:\Proyectos\API-Runt-simit-main`): `app/procesos/simit/service.py`,
  `app/procesos/simit/utils.py`, `app/procesos/simit/wehatecaptcha.js`, `test_simit.py`,
  `debug_response.json`.
- **manavarrp/SimitConsulta** — referencia del contrato (headers, formato del token).
- **Handsoff.md** — §2 (Agente SIMIT), §3 (pendiente), notas de portada del 10-08.
- **README.md** — §Windows (`os error 32`), §Migraciones.
- **ureq 2.12.1** (registry local): `Cargo.toml` (features default), `src/agent.rs` (CookieTin),
  `src/unit.rs` (`extract_cookies`/`store_response_cookies`), `src/cookies.rs`.
- **Herramientas del repo (11-08)**:
  - `scripts/check-simit.mjs` — diagnóstico; ante un 401 sin token corre la sonda E2E con token
    real de UNA solución; `--multas` incluye `microservicio.e2e` (totalPendiente + detalle por
    multa) en el JSON. URLs/DNS sobreescribibles por env para tests
    (`SIMIT_CAPTCHA_URL`/`SIMIT_CONSULTA_URL`/`SIMIT_PAGINA_URL`/`SIMIT_DNS_SKIP`).
  - `scripts/watch-simit.mjs` — vigilante periódico (`npm run watch:simit`); pasa `--multas`,
    persiste el último total pendiente en `data/simit_watch/ultimo_total.json` y alerta cuando
    cambia entre corridas.
  - `scripts/test-check-simit.mjs` — test del flujo 401→sonda E2E con servidor HTTP local
    (`npm run test:check-simit`).
  - `scripts/verificar-excel-simit.mjs` — valida el export Excel de comparendos contra
    `data/simit_watch/sync_result.json` (mismo mapeo que el botón de `/comparendos`).
  - `src-tauri/src/bin/sync_dev.rs` — sincronización E2E sin Tauri sobre la BD dev
    (`cargo run --features dev --bin sync_dev`), dump JSON + snapshot de comparendos antes/después.
  - `services/simit.rs` — test `#[ignore]` `jar_portal_real_captura_cookies_adc` (portal real,
    ver §2.1).
