# Chequeo del portal SIMIT

`scripts/check-simit.mjs` comprueba en un comando el estado de los dos servicios
que usa el **Agente SIMIT** (`src-tauri/src/services/simit.rs`), para saber
cuándo reintentar la verificación end-to-end **sin recrear tests temporales**:

```bash
node scripts/check-simit.mjs
# o:
npm run check:simit
```

## Qué comprueba

| Paso | Verificación |
| ---- | ------------ |
| **DNS** | **previo**: resuelve `qxcaptcha.fcm.org.co` y `consultasimit.fcm.org.co` (módulo `dns` de Node). Si no resuelven → `dns_caido` (el 10-08 los subdominios desaparecieron del DNS mientras el dominio raíz seguía vivo) |
| Captcha | POST `qxcaptcha.fcm.org.co/api.php` (endpoint=question) y resuelve el **Proof-of-Work** (mismo algoritmo que el backend: nonces primos con SHA256 `0000…`, token 1:1) |
| Página | GET `consultasimit.fcm.org.co` (código HTTP) |
| Consulta | POST del endpoint de estado de cuenta **sin token** → clasifica por la firma del gateway (401 `codigo:5` = portal caído, 503 = caído) |
| Sonda E2E | si el microservicio no está claramente caído, resuelve el captcha real y consulta una **placa de prueba** con el token (solo si el paso anterior no es concluyente) |

### Opciones

- `--placa <PLACA>` — placa para la sonda E2E (default `AAA000`, env `SIMIT_PLACA`).
- `--timeout <ms>` — timeout por petición (default `15000`, env `SIMIT_TIMEOUT_MS`).
- `--solo-captcha` / `--solo-micro` — comprueba solo un servicio (omite el chequeo DNS previo).
- `--json` — salida JSON (para scripts/CI).
- `--ayuda` — muestra la ayuda.

### Códigos de salida

| Código | Significado |
| ------ | ----------- |
| 0      | SIMIT operativo (captcha + microservicio listos para E2E) |
| 1      | error técnico (red, timeout) **o DNS caído** (`dns_caido`) |
| 2      | SIMIT caído (captcha o microservicio no operativo) |

Útil en scripts: `node scripts/check-simit.mjs && echo "E2E lista"`.

> **Firma de gateway caído**: el portal responde `401 {"codigo":5,"descripcion":
> "Autenticación fallida: Acceso denegado..."}` a **cualquier** petición (con o sin
> token) y la página principal da `503 Server-unavailable!` — fallo del gateway de
> seguridad externo, no del contrato de la app.

---

# Vigilante del portal SIMIT

`scripts/watch-simit.mjs` re-ejecuta `check-simit.mjs` **cada 2 horas** (default;
intervalo de sincronización del Agente SIMIT) hasta que el portal vuelva a estar
operativo, sin intervención manual. Pensado para el corte del 10-08 en que los
subdominios `qxcaptcha`/`consultasimit` dejaron de resolver en DNS.

```bash
npm run watch:simit
# o:
node scripts/watch-simit.mjs [--interval 2] [--max-horas 168]
```

**Comportamiento:**

- Cada intento ejecuta `check-simit.mjs --json` y lo registra (timestamp +
  resultado) en `data/simit_watch/watch.log` (gitignored).
- Mientras el portal siga caído o el DNS sin resolver, sigue esperando e
  imprimiendo el estado de cada intento.
- En cuanto el chequeo devuelva **0 (SIMIT operativo)**, imprime el aviso y
  termina con código **0** — la E2E se puede reintentar.
- `Ctrl+C` detiene el vigilante en cualquier momento (el log queda guardado).

### Opciones

- `--interval <horas>` — intervalo entre chequeos (default `2`, env `SIMIT_WATCH_INTERVAL_H`).
- `--max-horas <n>` — tiempo máximo total de vigilancia (default `168` = 7 días, env
  `SIMIT_WATCH_MAX_H`). `0` = sin límite.
- `--json` — solo salida JSON del último intento (para CI/scripts).

### Códigos de salida

| Código | Significado |
| ------ | ----------- |
| 0      | el portal SIMIT volvió a estar operativo (y `watch.log` tiene el historial) |
| 1      | se alcanzó `--max-horas` sin recuperación, o error inesperado |

