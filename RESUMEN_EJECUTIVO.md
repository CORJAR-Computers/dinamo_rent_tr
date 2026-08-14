# Resumen Ejecutivo — Dinamo Rent ERP

> **Fecha:** 2026-08-14 · **Estado general:** listo para producción — release v1.0.3 publicada por CI, firmada para el auto-update y validada de punta a punta en Windows limpio.

---

## 1. Estado actual (una mirada)

| Área | Estado |
|---|---|
| **Aplicación** | Todos los módulos operativos (rentas, comparendos + agente SIMIT, alertas, calendario, informes, reservas, contratos) |
| **Versión estable** | **v1.0.3** — la única release que se distribuye (con auto-update activo) |
| **Instalación limpia** | ✅ Validada E2E en Windows Sandbox (equipo sin nada): la app crea su BD, migra y arranca sin colgarse |
| **CI** | ✅ Verde en el tope de `main` (lint, svelte-check 0/0, 233 tests frontend, cargo 48 lib + 8 rentas + 11 migraciones, importador 16 casos) |
| **Repositorio** | Árbol limpio y sincronizado con `origin/main` |
| **Auto-actualización** | ✅ Activa desde la **v1.0.3** — la app chequea `latest.json` al arrancar y ofrece instalar (firma minisign verificada) | ✅ Secret `TAURI_SIGNING_PRIVATE_KEY` configurado; v1.0.3 publicada y firmada |

## 2. Releases en GitHub

| Release | Estado | Para quién |
|---|---|---|
| **v1.0.3** | ✅ **Latest / estable** — construida íntegramente por CI (GitHub Actions) | **Única descarga recomendada** |
| v1.0.2 | ✅ Estable anterior (sigue funcionando) | Actualizar a v1.0.3 (transición al auto-update) |
| v1.0.1 | ✅ Estable anterior (sigue funcionando) | Actualizar a v1.0.3 |
| v1.0.0 | ⚠️ **Descontinuada** (prerelease + aviso de deprecación) | Solo referencia — **no instalarla** |

**Assets de la v1.0.3:** [`DinamoRent_1.0.3_x64-setup.exe`](https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.3/DinamoRent_1.0.3_x64-setup.exe) (NSIS, ~21 MB, recomendado) y [`DinamoRent_1.0.3_x64_en-US.msi`](https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.3/DinamoRent_1.0.3_x64_en-US.msi) (~31 MB, despliegue GPO; sha256 publicados en la release). La release incluye además los `.sig` y el `latest.json` para el auto-update. Enlaces y credenciales iniciales en [`INSTALACION_OPERACIONES.md`](INSTALACION_OPERACIONES.md).

**Auto-actualización (activa desde la v1.0.3):** la app chequea al arrancar el
`latest.json` de GitHub Releases y ofrece instalar la versión nueva (firma minisign
verificada contra la pubkey embebida). Las instalaciones **v1.0.2 no tienen updater**: se
actualizan una vez a mano con el instalador de la v1.0.3. La release v1.0.3 salió firmada
(.sig + `latest.json` generados por el CI con el secret `TAURI_SIGNING_PRIVATE_KEY`, ya
configurado) y los artefactos publicados validan criptográficamente contra la pubkey
embebida.

**Qué corrigió la v1.0.1** (bugs del instalador v1.0.0 en equipos nuevos; histórica):

1. **La BD no se creaba** → la app se colgaba esperando un `.fdb` inexistente. Ahora `create_pool` crea la BD (y su carpeta) al primer arranque.
2. **Las migraciones no viajaban en el instalador** → las migraciones van embebidas en el binario (fallback automático; hoy 19: 0001-0019).
3. **Crash sin el runtime VC++** (`LoadLibraryExW failed`) → `SetDllDirectoryW(firebird/)` encuentra las DLLs que ya viajan en el instalador; no hace falta instalar redistribuibles.

