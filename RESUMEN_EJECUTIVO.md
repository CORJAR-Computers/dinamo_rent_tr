# Resumen Ejecutivo — Dinamo Rent ERP

> **Fecha:** 2026-08-13 · **Estado general:** listo para producción — release v1.0.1 estable publicada por CI y validada de punta a punta en Windows limpio.

---

## 1. Estado actual (una mirada)

| Área | Estado |
|---|---|
| **Aplicación** | Todos los módulos operativos (rentas, comparendos + agente SIMIT, alertas, calendario, informes, reservas, contratos) |
| **Versión estable** | **v1.0.1** — la única release que se distribuye |
| **Instalación limpia** | ✅ Validada E2E en Windows Sandbox (equipo sin nada): la app crea su BD, migra y arranca sin colgarse |
| **CI** | ✅ Verde en el tope de `main` (lint, check, 226 tests frontend, 43 tests Rust, 11 migraciones, test del importador) |
| **Repositorio** | Árbol limpio y sincronizado con `origin/main` |

## 2. Releases en GitHub

| Release | Estado | Para quién |
|---|---|---|
| **v1.0.1** | ✅ **Latest / estable** — construida íntegramente por CI (GitHub Actions) | **Única descarga recomendada** |
| v1.0.0 | ⚠️ **Descontinuada** (prerelease + aviso de deprecación con enlace a v1.0.1) | Solo referencia — **no instalarla** |

**Assets de la v1.0.1:** `DinamoRent_1.0.1_x64-setup.exe` (NSIS, ~21 MB, recomendado) y `DinamoRent_1.0.1_x64_en-US.msi` (~31 MB, despliegue GPO). Enlaces directos y credenciales iniciales en [`INSTALACION_OPERACIONES.md`](INSTALACION_OPERACIONES.md).

**Qué corrige la v1.0.1** (bugs del instalador v1.0.0 en equipos nuevos):

1. **La BD no se creaba** → la app se colgaba esperando un `.fdb` inexistente. Ahora `create_pool` crea la BD (y su carpeta) al primer arranque.
2. **Las migraciones no viajaban en el instalador** → las 16 migraciones van embebidas en el binario (fallback automático).
3. **Crash sin el runtime VC++** (`LoadLibraryExW failed`) → `SetDllDirectoryW(firebird/)` encuentra las DLLs que ya viajan en el instalador; no hace falta instalar redistribuibles.

**Actualizar desde v1.0.0 con datos:** idempotente — la v1.0.1 abre la BD existente y solo aplica migraciones pendientes (no hay que desinstalar ni perder datos).

## 3. CI (GitHub Actions)

- **`ci.yml`** (cada push/PR a main): eslint · svelte-check · **vitest (226 tests)** · vite build · **cargo test --lib (43)** · cargo check (all-targets + bins de mantenimiento) · **test del importador Python (16 casos)**.
- **`release.yml`** (por tag `v*`): construye y publica el instalador (NSIS + MSI) vía `tauri-action`.
- **Nota de operación:** el CI usa `cancel-in-progress` por rama — en pushes consecutivos solo el run del **tope** de main queda completo (los intermedios salen `cancelled`). Para verificar, mirar el run del HEAD.

## 4. Herramientas de operación (`scripts/`)

| Herramienta | Para qué | Uso |
|---|---|---|
| **`importar_autos_clientes.py`** | Poblar Autos/Clientes desde dump SQL o Excel (upsert idempotente por placa/no_doc, PII cifrados con la clave del destino, dry-run por defecto, `--commit` transaccional) | `python scripts/importar_autos_clientes.py --sql dump.sql --commit` |
| **`test_importar_autos_clientes.py`** | Test de regresión del importador (16 casos, sin BD; corre en CI) | `python scripts/test_importar_autos_clientes.py` |
| **`verificar-despliegue.ps1`** | Post-instalación en el equipo del cliente: exe v1.0.1, arranque vivo 10 s, `config.ini` + BD del primer arranque — veredicto OK/FALLOS | `powershell -File scripts/verificar-despliegue.ps1` |
| **`dinamorent-sandbox.wsb` + `smoke-test-sandbox.ps1`** | Smoke test del instalador en Windows limpio (Sandbox) | abrir el `.wsb`; resultado en `smoke-result.txt` |
| **`verificar-despliegue-sandbox.ps1` + `dinamorent-sandbox-verificar.wsb`** | Validar el verifier contra una instalación real en Sandbox | abrir el `.wsb` |
| **`check-simit.mjs` / `watch-simit.mjs` / `test-check-simit.mjs`** | Monitoreo del agente SIMIT: disponibilidad del portal, sonda E2E con token, alertas de total pendiente | `node scripts/check-simit.mjs` |
| **`backup-antes-rotacion.sh` / `verificar-rotacion.sh`** | Respaldo y verificación antes de rotar la clave PII | ver `SECURITY.md` |

## 5. Documentación clave

| Documento | Contenido |
|---|---|
| [`INSTALACION_OPERACIONES.md`](INSTALACION_OPERACIONES.md) | Guía de instalación para operaciones: enlaces directos a los assets, credenciales iniciales, verificación, rollback |
| [`DEPLOYMENT_CLIENTES.md`](DEPLOYMENT_CLIENTES.md) | Plan de despliegue en equipos de clientes: instalación silenciosa, verificación post-instalación, backup y rollback |
| [`Handsoff.md`](Handsoff.md) | Registro técnico completo: módulos, decisiones, herramientas de operación (§6), bugs de instalación y CI |
| [`README.md`](README.md) | Descarga/instalación para usuarios finales + documentación de desarrollo |
| [`SECURITY.md`](SECURITY.md) | Secretos, rotación de clave PII, reporte de vulnerabilidades |

## 6. Veredicto

**El proyecto está listo para producción.** La única release estable es la v1.0.1 (construida por CI y validada en Windows limpio), la suite completa está en verde (local y CI), y el kit de operaciones (instalación, verificación, importación de datos, monitoreo SIMIT) está documentado y validado. Los pendientes conocidos son de mantenimiento fino, no bloqueos.