> El log de sesión vive en `data/simit_watch/watch.log` (fuera de git) para
> revisar el historial de intentos en cualquier momento.

---

# Verificación de paginación

Script reutilizable para comprobar que los documentos imprimibles de la app
ocupen las páginas correctas en **tamaño Carta**, usando el mismo motor que
produce el PDF real. Acepta HTML estático (se imprime con Chrome/Edge headless)
o **PDFs ya generados** (se analizan directo, p. ej. los del smoke test):

```
HTML estático → Chrome/Edge headless → PDF ─┐
                                            ├→ contador de páginas
PDF ya generado ────────────────────────────┘   (+ tamaño, pie «Página X de Y»)
```

## Uso

```bash
node scripts/verificar-paginacion.mjs <archivo.html|archivo.pdf> [más...] [opciones]
```

Expectativas por archivo (sufijo `=`):

| Sintaxis                | Significado                     |
| ----------------------- | ------------------------------- |
| `archivo.html`          | solo informa la paginación      |
| `archivo.html=3`        | exige exactamente 3 páginas     |
| `archivo.html=3:4`      | exige entre 3 y 4 páginas       |

### Ejemplos

```bash
# Contrato: 3 páginas Carta (máx. 4) con pie «Página X de Y»; orden: 1 página exacta
node scripts/verificar-paginacion.mjs contrato.html=3:4 orden.html=1 --tamano carta

# Con el pie numerado
node scripts/verificar-paginacion.mjs contrato.html=3:4 --tamano carta --pie

# Fixtures de demostración incluidos en scripts/fixtures/
node scripts/verificar-paginacion.mjs scripts/fixtures/una-pagina.html=1
node scripts/verificar-paginacion.mjs scripts/fixtures/tres-paginas.html=3:4 --tamano carta --pie
node scripts/verificar-paginacion.mjs scripts/fixtures/a4.html --tamano a4

# Forzar el motor Edge (idéntico al WebView2 de la app) y conservar los PDFs
node scripts/verificar-paginacion.mjs contrato.html=3:4 --motor edge --salida ./pdfs
```

### Opciones principales

- `--motor auto|chrome|edge` — motor Chromium (por defecto `auto`: Edge → Chrome → sistema).
- `--bin <ruta>` — binario explícito (o variable de entorno `DINAMO_BROWSER_BIN`).
- `--tamano carta|letter|a4|AxB` — verifica el MediaBox del PDF contra ese tamaño.
- `--pie` — verifica el pie «Página X de Y» del margen `@page` (best-effort: depende
  de la fuente; ver el comentario en el script).
- `--headers` — simula el diálogo con «Encabezados y pies de página» ACTIVADO.
- `--salida <dir>` / `--conservar` — conservar los PDFs generados.

### Notas de robustez

- Tras imprimir, el script **espera con polling** a que el PDF exista: el proceso
  padre de Chrome/Edge headless sale con código 0 **antes** de que su renderizador
  escriba el archivo (sin esa espera, un PDF válido se reportaba como «archivo
  vacío»).
- Si el navegador no genera el PDF con `--no-pdf-header-footer` (algunas versiones
  de Edge headless), se reintenta automáticamente sin ese flag usando un perfil
  nuevo y se avisa en el output (el PDF queda con el chrome del navegador; la
  verificación `--pie` puede verse afectada).
- Los perfiles temporales se limpian al final con reintentos (un proceso Edge
  rezagado puede bloquear el directorio un instante).

### Códigos de salida

| Código | Significado                                   |
| ------ | --------------------------------------------- |
| 0      | todo OK                                       |
| 1      | error técnico (navegador, timeout, archivo…)  |
| 2      | verificación fallida (páginas/tamaño/pie)     |

Útil en CI o en scripts: `node scripts/verificar-paginacion.mjs ... && echo "OK"`.

### Test automatizado

`scripts/test-verificar-paginacion.mjs` valida el verificador contra los
fixtures de demostración (orden → 1 página Carta · contrato → 3-4 páginas con
pie · informe → A4) y exige que cada uno cumpla su expectativa real. Corre en
el CI (`ci.yml`) y en cada release (`release.yml`, antes de publicar — una
regresión de paginación bloquea la release), siempre con Edge headless de
windows-latest:

