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

### Códigos de salida

| Código | Significado                                   |
| ------ | --------------------------------------------- |
| 0      | todo OK                                       |
| 1      | error técnico (navegador, timeout, archivo…)  |
| 2      | verificación fallida (páginas/tamaño/pie)     |

Útil en CI o en scripts: `node scripts/verificar-paginacion.mjs ... && echo "OK"`.

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
