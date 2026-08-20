# Resumen Ejecutivo — Dinamo Rent ERP

> **Fecha:** 2026-08-18 · **Estado general:** listo para producción — release v1.0.21 (bumpeada y validada por CI; publicación con el tag), firmada para el auto-update, con backups de la BD (Fase 8: automático, cifrado y restauración), la versión real de la app en el menú/login y la verificación de despliegue -DryRun en el CI.

---

## 1. Estado actual (una mirada)

| Área | Estado |
|---|---|
| **Aplicación** | Todos los módulos operativos (rentas, comparendos + agente SIMIT, alertas, calendario, informes, reservas, contratos) |
| **Versión estable** | **v1.0.21** — la única release que se distribuye (con auto-update activo) |
| **Instalación limpia** | ✅ Validada E2E en Windows Sandbox (equipo sin nada): la app crea su BD, migra y arranca sin colgarse |
| **CI** | ✅ Verde en el tope de `main` (lint, svelte-check 0/0, 250 tests frontend, cargo 69 lib + integración con BD sembrada (backups/restauración con gbak real), importador 16 casos) |
| **Repositorio** | Árbol limpio y sincronizado con `origin/main` |
| **Auto-actualización** | ✅ Activa desde la **v1.0.14** — la app chequea `latest.json` al arrancar y ofrece instalar (firma minisign verificada) | ✅ Secret `TAURI_SIGNING_PRIVATE_KEY` configurado; v1.0.4–v1.0.15 publicadas y firmadas (la v1.0.21 sale firmada con el tag) |

## 2. Releases en GitHub

| Release | Estado | Para quién |
|---|---|---|
| **v1.0.21** | ✅ **Latest / estable** — construida íntegramente por CI (GitHub Actions) | **Única descarga recomendada** |
| v1.0.15 | ✅ Estable anterior (sigue funcionando) | Actualizar a v1.0.21 (a mano, una vez) |
| v1.0.9 | ✅ Estable anterior (sigue funcionando) | Actualizar a v1.0.21 (a mano, una vez) |
| v1.0.3 | ✅ Estable anterior (sigue funcionando) | Actualizar a v1.0.21 (a mano, una vez) |
| v1.0.2 | ✅ Estable anterior (sigue funcionando) | Actualizar a v1.0.21 (a mano; transición al auto-update) |
| v1.0.1 | ✅ Estable anterior (sigue funcionando) | Actualizar a v1.0.21 (a mano) |
| v1.0.0 | ⚠️ **Descontinuada** (prerelease + aviso de deprecación) | Solo referencia — **no instalarla** |

**Assets de la v1.0.21:** [`DinamoRent_1.0.21_x64-setup.exe`](https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.21/DinamoRent_1.0.21_x64-setup.exe) (NSIS, ~21 MB, recomendado) y [`DinamoRent_1.0.21_x64_en-US.msi`](https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.21/DinamoRent_1.0.21_x64_en-US.msi) (~33 MB, despliegue GPO; sha256 publicados en la release). La release incluye además los `.sig` y el `latest.json` para el auto-update. Enlaces y credenciales iniciales en [`INSTALACION_OPERACIONES.md`](INSTALACION_OPERACIONES.md).

**Auto-actualización (activa desde la v1.0.14):** la app chequea al arrancar el
`latest.json` de GitHub Releases y ofrece instalar la versión nueva (firma minisign
verificada contra la pubkey embebida). Las instalaciones **≤v1.0.13** (incluidas las v1.0.2,
sin updater) se actualizan **una vez a mano** con un instalador **≥v1.0.14**. La release v1.0.3 salió firmada
(.sig + `latest.json` generados por el CI con el secret `TAURI_SIGNING_PRIVATE_KEY`, ya
configurado) y los artefactos publicados validan criptográficamente contra la pubkey
embebida.

**Qué corrigió la v1.0.1** (bugs del instalador v1.0.0 en equipos nuevos; histórica):

1. **La BD no se creaba** → la app se colgaba esperando un `.fdb` inexistente. Ahora `create_pool` crea la BD (y su carpeta) al primer arranque.
2. **Las migraciones no viajaban en el instalador** → las migraciones van embebidas en el binario (fallback automático; hoy 20: 0001-0020).
3. **Crash sin el runtime VC++** (`LoadLibraryExW failed`) → `SetDllDirectoryW(firebird/)` encuentra las DLLs que ya viajan en el instalador; no hace falta instalar redistribuibles.