**Actualizar con datos:** idempotente — cada versión abre la BD existente y solo aplica las migraciones pendientes (no hay que desinstalar ni se pierden datos). Cualquier versión anterior → **v1.0.3** (las v1.0.2 sin updater se instalan a mano una vez; desde la v1.0.3 las siguientes son automáticas).

**Qué añade la v1.0.2** (13-08):

1. **IVA por renta (checkbox)** — el IVA dejó de aplicarse siempre; cada renta decide con «Cobrar IVA» (migración 0019, DEFAULT 1 para rentas existentes).
2. **Auto-cálculo de días/horas al cerrar** — cada 24 h = 1 día; excedente ≤ 3 h → horas extras (redondeadas hacia arriba); excedente > 3 h → día completo.
3. **Cambiar vehículo sin cerrar la renta** — transaccional, con auditoría (`CAMBIO AUTO`); el selector de placa se deshabilita al editar.
4. **Combos con búsqueda** (SearchSelect) en rentas, reservas, comparendos, mantenimiento y gastos (cliente por nombre/documento; vehículo por placa, marca, modelo, tipo o color).

**Qué añade la v1.0.3** (14-08):

1. **Auto-actualización** — updater de Tauri v2: chequea `latest.json` de GitHub al arrancar y ofrece instalar la versión nueva (diálogo «Actualización disponible» con Instalar ahora / Más tarde; firma minisign verificada contra la pubkey embebida antes de instalar).

## 3. CI (GitHub Actions)

- **`ci.yml`** (cada push/PR a main): eslint · svelte-check (0/0) · **vitest (233 tests)** · vite build · **cargo test --lib (48)** (integración en dev: 8 rentas + 11 migraciones) · cargo check (all-targets + bins de mantenimiento) · **test del importador Python (16 casos)**.
- **`release.yml`** (por tag `v*`): construye y publica el instalador (NSIS + MSI) vía `tauri-action`, con **body de release generado automáticamente** (changelog con los commits entre el tag anterior y el nuevo). **Firma los bundles para el auto-update** (`.sig` + `latest.json`) con el secret `TAURI_SIGNING_PRIVATE_KEY` (configurado — la v1.0.3 salió firmada).
- **Nota de operación:** el CI usa `cancel-in-progress` por rama — en pushes consecutivos solo el run del **tope** de main queda completo (los intermedios salen `cancelled`). Para verificar, mirar el run del HEAD.

## 4. Herramientas de operación (`scripts/`)

| Herramienta | Para qué | Uso |
|---|---|---|
| **`importar_autos_clientes.py`** | Poblar Autos/Clientes desde dump SQL o Excel (upsert idempotente por placa/no_doc, PII cifrados con la clave del destino, dry-run por defecto, `--commit` transaccional) | `python scripts/importar_autos_clientes.py --sql dump.sql --commit` |
| **`test_importar_autos_clientes.py`** | Test de regresión del importador (16 casos, sin BD; corre en CI) | `python scripts/test_importar_autos_clientes.py` |
| **`verificar-despliegue.ps1`** | Post-instalación en el equipo del cliente: exe v1.0.3, arranque vivo 10 s, `config.ini` + BD del primer arranque — veredicto OK/FALLOS | `powershell -File scripts/verificar-despliegue.ps1` |
| **`verificar-updater-e2e.sh`** | Verificación E2E del auto-update sin publicar en GitHub: firma un artifact con la clave real, sirve un `latest.json` local (v1.0.3) y valida detección + firma + bytes (caso negativo incluido) | `bash scripts/verificar-updater-e2e.sh` |
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

**El proyecto está listo para producción.** La única release estable es la v1.0.3 (construida por CI, firmada para el auto-update y validada en Windows limpio), la suite completa está en verde (local y CI), y el kit de operaciones (instalación, verificación, importación de datos, monitoreo SIMIT, verificación E2E del updater) está documentado y validado. La **auto-actualización está activa desde la v1.0.3**: el secret `TAURI_SIGNING_PRIVATE_KEY` quedó configurado y los artefactos publicados validan criptográficamente contra la pubkey embebida. Los pendientes conocidos son de mantenimiento fino, no bloqueos.
