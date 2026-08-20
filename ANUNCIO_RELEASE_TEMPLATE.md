# Plantilla de anuncio de release (Slack/Teams)

Mensaje listo para pegar en un canal del equipo al publicar una versión nueva.
Las secciones «Versión larga» y «Versión corta» están **completadas con los
datos reales de la v1.0.21** (enlaces, sha256, conteos de tests) — copiar el
bloque elegido tal cual. Para una versión futura, actualizar los valores de
la «Referencia rápida» (versión, URLs de assets, sha256 y conteos de tests)
y reemplazarlos en el texto de las dos secciones.

---

## Versión larga (una pantalla)

```text
🚀 Dinamo Rent ERP — v1.0.21 publicada (performance + accesibilidad + code quality)

La versión v1.0.21 ya está publicada en GitHub, construida y firmada
por CI (auto-update activo desde la v1.0.14).

📦 Descarga: release v1.0.21 → https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.21/DinamoRent_1.0.21_x64-setup.exe
(~22 MB, NSIS) o el .msi (~34 MB): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.21/DinamoRent_1.0.21_x64_en-US.msi
sha256 NSIS: d8d602315c7eb8a4d8a08db6fe3d76cc30604ad8beb1eac68cb8dc9bf177f224
sha256 MSI:  6aefb6d9d278ec451607ea65bcf30eccc3f4126fe6fd5d2de733a64a7cbfb8bb
Credenciales iniciales: admin / admin123 (cambio forzado al primer ingreso).
⚠️ No instalar versiones anteriores descontinuadas.

🔧 Qué incluye esta versión:
  ⚡ INFORMES OPTIMIZADOS: el informe mensual pasó de 13 a 5 round-trips
     con queries UNION ALL (totales_rango + movimientos_por_placa)
  📦 STORE GLOBAL BusinessLists: cachea las listas de config con TTL 5 min,
     evita 1 round-trip por cada navegación a rentas/autos/clientes/reservas
  🔄 ASYNC SPAWN_BLOCKING: listar_rentas e informe_mensual ahora corren en
     threads separados, sin bloquear el event loop de Tauri
  🏗️ REPOSITORY DRY: core::repository centraliza helpers duplicados
     (map_fb_error, opt_str, parse_fecha/hora, params!) en 3 repositorios
  🔒 AUDITORÍA INMUTABLE (migración 0025): triggers append-only en la tabla
     auditoria — no se puede UPDATE ni DELETE (no-repudio, Ley 1581)
  📊 TRACING ESTRUCTURADO: spans de tracing en login, cerrar_renta y
     registrar_pago (coexistencia con tauri-plugin-log, RUST_LOG configurable)
  ♿ ACCESIBILIDAD WCAG 2.1: Modal con focus trap + autofocus + restore,
     FormField con ARIA (label for, aria-describedby, aria-invalid),
     skip-link de accesibilidad, página de error global (404/5xx)
  📝 ts-rs: genera contratos TypeScript (Renta, Pago, Inspeccion, RentaDatos)
     automáticamente desde structs Rust con cargo test
  🤖 Dependabot: actualizaciones automáticas semanales de npm/cargo/CI

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
🚀 Dinamo Rent ERP v1.0.21 publicada y firmada por CI. Novedades: informes
optimizados (13→5 queries), store global BusinessLists (TTL 5 min), tracing
estructurado, auditoría inmutable (triggers append-only), accesibilidad
WCAG 2.1 (Modal focus trap, ARIA, skip-link), ts-rs para contratos
TypeScript y dependabot para dependencias. Auto-update desde v1.0.14.
Descarga: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.21
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.0.21:** `DinamoRent_1.0.21_x64-setup.exe` (NSIS, ~22 MB, sha256 `d8d602315c7eb8a4d8a08db6fe3d76cc30604ad8beb1eac68cb8dc9bf177f224`) y `DinamoRent_1.0.21_x64_en-US.msi` (~34 MB, sha256 `6aefb6d9d278ec451607ea65bcf30eccc3f4126fe6fd5d2de733a64a7cbfb8bb`)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets v1.0.21:**
  - NSIS: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.21/DinamoRent_1.0.21_x64-setup.exe
  - MSI: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.21/DinamoRent_1.0.21_x64_en-US.msi
  - Release (con changelog automático de commits): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.21
  - Firmas del updater: `DinamoRent_1.0.21_x64-setup.exe.sig` / `DinamoRent_1.0.21_x64_en-US.msi.sig`
  - Endpoint del auto-update (latest.json): https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json
- **Conteos de tests** (actualizarlos si cambian): vitest · svelte-check · cargo 69 lib + integración completa (seed_ci) · importador · paginación
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → calcular sha256 de los instaladores y completar aquí → pegar el anuncio.
- **Changelog automático:** `release.yml` genera el body de la release con los commits entre el tag anterior y el nuevo.
- **Auto-actualización (v1.0.14+):** la app chequea GitHub Releases al arrancar (`latest.json`), verifica firma minisign. Las instalaciones **≤v1.0.13 se actualizan UNA vez a mano**.
- **Assets de la v1.0.21 (al publicar):** los 2 instaladores, sus firmas del updater y `latest.json`. El sha256 se calcula al publicar y se pega en esta sección.