**Actualizar con datos:** idempotente — cada versión abre la BD existente y solo aplica las migraciones pendientes (no hay que desinstalar ni se pierden datos). Cualquier versión anterior → **v1.0.21** (las ≤v1.0.13 se instalan a mano una vez; desde la v1.0.14 las siguientes son automáticas).

**Qué añade la v1.0.2** (13-08):

1. **IVA por renta (checkbox)** — el IVA dejó de aplicarse siempre; cada renta decide con «Cobrar IVA» (migración 0019, DEFAULT 1 para rentas existentes).
2. **Auto-cálculo de días/horas al cerrar** — cada 24 h = 1 día; excedente ≤ 3 h → horas extras (redondeadas hacia arriba); excedente > 3 h → día completo.
3. **Cambiar vehículo sin cerrar la renta** — transaccional, con auditoría (`CAMBIO AUTO`); el selector de placa se deshabilita al editar.
4. **Combos con búsqueda** (SearchSelect) en rentas, reservas, comparendos, mantenimiento y gastos (cliente por nombre/documento; vehículo por placa, marca, modelo, tipo o color).

**Qué añade la v1.0.3** (14-08):

1. **Auto-actualización** — updater de Tauri v2: chequea `latest.json` de GitHub al arrancar y ofrece instalar la versión nueva (diálogo «Actualización disponible» con Instalar ahora / Más tarde; firma minisign verificada contra la pubkey embebida antes de instalar).

**Qué añaden las v1.0.4 → v1.0.12** (14-08, diagnóstico y documento):

1. **Errores de BD visibles** (v1.0.4) — el toast de error ahora muestra el detalle real de Firebird (SQLCODE, columna, lock) y el logging se activa en producción (`%APPDATA%\com.corjar.dinamorent\logs\app.log`, 5 MB por archivo con rotación conservada).
2. **Fix del error -303** (v1.0.5) — los campos monetarios vacíos en rentas/reservas/autos se normalizan a `0.00` antes del `CAST(? AS DECIMAL)` de Firebird (antes: `conversion error from string ""` al crear/modificar rentas).
3. **Contrato y orden más legibles** (v1.0.6) — contrato en **2 hojas Carta** (tipografía ajustada), `+57` automático a los celulares del encabezado, cláusula 4 con la multa en **blanco** (espacio para llenar a mano en vez de `$0`), póliza de lucro cesante con valores 40/50/70 mil, y **campo Gasolina** en el formulario de renta (migración 0020, se muestra en la orden).
4. **Fix del INSERT de rentas** (v1.0.8) — conteo de placeholders corregido (34=34) tras el error -804 que apareció al crear rentas en la v1.0.6/v1.0.7.
5. **Contrato en 2 hojas definitivo** (v1.0.9) — logo 70px y encabezado compacto; verificado con el PDF real (2 páginas Carta, pie «Página X de Y»).
6. **Contrato sin cortes al imprimir** (v1.0.10) — margen inferior 16mm (antes 8mm) + interlineado 0.94, para que el final del documento (firmas y cláusulas) no se corte en impresoras reales; `orphans/widows: 2` evita párrafos cortados a 1 línea entre hojas.
7. **Kilometraje sin cola de ceros** (v1.0.11) — `km_salida` e inspecciones se normalizan al leer (`km_limpio`): `42000` en vez de `42000.000000000000`, conservando decimales significativos.
8. **Firmas con espacio amplio** (v1.0.12) — 44px sobre cada línea de firma (antes 10px, imposible firmar), compensado con espaciados internos para seguir en 2 hojas; verificado con el PDF real.

## 3. CI (GitHub Actions)

