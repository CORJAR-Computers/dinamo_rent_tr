# Plantilla de anuncio de release (Slack/Teams)

Mensaje listo para pegar en un canal del equipo al publicar una versión nueva.
Las secciones «Versión larga» y «Versión corta» están **completadas con los
datos reales de la v1.0.15** (enlaces, sha256, conteos de tests) — copiar el
bloque elegido tal cual. Para una versión futura, actualizar los valores de
la «Referencia rápida» (versión, URLs de assets, sha256 y conteos de tests)
y reemplazarlos en el texto de las dos secciones.

---

## Versión larga (una pantalla)

```text
🚀 Dinamo Rent ERP — lista para producción (v1.0.15)

La versión estable v1.0.15 ya está publicada en GitHub, construida y firmada
por CI (auto-update activo desde la v1.0.14).

📦 Descarga: release v1.0.15 → https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.15/DinamoRent_1.0.15_x64-setup.exe
(~21 MB, NSIS) o el .msi (~33 MB): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.15/DinamoRent_1.0.15_x64_en-US.msi
sha256 NSIS: a16ccfc4bca7670dd875c50b5b3f1d54fd6d85a627881befcaef53007afed64d
sha256 MSI:  75c4240de498c76bda4689ed4ff424d6970c727e7079204d356409018623ebc2
Credenciales iniciales: admin / admin123 (cambio forzado al primer ingreso).
⚠️ No instalar versiones anteriores descontinuadas.

🔧 Qué incluye esta versión:
  💰 COMISIÓN POR RENTA: checkbox «Cobrar comisión» + valor; el neto
     (total − comisión) se persiste y se muestra en el informe mensual,
     el balance, el listado de rentas y la timeline por vehículo
  📊 INFORME MENSUAL: comisiones, ingresos netos y balance neto (tras
     comisiones); la comisión cuenta como costo en la utilidad por vehículo
  🚔 COMPARENDOS: procedencia persistente (SIMIT/Manual + ultimo_visto_simit),
     filtro «No confirmadas por SIMIT» (≥3 días o nunca confirmado), filtro
     «Solo nuevos de la última sincronización» — combinables (intersección)
  💾 El último resultado del Agente SIMIT se guarda en la BD: el badge 🆕 y
     el filtro «Solo nuevos» sobreviven al reinicio de la app
  🛠️ CI en Node 24 (actions actualizadas) y eliminado el flake de los tests
     del agente SIMIT con servidor TCP local

🔄 Para las instalaciones v1.0.13 y anteriores: actualízalas a esta versión
UNA sola vez a mano (el auto-update de ≤v1.0.13 estaba bloqueado por ACL).
Desde la v1.0.14 reciben las siguientes actualizaciones automáticamente.

🟢 CI verde en main: lint · 250 tests frontend (vitest) · svelte-check 0/0 ·
cargo (48 lib + integración completa con BD sembrada por seed_ci) ·
test del importador de datos (16 casos).

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
https://github.com/CORJAR-Computers/dinamo_rent_tr/blob/main/RESUMEN_EJECUTIVO.md
```

## Versión corta (anuncio rápido, 2-3 líneas)

```text
🚀 Dinamo Rent ERP v1.0.15 publicada y firmada por CI. Novedades: comisión por
renta (checkbox + valor; neto = total − comisión) visible en el informe
mensual, el balance, el listado de rentas y la timeline; comparendos con
procedencia persistente (SIMIT/Manual) y filtros «No confirmadas por SIMIT»
+ «Solo nuevos de la última sincronización» combinables; el último resultado
del agente se guarda en la BD (sobrevive al reinicio). CI en Node 24.
Descarga solo desde la release v1.0.15:
https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.15
Guías y kit de operaciones en el repo:
RESUMEN_EJECUTIVO.md · INSTALACION_OPERACIONES.md · DEPLOYMENT_CLIENTES.md
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.0.15:** `DinamoRent_1.0.15_x64-setup.exe` (NSIS, ~21 MB, sha256 `a16ccfc4bca7670dd875c50b5b3f1d54fd6d85a627881befcaef53007afed64d`) y `DinamoRent_1.0.15_x64_en-US.msi` (~33 MB, sha256 `75c4240de498c76bda4689ed4ff424d6970c727e7079204d356409018623ebc2`)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets v1.0.15:**
  - NSIS: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.15/DinamoRent_1.0.15_x64-setup.exe
  - MSI: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.15/DinamoRent_1.0.15_x64_en-US.msi
  - Release (con changelog automático de commits): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.15
  - Firmas del updater: `DinamoRent_1.0.15_x64-setup.exe.sig` / `DinamoRent_1.0.15_x64_en-US.msi.sig`
  - Endpoint del auto-update (latest.json): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json
- **Conteos de tests** (actualizarlos si cambian): vitest 250 · svelte-check 0/0 · cargo 48 lib + integración completa (seed_ci) · importador 16
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → calcular sha256 de los instaladores y completar aquí → marcar versiones anteriores como pre-release/descontinuadas si aplica → pegar el anuncio.
- **Changelog automático:** `release.yml` genera el body de la release con los commits entre el tag anterior y el nuevo (`git log prev..tag`); el anuncio puede enlazar a la página de la release en lugar de repetir la lista de cambios.
- **Auto-actualización (v1.0.14+):** la app chequea GitHub Releases al arrancar (`latest.json`), muestra «Actualización disponible» y verifica la firma minisign contra la pubkey embebida antes de instalar. OJO: las v1.0.3–v1.0.13 tenían el permiso ACL del plugin faltante en `capabilities/default.json` — el check fallaba en silencio y el modal nunca aparecía (corregido en la v1.0.14). Las instalaciones **≤v1.0.13 se actualizan UNA vez a mano** instalando una versión ≥v1.0.14 encima; desde ahí reciben las siguientes automáticamente. El CI sube `latest.json` + los `.sig` con cada release (prerrequisito: secret `TAURI_SIGNING_PRIVATE_KEY`; ver RELEASE_CHECKLIST.md).
- **Assets de la v1.0.15 (al publicar):** los 2 instaladores (`DinamoRent_1.0.15_x64-setup.exe` NSIS ~21 MB y `DinamoRent_1.0.15_x64_en-US.msi` ~33 MB), sus firmas del updater (`*.exe.sig` / `*.msi.sig`) y `latest.json`. El sha256 de los instaladores se calcula al publicar y se pega en esta sección.
