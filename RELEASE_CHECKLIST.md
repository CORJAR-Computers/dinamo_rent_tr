# Checklist de publicación de release — Dinamo Rent ERP

> Procedimiento operativo para publicar una versión nueva en
> `github.com/CORJAR-Computers/dinamo_rent_tr`: el bump de versión, el tag que
> dispara el CI, la verificación de los assets y el anuncio. Complementa a
> `INSTALACION_OPERACIONES.md` (instalación), `DEPLOYMENT_CLIENTES.md`
> (despliegue a clientes) y `ANUNCIO_RELEASE_TEMPLATE.md` (mensajes de anuncio).
>
> **📋 Objetivo actual: v1.0.21 (preparando).**
> Incluye todas las features de v1.0.17 más: **tracing estructurado** (tracing
> + tracing-subscriber), **informes optimizados** (UNION ALL 13→5 round-trips),
> **store global BusinessLists** (TTL 5 min, invalidable), **repository DRY**
> (core::repository centraliza helpers), **migración 0025 audit_inmutable**
> (triggers append-only para no-repudio), **accesibilidad WCAG 2.1** (Modal
> focus trap, FormField ARIA, skip-link, página de error 404/5xx), **ts-rs**
> para contratos TypeScript, **dependabot** para dependencias npm/cargo/CI.

---

**Estado de la v1.0.17 — publicada (18-08, tag `v1.0.17` sobre `0e0180f`):**

Ya en `main` y publicada por CI: release v1.0.17 con los 5 assets
(NSIS 21 MB + MSI 32 MB + `.sig` ×2 + `latest.json`), firmada y
auto-update verificado en máquina real.

**Features v1.0.17 (incluidas en v1.0.21):**
- Versión REAL de la app (comando `app_version`, sin el v3.2.0 heredado)
- Backups de la BD (Fase 8: automático en 4 horarios, rotación a 10, cifrado
  AES-256-GCM, panel `/backups` y restauración con `gbak -r`)
- **Edición de rentas cerradas** (solo Admin: recálculo de totales, auditoría
  ANTES→DESPUÉS, motivo obligatorio)
- **Extensiones acumulables de rentas** (migración 0024, historial de horas/días
  extras, modal con preview del nuevo retorno)
- **Mayúsculas automáticas** en todos los campos de texto (excepto email, rol,
  web) con validaciones case-insensitive
- Fix de selects de categoría/tipo (gastos/mantenimiento) alineados con la DB
- Verificador de despliegue -DryRun en el CI

**Features v1.0.21 (nuevas):**
- **Tracing estructurado** (spans en login, cerrar_renta, registrar_pago)
- **Informes optimizados** (UNION ALL: 13→5 round-trips en informe mensual)
- **Store global BusinessLists** (TTL 5 min, cachea `businessApi.listas`)
- **core::repository** (centraliza `map_fb_error`, `opt_str`, `parse_fecha/hora`, `params!`)
- **Migración 0025 audit_inmutable** (triggers append-only para no-repudio)
- **Accesibilidad WCAG 2.1** (Modal focus trap, FormField ARIA, skip-link, error page)
- **ts-rs** para contratos TypeScript (genera tipos desde structs Rust)
- **Dependabot** para actualizaciones automáticas de npm/cargo/CI
- **domain/ scaffold** (guía para futura migración a value objects)

**Checklist v1.0.17 completada:**

- [x] CI verde en main (lint, svelte-check, vitest, cargo test --lib + integración)
- [x] Bump a v1.0.17 en los 3 archivos (§2)
- [x] Verificación local con `-DryRun` (§3)
- [x] Commit `chore: versión 1.0.17` + tag `v1.0.17` (§4-5)
- [x] Prueba de campo en máquina real — auto-update verificado ✅
- [x] Anuncio en canal del equipo (§8)

---

## Pendiente para la v1.0.21

- [ ] Bump a v1.0.21 en los 3 archivos (§2)
- [ ] Verificación local con `-DryRun` (§3)
- [ ] Commit `chore: versión 1.0.21` + tag `v1.0.21` (§4-5)
- [ ] Prueba de campo en máquina real — auto-update de v1.0.17→v1.0.21 (§6)
- [ ] Anuncio en canal del equipo (§8)

---

## 0. Regla de oro

**El tag `vX.Y.Z` debe apuntar a un commit donde los tres archivos de versión
ya estén bumpeados.** El CI (`release.yml`) compila el código del commit del tag
y los instaladores se nombran con la versión de `src-tauri/tauri.conf.json`,
NO con el nombre del tag. Un tag sobre un commit sin bumpear publicaría una
release `v1.0.17` con instaladores `DinamoRent_1.0.15_*` (si el bump quedara a medias).

---

## 1. Pre-requisitos

- [ ] `ci.yml` verde en el tope de `main` (lint, svelte-check, vitest, cargo test --lib +
      integración — incluye los tests de backups/restauración con gbak real contra la BD
      sembrada —, importador, test de paginación con el 4º caso: orden de reserva 1 página Carta).
      El workflow de release NO valida: un tag sobre un commit roto publicaría igual.