- **`ci.yml`** (cada push/PR a main): eslint · svelte-check (0/0) · **vitest (250 tests)** · vite build · **cargo test --lib (69)** + integración con BD sembrada por `seed_ci` (incluye backups/restauración con gbak real) · cargo check (all-targets + bins de mantenimiento) · **test del importador Python (16 casos)**.
- **`release.yml`** (por tag `v*`): construye y publica el instalador (NSIS + MSI) vía `tauri-action`, con **body de release generado automáticamente** (changelog con los commits entre el tag anterior y el nuevo). **Firma los bundles para el auto-update** (`.sig` + `latest.json`) con el secret `TAURI_SIGNING_PRIVATE_KEY` (configurado — la v1.0.3 salió firmada).
- **Nota de operación:** el CI usa `cancel-in-progress` por rama — en pushes consecutivos solo el run del **tope** de main queda completo (los intermedios salen `cancelled`). Para verificar, mirar el run del HEAD.

## 4. Herramientas de operación (`scripts/`)

| Herramienta | Para qué | Uso |
|---|---|---|
| **`importar_autos_clientes.py`** | Poblar Autos/Clientes desde dump SQL o Excel (upsert idempotente por placa/no_doc, PII cifrados con la clave del destino, dry-run por defecto, `--commit` transaccional) | `python scripts/importar_autos_clientes.py --sql dump.sql --commit` |
| **`test_importar_autos_clientes.py`** | Test de regresión del importador (16 casos, sin BD; corre en CI) | `python scripts/test_importar_autos_clientes.py` |
| **`verificar-despliegue.ps1`** | Post-instalación en el equipo del cliente: exe v1.0.21, arranque vivo 10 s, `config.ini` + BD del primer arranque — veredicto OK/FALLOS | `powershell -File scripts/verificar-despliegue.ps1` |
| **`verificar-updater-e2e.sh`** | Verificación E2E del auto-update sin publicar en GitHub: firma un artifact con la clave real, sirve un `latest.json` local y valida detección + firma + bytes (caso negativo incluido) | `bash scripts/verificar-updater-e2e.sh` |
| **`dinamorent-sandbox.wsb` + `smoke-test-sandbox.ps1`** | Smoke test del instalador en Windows limpio (Sandbox) | abrir el `.wsb`; resultado en `smoke-result.txt` |
| **`verificar-despliegue-sandbox.ps1` + `dinamorent-sandbox-verificar.wsb`** | Validar el verifier contra una instalación real en Sandbox | abrir el `.wsb` |
| **`check-simit.mjs` / `watch-simit.mjs` / `test-check-simit.mjs`** | Monitoreo del agente SIMIT: disponibilidad del portal, sonda E2E con token, alertas de total pendiente | `node scripts/check-simit.mjs` |
| **`backup-antes-rotacion.sh` / `verificar-rotacion.sh`** | Respaldo y verificación antes de rotar la clave PII | ver `SECURITY.md` |

## 5. Documentación clave

| Documento | Contenido |
|---|---|
| [`INSTALACION_OPERACIONES.md`](INSTALACION_OPERACIONES.md) | Guía de instalación para operaciones: enlaces directos a los assets, credenciales iniciales, verificación, rollback y auto-actualización (§6) |
| [`DEPLOYMENT_CLIENTES.md`](DEPLOYMENT_CLIENTES.md) | Plan de despliegue en equipos de clientes: instalación silenciosa, verificación post-instalación, backup y rollback — alineado al auto-update (último despliegue manual) |
| [`Handsoff.md`](Handsoff.md) | Registro técnico completo: módulos, decisiones, herramientas de operación (§6), bugs de instalación y CI |
| [`README.md`](README.md) | Descarga/instalación para usuarios finales + documentación de desarrollo |
| [`SECURITY.md`](SECURITY.md) | Secretos, rotación de clave PII, reporte de vulnerabilidades |

## 6. Veredicto

**El proyecto está listo para producción.** La única release estable es la v1.0.21 (construida por CI, firmada para el auto-update, validada en Windows limpio y verificada en el equipo de operaciones), la suite completa está en verde (local y CI), y el kit de operaciones (instalación, verificación, importación de datos, monitoreo SIMIT, verificación E2E del updater, backups con restauración) está documentado y validado. La **auto-actualización está activa desde la v1.0.14**: el secret `TAURI_SIGNING_PRIVATE_KEY` quedó configurado y los artefactos publicados validan criptográficamente contra la pubkey embebida. Los pendientes conocidos son de mantenimiento fino, no bloqueos.
