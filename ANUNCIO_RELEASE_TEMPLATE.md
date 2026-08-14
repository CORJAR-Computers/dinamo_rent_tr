# Plantilla de anuncio de release (Slack/Teams)

Mensaje listo para pegar en un canal del equipo al publicar una versión nueva.
Sustituir los valores entre `{{ }}` y ajustar la lista de correcciones.

---

## Versión larga (una pantalla)

```text
🚀 {{PRODUCTO}} — listo para producción (v{{VERSION}})

La versión estable v{{VERSION}} ya está publicada en GitHub, construida por CI
y validada en Windows limpio (instalación sin colgarse en equipos nuevos).

📦 Descarga: release v{{VERSION}} → {{NSIS_ASSET}} (~{{NSIS_MB}} MB, NSIS)
o el .msi (~{{MSI_MB}} MB). Credenciales iniciales: admin / admin123 (cambio
forzado al primer ingreso). ⚠️ No instalar versiones anteriores descontinuadas.

🔧 Qué incluye esta versión:
  {{BULLET_1}}
  {{BULLET_2}}
  {{BULLET_3}}

🔄 Auto-actualización: a partir de esta versión la app detecta al arrancar si
hay una release nueva y pide instalarla (firma minisign verificada). Si ya
tienes la v1.0.2 instalada, actualízala a esta versión una sola vez a mano —
las siguientes te llegarán solas.

🟢 CI verde en main: lint, 233 tests frontend (vitest), svelte-check 0/0, cargo (48 lib + 8 rentas + 11 migraciones)
+ test del importador de datos (16 casos).

🛠️ Kit de operaciones (repo, scripts/):
  • verificar-despliegue.ps1 — verificación post-instalación por equipo
  • importar_autos_clientes.py — migrar Autos/Clientes desde SQL o Excel
    (PII cifrado, dry-run antes de aplicar)
  • check-simit.mjs / watch-simit.mjs — monitoreo del agente SIMIT
  • smoke test del instalador en Windows Sandbox (reproducible)

📄 Guías: INSTALACION_OPERACIONES.md (instalación) ·
DEPLOYMENT_CLIENTES.md (despliegue y rollback) ·
RESUMEN_EJECUTIVO.md (estado completo).

Resumen completo:
{{REPO_URL}}/blob/main/RESUMEN_EJECUTIVO.md
```

## Versión corta (anuncio rápido, 2-3 líneas)

```text
🚀 {{PRODUCTO}} v{{VERSION}} publicada y validada en Windows limpio — instalación
sin colgarse en equipos nuevos (crea BD, migraciones embebidas, sin
runtime extra). A partir de esta versión la app se actualiza sola; las
instalaciones v1.0.2 se actualizan una vez a mano. Descarga solo desde la
release v{{VERSION}} (versiones anteriores descontinuadas). Guías y kit de operaciones en el repo:
RESUMEN_EJECUTIVO.md · INSTALACION_OPERACIONES.md · DEPLOYMENT_CLIENTES.md
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.0.3:** `DinamoRent_1.0.3_x64-setup.exe` (NSIS, ~21 MB, sha256 `d0b8c07f8f49b38c85690fb2133b805ce1c4374e762fa7e897a321cb07709eca`) y `DinamoRent_1.0.3_x64_en-US.msi` (~31 MB, sha256 `bcfa5ac12e672857cac2f62e7cd676d081a5a4819e01deadd6b46e76334978e2`)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets v1.0.3:**
  - NSIS: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.3/DinamoRent_1.0.3_x64-setup.exe
  - MSI: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.3/DinamoRent_1.0.3_x64_en-US.msi
  - Release (con changelog automático de commits): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.3
- **Conteos de tests** (actualizarlos si cambian): vitest 233 · svelte-check 0/0 · cargo 48 lib + 8 rentas + 11 migraciones · importador 16
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → marcar versiones anteriores como pre-release/descontinuadas si aplica → pegar el anuncio.
- **Changelog automático:** `release.yml` genera el body de la release con los commits entre el tag anterior y el nuevo (`git log prev..tag`); el anuncio puede enlazar a la página de la release en lugar de repetir la lista de cambios.
- **Auto-actualización (v1.0.3+):** la app chequea GitHub Releases al arrancar (`latest.json`), muestra «Actualización disponible» y verifica la firma minisign contra la pubkey embebida antes de instalar. Las instalaciones **v1.0.2 no tienen updater**: se actualizan UNA vez a mano instalando la v1.0.3 encima; desde ahí reciben las siguientes automáticamente. El CI sube `latest.json` + los `.sig` con cada release (prerrequisito: secret `TAURI_SIGNING_PRIVATE_KEY`; ver RELEASE_CHECKLIST.md).
- **Assets de la v1.0.3 (al publicar):** los 2 instaladores (`DinamoRent_1.0.3_x64-setup.exe` NSIS ~21 MB y `DinamoRent_1.0.3_x64_en-US.msi` ~31 MB), sus firmas del updater (`*.exe.sig` / `*.msi.sig`) y `latest.json`. El sha256 de los instaladores se calcula al publicar y se pega en esta sección.