```bash
node scripts/test-verificar-paginacion.mjs   # o: npm run test:paginacion
```

## Uso con los documentos reales de la app

Dos vías para verificar el contrato y la orden reales (componentes Svelte
`ContratoRenta` / `OrdenRenta`):

**A) App compilada (recomendada):** el smoke test captura los PDFs reales con
`Page.printToPDF` y el verificador los analiza directo (sección *Flujo
completo* al final de este documento).

**B) HTML estático:** renderizar los componentes con datos de prueba a
archivos `.html` (harness local, no incluido en el repo) que embeban el CSS
compilado de `npm run build`, y verificar (el `--pie` solo al contrato):

```bash
node scripts/verificar-paginacion.mjs contrato.html=3:4 --tamano carta --pie
node scripts/verificar-paginacion.mjs orden.html=1 --tamano carta
```

El motor de verificación es el mismo que usa la app: la aplicación corre sobre
**WebView2** (motor Edge/Chromium), y este script imprime con el binario de Edge
o Chrome instalado en el equipo.

---

# Smoke test de la app compilada

`scripts/smoke-test-app.mjs` controla el **binario real de la app** (release o
debug) vía el protocolo CDP de WebView2, recorre el flujo de negocio completo y
captura los PDFs reales de orden y contrato con `Page.printToPDF` — el mismo
pipeline de renderizado que usa el diálogo de impresión, dentro del runtime
WebView2 real de la app:

```
app compilada (WebView2) ← CDP ← smoke-test-app.mjs → PDFs reales
```

## Uso

```bash
# 1) Lanzar la app con depuración remota de WebView2 habilitada
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 \
  ./src-tauri/target/release/dinamo-rent.exe

# 2) Ejecutar el humo-test
npm run smoke:app
# o con opciones:
node scripts/smoke-test-app.mjs --puerto 9222 --pwd 'Admin123!' --dir .tmp-print
```

### Opciones

- `--puerto <n>` — puerto CDP de WebView2 (default `9222`, env `CDP_PORT`).
- `--pwd <pass>` — contraseña del admin (default `Admin123!`, env `APP_PWD`).
- `--dir <dir>` — directorio de salida (default `.tmp-print`).
- `--ayuda` — muestra la ayuda.

### Qué verifica

| Paso | Verificación |
| ---- | ------------ |
| Login | autentica (o detecta sesión activa) y entra al módulo |
| Rentas | la tabla carga (o crea una renta de prueba si no hay) |
| Pago | registra un pago (la orden muestra la tabla de pagos) |
| Orden | el modal muestra el aviso «Encabezados y pies…» y captura el PDF |
| Contrato | el modal renderiza y captura el PDF |

Resultados: `orden-real.pdf`, `contrato-real.pdf` y capturas PNG de ambos
modales en el directorio de salida. Códigos de salida: `0` OK · `1` fallo.

> **Nota:** los selectores (ids del login, títulos de botones, clases
> `.print-area.*`) dependen de la UI actual; si cambian los componentes hay
> que actualizar el script.

### Verificar los PDFs generados

El verificador de paginación también acepta PDFs ya generados (los analiza sin
navegador):

```bash
# Contrato real: 3 páginas Carta (máx. 4) con pie «Página X de Y»
node scripts/verificar-paginacion.mjs .tmp-print/contrato-real.pdf=3:4 --tamano carta --pie

# Orden real: exactamente 1 página Carta, sin pie
node scripts/verificar-paginacion.mjs .tmp-print/orden-real.pdf=1 --tamano carta
```

---

# Flujo completo: build → smoke test → verificación de paginación

Recorrido de extremo a extremo para validar los documentos imprimibles con la
**app real compilada**:

## 1) Build

```bash
npx tauri build
# exe: src-tauri/target/release/dinamo-rent.exe
```

- El bundle incluye el runtime Firebird embedded completo (el glob de
  `tauri.conf.json` copia `resources/firebird` íntegro; sin los archivos raíz
  — `fbclient.dll`, `firebird.conf`, ICU — la app no arranca).
- El paso MSI/WiX puede fallar en algunos equipos; el exe se genera igualmente.

## 2) Lanzar con CDP

