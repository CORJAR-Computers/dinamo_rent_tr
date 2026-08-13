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

🟢 CI verde en main: lint, 226 tests frontend, 43 Rust, 11 migraciones
+ test del importador de datos.

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
runtime extra). Descarga solo desde la release v{{VERSION}} (versiones
anteriores descontinuadas). Guías y kit de operaciones en el repo:
RESUMEN_EJECUTIVO.md · INSTALACION_OPERACIONES.md · DEPLOYMENT_CLIENTES.md
```

## Referencia rápida para rellenar

- **Producto:** Dinamo Rent ERP
- **Repo:** https://github.com/CORJAR-Computers/dinamo_rent_tr
- **Assets de la v1.0.1 (ejemplo):** `DinamoRent_1.0.1_x64-setup.exe` (NSIS, ~21 MB) y `DinamoRent_1.0.1_x64_en-US.msi` (~31 MB)
- **Credenciales iniciales:** `admin` / `admin123` (cambio forzado)
- **URLs directas de assets:** ver `INSTALACION_OPERACIONES.md` §1
- **Conteos de tests** (actualizarlos si cambian): vitest 226 · cargo test --lib 43 · migraciones 11 · importador 16
- **Pasos al publicar:** crear tag `vX.Y.Z` → `release.yml` (CI) construye y publica → marcar versiones anteriores como pre-release/descontinuadas si aplica → pegar el anuncio.
