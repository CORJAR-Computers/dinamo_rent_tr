# Plantilla de anuncio de release (Slack/Teams)

Mensaje listo para pegar en un canal del equipo al publicar una versión nueva.
Las secciones «Versión larga» y «Versión corta» están **preparadas para la v1.2.2**
(con los sha256 y tamaños reales ya publicados). Para una versión futura, actualizar
los valores de la «Referencia rápida» (versión, URLs de assets, sha256 y conteos de
tests) y reemplazarlos en el texto.

---

## Versión larga (una pantalla)

```text
🚀 Dinamo Rent ERP — v1.2.2 publicada (ronda de QA: fechas locales, cobros y salvaguardas)

La versión v1.2.2 ya está publicada en GitHub, construida y firmada
por CI (auto-update activo desde la v1.0.14).

📦 Descarga: release v1.2.2 → https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.2/DinamoRent_1.2.2_x64-setup.exe
(~21 MB, NSIS) o el .msi (~32 MB): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.2/DinamoRent_1.2.2_x64_en-US.msi
sha256 NSIS: 65e6e9b9e3b20714b2ae01e24525168df59395a99baa7bcf692faacefde7c101
sha256 MSI:  c61e61c2e7a7295ec47cf1782f6ad8342aa423ab30364e3b34db95a8c8a90eef
Credenciales iniciales: admin / admin123 (cambio forzado al primer ingreso).
⚠️ No instalar versiones anteriores descontinuadas.

🔧 Qué incluye esta versión:
  📅 FECHAS LOCALES EN TODA LA APP: en UTC-5, tras las 7 PM se proponía la
     fecha de mañana — la más crítica, la devolución real del cierre de rentas
  💰 DOBLE COBRO EN EXTENSIONES ELIMINADO: la extensión se valoriza solo en
     valor_dia_extra + historial, sin tocar la base contractual
  🛟 RESTAURACIÓN MÁS SEGURA: copia preventiva pre_restore y rechazo de
     backups vacíos/truncados antes de reemplazar la BD
  🔒 aes 0.9 + cbc 0.2 (cipher 0.5 unificado) — formatos de cifrado intactos
  🟢 Gate obligatorio de clippy -D warnings en script y pre-commit

🔄 Auto-update: las instalaciones v1.0.14+ detectan esta versión automáticamente.
   Para v1.0.13 y anteriores: actualiza una vez a mano.

🟢 CI verde en main: lint · vitest · svelte-check · cargo (92 lib + integración
   completa con BD sembrada por seed_ci) · clippy -D warnings · paginación ·
   verificador -DryRun.

📄 Guías: INSTALACION_OPERACIONES.md (instalación) ·
DEPLOYMENT_CLIENTES.md (despliegue y rollback) ·
RESUMEN_EJECUTIVO.md (estado completo).

Resumen completo:
https://github.com/CORJAR-Computers/dinamo_rent_tr/blob/main/RESUMEN_EJECUTIVO.md
```

## Versión corta (anuncio rápido, 2-3 líneas)

```text
🚀 Dinamo Rent ERP v1.2.2 publicada y firmada por CI. Ronda de QA: fechas locales en toda la app (en UTC-5 tras las 7 PM se proponía la fecha de mañana — la crítica: devolución real del cierre), doble cobro eliminado en extensiones de renta y restauración de BD con copia preventiva + rechazo de backups vacíos. aes 0.9 + cbc 0.2 con formatos de cifrado intactos. Auto-update desde v1.0.14.
Descarga: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.2
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.2.2:** `DinamoRent_1.2.2_x64-setup.exe` (NSIS, ~21 MB, sha256 `65e6e9b9e3b20714b2ae01e24525168df59395a99baa7bcf692faacefde7c101`) y `DinamoRent_1.2.2_x64_en-US.msi` (~32 MB, sha256 `c61e61c2e7a7295ec47cf1782f6ad8342aa423ab30364e3b34db95a8c8a90eef`)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets v1.2.2:**
  - NSIS: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.2/DinamoRent_1.2.2_x64-setup.exe
  - MSI: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.2/DinamoRent_1.2.2_x64_en-US.msi
  - Release (con changelog automático de commits): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.2
  - Firmas del updater: `DinamoRent_1.2.2_x64-setup.exe.sig` / `DinamoRent_1.2.2_x64_en-US.msi.sig`
  - Endpoint del auto-update (latest.json): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json
- **Conteos de tests** (actualizarlos si cambian): vitest 254 · svelte-check 0/0 · cargo 92 lib + 82 integración (seed_ci) + clippy -D warnings · importador · paginación
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → calcular sha256 de los instaladores y completar aquí → pegar el anuncio.
- **Changelog automático:** `release.yml` genera el body de la release con los commits entre el tag anterior y el nuevo.
- **Auto-actualización (v1.0.14+):** la app chequea GitHub Releases al arrancar (`latest.json`), verifica firma minisign. Las instalaciones **≤v1.0.13 se actualizan UNA vez a mano**.
- **Assets de la v1.2.2 (publicada):** los 2 instaladores, sus firmas del updater y `latest.json`. sha256 NSIS `65e6e9b9e3b20714b2ae01e24525168df59395a99baa7bcf692faacefde7c101`, MSI `c61e61c2e7a7295ec47cf1782f6ad8342aa423ab30364e3b34db95a8c8a90eef` (16-09-2026).