- [ ] `scripts/verificar-despliegue.ps1 -DryRun` en verde (caso OK exit 0 y `-SimularFallo` exit 1) —
      validado además por `ci.yml` en cada push (paso «Verificador de despliegue (-DryRun)»).
- [ ] Working tree limpio y `main` local = `origin/main`.
- [ ] El secret `TAURI_SIGNING_PRIVATE_KEY` está configurado en Settings → Secrets → Actions
      del repo (y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` solo si la clave tiene password).
      Sin él, `tauri build` NO firma los bundles y la release saldría sin `.sig`/`latest.json`
      → la app instalada no podría auto-actualizarse. La clave privada vive SOLO en
      `~/.tauri/dinamorent.key` de la máquina que la generó: respáldala (si se pierde,
      las instalaciones v1.0.3+ dejarían de actualizarse).
      → Configurar y verificar por CLI: [`SECRET_FIRMA_UPDATER.md`](SECRET_FIRMA_UPDATER.md)

## 2. Bump de versión

Editar la versión en los **tres** archivos (deben coincidir):

| Archivo | Campo |
|---|---|
| `package.json` | `"version": "1.0.17"` |
| `src-tauri/Cargo.toml` | `version = "1.0.17"` (crate `dinamo-rent`) |
| `src-tauri/tauri.conf.json` | `"version": "1.0.17"` |

Verificar la consistencia:

```bash
grep '"version"' package.json src-tauri/tauri.conf.json
grep -m1 '^version' src-tauri/Cargo.toml
```

Actualizar además (patrón del bump 43aa80b):

- [ ] `INSTALACION_OPERACIONES.md` — enlaces de descarga de la release nueva (título, tabla de assets, comandos silenciosos).
- [ ] `README.md` — "última versión estable" y enlaces de descarga.
- [ ] `ANUNCIO_RELEASE_TEMPLATE.md` — datos de referencia (assets, sha256, conteos de tests si cambiaron).

## 3. Verificación local (opcional pero recomendada)

```bash
bun run lint
bunx svelte-check --tsconfig ./tsconfig.json
bunx vitest run
cd src-tauri && cargo test --lib
```

Validar además el verificador de despliegue (sin tocar la máquina):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1 -DryRun
# el caso FALLOS debe terminar con VEREDICTO: FALLOS y exit 1:
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1 -DryRun -SimularFallo
```

## 4. Commit del bump

Mensaje con el estilo del repo (español, prefijo `chore:`):

```text
chore: versión 1.0.17 — versión real de la app, backups de la BD (Fase 8: automático, cifrado, restauración) y documentación al día
# (ajustar el resumen a lo que incluya la release al bumpear)
```

## 5. Publicar: push + tag

```bash
git push origin main
git tag v1.0.17
git push origin v1.0.17
```

El push del tag dispara `release.yml` (GitHub Actions, `windows-latest`):
`checkout` (fetch-depth 0) → **test de paginación** (orden 1 página Carta,
contrato 3-4 páginas con pie, informe A4 — bloquea la release si falla) →
changelog automático → `tauri build` (NSIS + MSI) → crea la release
**publicada** (no draft) y sube los assets. ~10 minutos (referencia v1.0.2:
11 min · v1.0.3: ~10 min).

La **E2E del auto-update en máquina real** no puede correr en el CI (necesita la
release ya publicada): se valida en el §6 como contraparte de campo del test de
paginación.

> El body de la release se genera solo: lista los commits entre el tag anterior
> y el nuevo, con hash corto y mensaje. Si quieres verlo antes de publicar,
> cambia `releaseDraft: true` en `release.yml` y publícala a mano.

> **Auto-actualización (v1.0.3+):** la app instalada comprueba al arrancar si hay una
> release más nueva (endpoint `latest.json` de GitHub) y pide permiso para instalarla.
> Las instalaciones **v1.0.2 no tienen updater**: se actualizan UNA vez a mano con el
> instalador de la v1.0.3 y de ahí en adelante reciben las siguientes automáticamente.

## 6. Verificar la release (no confiar a ciegas en el CI)

- [ ] Release `v1.0.17` existe en <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.17>
      con **5 assets**: los 2 instaladores (`DinamoRent_1.0.17_x64-setup.exe` NSIS ~21 MB y
      `DinamoRent_1.0.17_x64_en-US.msi` ~33 MB), sus firmas del updater (`*.exe.sig` / `*.msi.sig`)
      y `latest.json`. Los `.sig` son de **minisign** (verificación del updater), NO firma de
      código Authenticode.
- [ ] `latest.json` existe y `platforms.windows-x86_64.url` apunta al instalador de esta
      release (el CI elige cuál sube al publicar — en la v1.0.3 fue el `.msi`) — es lo que la
      app instalada (v1.0.3+) consulta al arrancar para auto-actualizarse. En la v1.0.17 la app
      instalada debe DETECTAR la release nueva y ofrecer instalarla (prueba de campo del
      auto-update).
- [ ] **E2E del auto-update — validación sin máquina** (antes de la prueba de campo):
      desde un árbol dev **aún en v1.0.15** (antes del bump local o desde el commit previo),
      verificar que la release publicada se detecta, su firma valida y los bytes coinciden,
      con el binario de desarrollo `updater_e2e`:

      ```bash
      cd src-tauri && cargo run --features dev --bin updater_e2e -- \
        --endpoint https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json \
        --expect-version 1.0.17 \
        --expect-file ./DinamoRent_1.0.17_x64_en-US.msi
      ```

      Debe terminar con `[OK]` (check() detecta v1.0.17, firma minisign verificada contra
      la pubkey de producción y bytes idénticos al instalador).

      > ⚠️ El updater sirve por defecto la plataforma `windows-x86_64` = **MSI** — el
      > `--expect-file` debe ser el `.msi` (verificado el 18-08 contra la release real).
      > Para validar el NSIS en su lugar, usar `--expect-file ./DinamoRent_1.0.17_x64-setup.exe`
      > apuntando el endpoint a una copia local del `latest.json` con `windows-x86_64-nsis`
      > como plataforma por defecto (o verificar el NSIS con `verificar-updater-e2e.sh`).
- [ ] **E2E del auto-update en máquina real** (la contraparte de campo del test de
      paginación de `release.yml`, que sí corre en CI): en un equipo con una **v1.0.14+
      instalada** (p. ej. la v1.0.15), abrir la app y confirmar que **detecta la v1.0.17**
      («Actualización disponible»), descarga con verificación de firma minisign, instala
      y **reinicia en la v1.0.17** — verificar la versión resultante (menú lateral / login
      o `scripts/verificar-despliegue.ps1` → VEREDICTO OK).
- [ ] **Paginación en máquina real** (contraparte de campo del test de `release.yml`): desde
      una renta real, «Ver contrato (Carta)» → contrato en **3-4 páginas Carta** con pie
      «Página X de Y», y la **orden de reserva en 1 página Carta** (sin cortes ni columnas
      desperdiciadas). Si se exporta el PDF se puede validar con
      `node scripts/verificar-paginacion.mjs contrato.pdf=3:4 --tamano carta --pie`.
- [ ] El **body contiene el changelog** (commits del rango).
- [ ] Los enlaces responden HTTP 200 y el tamaño coincide:

```powershell
# En el PC objetivo
Get-FileHash .\DinamoRent_1.0.17_x64-setup.exe -Algorithm SHA256
# comparar contra el sha256 publicado por GitHub en la página de la release
```

- [ ] (Opcional) `scripts/verificar-despliegue.ps1` en un equipo de prueba → VEREDICTO OK
      (sin `-DryRun`: chequeos reales sobre la instalación; el modo `-DryRun` se valida
      en `ci.yml` y en el §3, no requiere máquina).

## 7. Actualizar la operación

Si el bump cambió algo de operación (p. ej. el check de versión del exe):

- [ ] `scripts/verificar-despliegue.ps1` — `Check "Version 1.0.17" ($ver -like '1.0.17*')`.
- [ ] `DEPLOYMENT_CLIENTES.md` — versión esperada e instaladores en la tabla de verificación.
- [ ] `RESUMEN_EJECUTIVO.md` — versión estable, assets, conteos.
- [ ] `Handsoff.md` — cabecera y nota de portada de la release nueva.
- [ ] Commitear estos ajustes y empujar.

## 8. Anunciar

- [x] Marcar releases anteriores si aplica (la v1.0.15 pasa a "estable anterior" en `RESUMEN_EJECUTIVO.md` §2; la v1.0.9 ya lo es — no se descontinúa salvo motivo).
- [ ] Pegar el mensaje de `ANUNCIO_RELEASE_TEMPLATE.md` (versión larga o corta, **ya con los
      sha256 reales** `d0f043df…`/`ef93c9c1…`) en Slack/Teams con los enlaces de descarga y
      el resumen de la release. **El bloque está listo para copiar tal cual** (18-08).

---

## Checklist exprés (resumen)

```
[ ] CI verde en main (incluye verificar-despliegue.ps1 -DryRun)
[ ] Bump en package.json + Cargo.toml + tauri.conf.json (idénticos, 1.0.17)
[ ] Docs de descarga actualizadas (INSTALACION_OPERACIONES.md, README.md, ANUNCIO)
[ ] commit chore: versión 1.0.17
[ ] git push origin main && git push origin v1.0.17
[ ] Release publicada por CI con changelog y 5 assets (NSIS + MSI + .sig x2 + latest.json)
[ ] sha256 verificado contra el publicado
[ ] verificar-despliegue.ps1 → OK (equipo de prueba)
[ ] E2E auto-update en máquina real → v1.0.17 detectada, instalada y reiniciada
[ ] Docs de operación al día (ps1, DEPLOYMENT_CLIENTES, RESUMEN, Handsoff)
[ ] Anuncio en Slack/Teams
```
