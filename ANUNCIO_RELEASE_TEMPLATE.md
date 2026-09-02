# Plantilla de anuncio de release (Slack/Teams)

Mensaje listo para pegar en un canal del equipo al publicar una versión nueva.
Las secciones «Versión larga» y «Versión corta» están **completadas con los
datos reales de la v1.2.0** (enlaces, sha256, conteos de tests) — copiar el
bloque elegido tal cual. Para una versión futura, actualizar los valores de
la «Referencia rápida» (versión, URLs de assets, sha256 y conteos de tests)
y reemplazarlos en el texto de las dos secciones.

---

## Versión larga (una pantalla)

```text
🚀 Dinamo Rent ERP — v1.2.0 publicada (cierre de renta completo: valor día extra y cobro de horas extras editables, gasolina en el recálculo y contrato con desglose de costos)

La versión v1.2.0 ya está publicada en GitHub, construida y firmada
por CI (auto-update activo desde la v1.0.14).

📦 Descarga: release v1.2.0 → https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.0/DinamoRent_1.2.0_x64-setup.exe
(~22 MB, NSIS) o el .msi (~34 MB): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.0/DinamoRent_1.2.0_x64_en-US.msi
sha256 NSIS: _(calcular al publicar)_
sha256 MSI:  _(calcular al publicar)_
Credenciales iniciales: admin / admin123 (cambio forzado al primer ingreso).
⚠️ No instalar versiones anteriores descontinuadas.

🔧 Qué incluye esta versión:
  🔄 CIERRE DE RENTA COMPLETO: el modal de cierre permite editar el valor
     día extra y decidir si se cobran las horas extras (checkbox desmarcable
     para hora de gracia/cortesía), recalculando el total
  ⛽ GASOLINA EN EL RECÁLCULO: valor_gasolina ahora se incluye en los extras
     al cerrar o editar una renta (antes quedaba fuera del total)
  ✏️ EDICIÓN DE RENTAS CERRADAS AMPLIADA: corrige valor día extra y el flag
     de cobrar horas extras, con auditoría ANTES→DESPUÉS incluyendo valor_dia_extra
  📄 CONTRATO CON DESGLOSE DE COSTOS: días base × tarifa, horas extras,
     día(s) extra, otros cargos, descuento y TOTAL; tarifa por hora en la
     cláusula de multa por retardo y devolución real registrada (fecha/hora)
     cuando la renta está cerrada

🔄 Auto-update: las instalaciones v1.0.14+ detectan esta versión automáticamente.
   Para v1.0.13 y anteriores: actualiza una vez a mano.

🟢 CI verde en main: lint · vitest · svelte-check · cargo (69 lib + integración
   completa con BD sembrada por seed_ci) · paginación · verificador -DryRun.

📄 Guías: INSTALACION_OPERACIONES.md (instalación) ·
DEPLOYMENT_CLIENTES.md (despliegue y rollback) ·
RESUMEN_EJECUTIVO.md (estado completo).

Resumen completo:
https://github.com/CORJAR-Computers/dinamo_rent_tr/blob/main/RESUMEN_EJECUTIVO.md
```

## Versión corta (anuncio rápido, 2-3 líneas)

```text
🚀 Dinamo Rent ERP v1.2.0 publicada y firmada por CI. Cierre de renta completo: valor día extra y cobro de horas extras editables, gasolina en el recálculo del total y contrato con desglose de costos (tarifa por hora y devolución real). Auto-update desde v1.0.14.
Descarga: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.0
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.2.0:** `DinamoRent_1.2.0_x64-setup.exe` (NSIS, ~22 MB, sha256 `_(calcular al publicar)_`) y `DinamoRent_1.2.0_x64_en-US.msi` (~34 MB, sha256 `_(calcular al publicar)_`)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets v1.2.0:**
  - NSIS: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.0/DinamoRent_1.2.0_x64-setup.exe
  - MSI: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.2.0/DinamoRent_1.2.0_x64_en-US.msi
  - Release (con changelog automático de commits): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.0
  - Firmas del updater: `DinamoRent_1.2.0_x64-setup.exe.sig` / `DinamoRent_1.2.0_x64_en-US.msi.sig`
  - Endpoint del auto-update (latest.json): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json
- **Conteos de tests** (actualizarlos si cambian): vitest · svelte-check · cargo 69 lib + integración completa (seed_ci) · importador · paginación
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → calcular sha256 de los instaladores y completar aquí → pegar el anuncio.
- **Changelog automático:** `release.yml` genera el body de la release con los commits entre el tag anterior y el nuevo.
- **Auto-actualización (v1.0.14+):** la app chequea GitHub Releases al arrancar (`latest.json`), verifica firma minisign. Las instalaciones **≤v1.0.13 se actualizan UNA vez a mano**.
- **Assets de la v1.2.0 (al publicar):** los 2 instaladores, sus firmas del updater y `latest.json`. El sha256 se calcula al publicar y se pega en esta sección.