```bash
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 \
  ./src-tauri/target/release/dinamo-rent.exe
```

La depuración remota de WebView2 expone el puerto CDP (9222) que usa el smoke
test. Para el binario debug: `./src-tauri/target/debug/dinamo-rent.exe`.

## 3) Smoke test

```bash
npm run smoke:app
# → login → rentas → pago → orden → contrato
# → .tmp-print/orden-real.pdf · contrato-real.pdf + capturas PNG
```

## 4) Verificación de paginación

```bash
# Contrato: 3 páginas Carta (máx. 4) con pie «Página X de Y»
node scripts/verificar-paginacion.mjs .tmp-print/contrato-real.pdf=3:4 --tamano carta --pie

# Orden: exactamente 1 página Carta, sin pie
node scripts/verificar-paginacion.mjs .tmp-print/orden-real.pdf=1 --tamano carta
```

Salida `0` = correcto. Resultado esperado (verificado con la app real):
**contrato 3 páginas Carta con pie 1/2/3 de 3 · orden 1 página Carta sin pie**.

### Variante rápida sin la app (solo las reglas CSS de impresión)

```bash
node scripts/verificar-paginacion.mjs scripts/fixtures/tres-paginas.html=3:4 --tamano carta --pie
node scripts/verificar-paginacion.mjs scripts/fixtures/una-pagina.html=1 --tamano carta
```

---

# Tests completos del proyecto (un comando)

`scripts/test-completo.sh` **verifica el entorno** (node, npm, bun, cargo, rustc
con el linker MSVC de VS Build Tools) y corre **todos los tests del proyecto**
desde la raíz, en Git Bash / MSYS2:

```bash
bash scripts/test-completo.sh                 # lint + svelte-check + vitest + cargo test --lib
bash scripts/test-completo.sh --instalar      # + bun install --frozen-lockfile primero
bash scripts/test-completo.sh --integra       # + tests de integración Rust (requiere BD dev)
bash scripts/test-completo.sh --solo-frontend # solo frontend
bash scripts/test-completo.sh --solo-backend  # solo backend
```

## Qué verifica del entorno

| Herramienta | Cómo se comprueba | Si falta, el mensaje da la instalación |
| ----------- | ----------------- | -------------------------------------- |
| Node ≥ 22.4 | `node --version` | `winget install --id OpenJS.NodeJS.LTS -e` |
| npm | `npm --version` | viene con Node |
| Bun | `bun --version` | `npm install -g bun` |
| Cargo + rustc | `cargo`/`rustc --version` | rustup (toolchain `stable-x86_64-pc-windows-msvc`) |
| Linker MSVC | busca `link.exe` en `Program Files (x86)/Microsoft Visual Studio/2022/*/VC/Tools/MSVC/*` | VS Build Tools 2022 con la workload C++ |

> **Por qué importa el linker MSVC:** Tauri v2 en Windows solo compila con el
> toolchain MSVC; sin `link.exe` (VS Build Tools + workload C++) `cargo` falla
> al enlazar aunque rustup esté instalado.

El script añade a `PATH` (si faltan) las rutas típicas de Windows en formato
MSYS: `C:\Program Files\nodejs`, `%USERPROFILE%\.cargo\bin` y
`%APPDATA%\npm` (bun instalado con `npm -g`).

## Qué tests corre (por defecto)

1. `bun run lint` (eslint) — 0 errores
2. `bunx svelte-kit sync && bunx svelte-check --tsconfig ./tsconfig.json` — 0 errores
3. `bunx vitest run` — 233/233 (frontend)
4. `cd src-tauri && cargo test --lib` — 48/48 (backend, sin BD)

Con `--integra` corre además `cargo test --tests` (suites de integración:
migraciones, rentas y el resto), que **requieren la BD de desarrollo**
(`data/dinamo_rent_v3.fdb`, gitignored). Antes de correr, el script avisa si
la BD **no existe** o está **sin flota** (0 autos), sugiriendo
`bash scripts/setup-bd-dev.sh` (la receta completa de la §6.3 del Handsoff:
crea la BD, aplica las migraciones y siembra autos/clientes de prueba).

Códigos de salida: `0` todo verde · `1` falló el entorno o algún test ·
`2` opción desconocida.

---

