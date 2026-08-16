# Plantilla de anuncio de release (Slack/Teams)

Mensaje listo para pegar en un canal del equipo al publicar una versión nueva.
Las secciones «Versión larga» y «Versión corta» están **completadas con los
datos reales de la v1.0.14** (enlaces, sha256, conteos de tests) — copiar el
bloque elegido tal cual. Para una versión futura, actualizar los valores de
la «Referencia rápida» (versión, URLs de assets, sha256 y conteos de tests)
y reemplazarlos en el texto de las dos secciones.

---

## Versión larga (una pantalla)

```text
🚀 Dinamo Rent ERP — lista para producción (v1.0.14)

La versión estable v1.0.14 ya está publicada en GitHub, construida y firmada
por CI, con auto-actualización real por primera vez (firma minisign contra la
clave embebida).

📦 Descarga: release v1.0.14 → https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.14/DinamoRent_1.0.14_x64-setup.exe
(~21 MB, NSIS) o el .msi (~33 MB): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.14/DinamoRent_1.0.14_x64_en-US.msi
sha256 NSIS: 527d93e5526b2da558b90c35b7a8ba772792ada2575a8b79e9573096673040a5
sha256 MSI:  3862b26687a323b5f18642804ae0f7b3cfe882842597a30d126d70a25aaf9ba9
Credenciales iniciales: admin / admin123 (cambio forzado al primer ingreso).
⚠️ No instalar versiones anteriores descontinuadas.

🔧 Qué incluye esta versión:
  🔄 AUTO-UPDATE CORREGIDO: el permiso ACL del plugin updater faltaba desde
     v1.0.3 (el check fallaba en silencio y el modal nunca aparecía). Desde la
     v1.0.14 la app detecta al arrancar si hay una release nueva y pide
     instalarla (firma minisign verificada contra la clave embebida)
  🔘 Botón «Buscar actualización» en la barra superior: check manual con
     feedback visible (modal, «ya tienes la más reciente» o el error real)
  ➕ Crear renta desde una reserva: precarga el formulario (cliente, vehículo,
     fechas, tarifas) y completa la reserva automáticamente al guardar
  🧮 Cálculo unificado de días/horas (24h = 1 día; excedente ≤3h = horas extra,
     >3h = día completo) en renta y reserva, también al cambiar la hora
  🧾 IVA por renta (checkbox), cambio de vehículo sin cerrar la renta
  🔍 Combos con búsqueda en rentas, reservas, comparendos, mantenimiento y gastos

🔄 Para las instalaciones v1.0.13 y anteriores: actualízalas a esta versión
UNA sola vez a mano (el auto-update de ≤v1.0.13 estaba bloqueado por ACL).
Desde la v1.0.14 reciben las siguientes actualizaciones automáticamente.

🟢 CI verde en main: lint · 242 tests frontend (vitest) · svelte-check 0/0 ·
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
🚀 Dinamo Rent ERP v1.0.14 publicada y firmada por CI. Novedades: botón
«Buscar actualización» con feedback visible y AUTO-UPDATE CORREGIDO — el
permiso ACL del plugin faltaba desde v1.0.3 (el check fallaba en silencio);
esta es la primera release que se actualiza sola de verdad. Las instalaciones
≤v1.0.13 se actualizan UNA vez a mano; desde la v1.0.14 reciben las siguientes
automáticamente.
Descarga solo desde la release v1.0.14:
https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.14
Guías y kit de operaciones en el repo:
RESUMEN_EJECUTIVO.md · INSTALACION_OPERACIONES.md · DEPLOYMENT_CLIENTES.md
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.0.14:** `DinamoRent_1.0.14_x64-setup.exe` (NSIS, ~21 MB, sha256 `527d93e5526b2da558b90c35b7a8ba772792ada2575a8b79e9573096673040a5`) y `DinamoRent_1.0.14_x64_en-US.msi` (~33 MB, sha256 `3862b26687a323b5f18642804ae0f7b3cfe882842597a30d126d70a25aaf9ba9`)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets v1.0.14:**
  - NSIS: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.14/DinamoRent_1.0.14_x64-setup.exe
  - MSI: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.14/DinamoRent_1.0.14_x64_en-US.msi
  - Release (con changelog automático de commits): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.14
  - Firmas del updater: `DinamoRent_1.0.14_x64-setup.exe.sig` / `DinamoRent_1.0.14_x64_en-US.msi.sig`
  - Endpoint del auto-update (latest.json): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json
- **Conteos de tests** (actualizarlos si cambian): vitest 242 · svelte-check 0/0 · cargo 48 lib + integración completa (seed_ci) · importador 16
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → marcar versiones anteriores como pre-release/descontinuadas si aplica → pegar el anuncio.
- **Changelog automático:** `release.yml` genera el body de la release con los commits entre el tag anterior y el nuevo (`git log prev..tag`); el anuncio puede enlazar a la página de la release en lugar de repetir la lista de cambios.
- **Auto-actualización (v1.0.14+):** la app chequea GitHub Releases al arrancar (`latest.json`), muestra «Actualización disponible» y verifica la firma minisign contra la pubkey embebida antes de instalar. OJO: las v1.0.3–v1.0.13 tenían el permiso ACL del plugin faltante en `capabilities/default.json` — el check fallaba en silencio y el modal nunca aparecía (corregido en la v1.0.14). Las instalaciones **≤v1.0.13 se actualizan UNA vez a mano** instalando la v1.0.14 encima; desde ahí reciben las siguientes automáticamente. El CI sube `latest.json` + los `.sig` con cada release (prerrequisito: secret `TAURI_SIGNING_PRIVATE_KEY`; ver RELEASE_CHECKLIST.md).
- **Assets de la v1.0.14 (al publicar):** los 2 instaladores (`DinamoRent_1.0.14_x64-setup.exe` NSIS ~21 MB y `DinamoRent_1.0.14_x64_en-US.msi` ~33 MB), sus firmas del updater (`*.exe.sig` / `*.msi.sig`) y `latest.json`. El sha256 de los instaladores se calcula al publicar y se pega en esta sección.
