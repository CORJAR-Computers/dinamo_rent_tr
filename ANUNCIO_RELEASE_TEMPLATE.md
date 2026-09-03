# Plantilla de anuncio de release (Slack/Teams)

Mensaje listo para pegar en un canal del equipo al publicar una versión nueva.
Las secciones «Versión larga» y «Versión corta» están **preparadas para la v1.2.1**
— completar los sha256 y tamaños **tras publicar** (los calcula el paso final de la
release), actualizar los conteos de tests si cambian, y copiar el bloque elegido tal
cual. Para una versión futura, actualizar los valores de la «Referencia rápida»
(versión, URLs de assets, sha256 y conteos de tests) y reemplazarlos en el texto.

---

## Versión larga (una pantalla)

```text
🚀 Dinamo Rent ERP — v1.2.1 publicada (migración criptográfica con formatos intactos y CI determinista)

La versión v1.2.1 ya está publicada en GitHub, construida y firmada
por CI (auto-update activo desde la v1.0.14).

📦 Descarga: release v1.2.1 → https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.1/DinamoRent_1.2.1_x64-setup.exe
(~22 MB, NSIS) o el .msi (~34 MB): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.1/DinamoRent_1.2.1_x64_en-US.msi
sha256 NSIS: 6f086a6cb23e82343ae32305a789c903e9d0191e356a57addfbed0bbefef0039
sha256 MSI:  6f00145efc5791d0de7dbff61eab6315c7ee2053821295dff816e1238dec5b85
Credenciales iniciales: admin / admin123 (cambio forzado al primer ingreso).
⚠️ No instalar versiones anteriores descontinuadas.

🔧 Qué incluye esta versión:
  🔐 MIGRACIÓN CRIPTOGRÁFICA: rand 0.10, argon2 0.6, hmac 0.13 y aes-gcm 0.11 —
     el cifrado PII, los hash de contraseñas y los backups cifrados existentes
     se siguen leyendo SIN cambios de formato
  🧪 CI DETERMINISTA: los tests de backups de integración se serializan y
     reintentan lecturas frescas — se acabaron los fallos intermitentes por
     Defender en el runner del CI
  📦 log 0.4.34 (dependabot)

🔄 Auto-update: las instalaciones v1.0.14+ detectan esta versión automáticamente.
   Para v1.0.13 y anteriores: actualiza una vez a mano.

🟢 CI verde en main: lint · vitest · svelte-check · cargo (91 lib + integración
   completa con BD sembrada por seed_ci) · paginación · verificador -DryRun.

📄 Guías: INSTALACION_OPERACIONES.md (instalación) ·
DEPLOYMENT_CLIENTES.md (despliegue y rollback) ·
RESUMEN_EJECUTIVO.md (estado completo).

Resumen completo:
https://github.com/CORJAR-Computers/dinamo_rent_tr/blob/main/RESUMEN_EJECUTIVO.md
```

## Versión corta (anuncio rápido, 2-3 líneas)

```text
🚀 Dinamo Rent ERP v1.2.1 publicada y firmada por CI. Migración criptográfica (rand 0.10, argon2 0.6, hmac 0.13, aes-gcm 0.11) con los formatos de salida intactos — el cifrado PII, los hash y los backups existentes siguen leyéndose igual. Tests de backups deterministas y log 0.4.34. Auto-update desde v1.0.14.
Descarga: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.1
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.2.1:** `DinamoRent_1.2.1_x64-setup.exe` (NSIS, ~22 MB, sha256 `6f086a6cb23e82343ae32305a789c903e9d0191e356a57addfbed0bbefef0039`) y `DinamoRent_1.2.1_x64_en-US.msi` (~34 MB, sha256 `6f00145efc5791d0de7dbff61eab6315c7ee2053821295dff816e1238dec5b85`)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets v1.2.1:**
  - NSIS: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.1/DinamoRent_1.2.1_x64-setup.exe
  - MSI: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.1/DinamoRent_1.2.1_x64_en-US.msi
  - Release (con changelog automático de commits): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.1
  - Firmas del updater: `DinamoRent_1.2.1_x64-setup.exe.sig` / `DinamoRent_1.2.1_x64_en-US.msi.sig`
  - Endpoint del auto-update (latest.json): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json
- **Conteos de tests** (actualizarlos si cambian): vitest · svelte-check · cargo 91 lib + integración completa (seed_ci) · importador · paginación
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → calcular sha256 de los instaladores y completar aquí → pegar el anuncio.
- **Changelog automático:** `release.yml` genera el body de la release con los commits entre el tag anterior y el nuevo.
- **Auto-actualización (v1.0.14+):** la app chequea GitHub Releases al arrancar (`latest.json`), verifica firma minisign. Las instalaciones **≤v1.0.13 se actualizan UNA vez a mano**.
- **Assets de la v1.2.1 (publicada):** los 2 instaladores, sus firmas del updater y `latest.json`. sha256 NSIS `6f086a6cb23e82343ae32305a789c903e9d0191e356a57addfbed0bbefef0039`, MSI `6f00145efc5791d0de7dbff61eab6315c7ee2053821295dff816e1238dec5b85` (03-09-2026).