# BD de desarrollo lista desde cero (un comando)

`scripts/setup-bd-dev.sh` ejecuta los **7 pasos de la receta del Handsoff
(§6.3)** para dejar `data/dinamo_rent_v3.fdb` funcional en un clon nuevo —
sin esto, `cargo test --tests` sale verde sin probar nada (las suites con
flota ahora fallan con un mensaje que indica correr este script):

```bash
bash scripts/setup-bd-dev.sh              # setup completo (idempotente)
bash scripts/setup-bd-dev.sh --verificar  # + cargo test --tests al final
```

## Los 7 pasos

1. `sync_dev --solo-total` → crea `config.ini` + la BD + aplica las 19 migraciones
   (no toca el portal SIMIT).
2. Paquetes Python → `firebird-driver`, `cryptography` (los necesita el importador).
3. Clave PII → genera `db_encryption_key` en `[security]` si está vacía (la clave
   queda SOLO en el config local, nunca en el repo).
4. Flota de prueba → `importar_autos_clientes.py` con `scripts/fixtures`
   (dry-run → `--commit`): 2 autos + 2 clientes con PII cifrada.
5. Admin → seed solo si no existe (verificar_instalacion_limpia) + reset a
   **`Admin123!`** con `dev_reset_admin` (lo que espera `auth_integration`).
6. Historial de auditoría → `LOGIN OK` / `LOGIN FALLIDO` (lo exige
   `auditoria_acciones_y_usuarios`), solo si faltan.
7. Identidad de rentas → reinicia el IDENTITY a `max(1000, MAX(id)+1)` (lo exige
   `renta_no_contrato_secuencial_independiente_del_id`); no borra nada.

Todo es **idempotente**: se puede volver a correr sin romper el estado actual.
Códigos de salida: `0` listo · `1` falló algún paso (aborta con el paso
señalado) · `2` opción desconocida.

> **Nota para Git Bash en Windows:** el driver de Firebird embedded necesita
> `fbclient.dll` en `PATH`; el script la añade en formato MSYS (el importador
> trae hardcodeada una ruta `D:\...` que no aplica en todos los clones, así que
> el PATH del script es lo que hace funcionar la conexión).


---

# Verificación E2E del auto-update (sin publicar en GitHub)

`scripts/verificar-updater-e2e.sh` comprueba de punta a punta el flujo de
auto-actualizacion (`tauri-plugin-updater`, feature de la v1.0.3) **sin tocar
GitHub**: firma un artifact de prueba con la clave real, lo sirve desde
127.0.0.1 y valida que la app lo detecte y verifique su firma.

```bash
bash scripts/verificar-updater-e2e.sh
```

Que valida:

1. **Firma real**: firma un instalador de prueba (1 MiB) con la clave privada
   de `~/.tauri/dinamorent.key` (la misma que usara el CI con el secret
   `TAURI_SIGNING_PRIVATE_KEY`).
2. **latest.json**: arma un `latest.json` (siguiente patch > version del repo,
   derivado de `tauri.conf.json`) con la firma y lo sirve en un puerto libre
   de 127.0.0.1.
3. **Deteccion**: `src-tauri/src/bin/updater_e2e.rs` (dev, `--features dev`)
   monta la app Tauri headless con el plugin real y `check()` debe detectar
   la version nueva.
4. **Verificacion de firma**: `download()` valida la firma contra la **pubkey
   embebida en `tauri.conf.json`** (la misma que usa la app instalada) y que
   los bytes descargados sean identicos al artifact servido.
5. **Negativo**: con un `latest.json` de la version del repo -> sin
   actualizacion.

Codigos de salida: `0` todo verde · `1` fallo algo o falta entorno (cargo,
bun o la clave de firma).

> Nota Git Bash: el signer de tauri-cli pregunta la contrasena por stdin aun
> cuando la clave no tiene; el script pasa `-p ""` para firmar de forma no
> interactiva (sin eso, cuelga esperando input).

> Nota: valida localmente el flujo completo. El flujo real ya esta activo desde
> la v1.0.3 (publicada y firmada con el secret `TAURI_SIGNING_PRIVATE_KEY`
> configurado; ver RELEASE_CHECKLIST.md) — el endpoint real ya sirve
> `latest.json` con la version vigente.
