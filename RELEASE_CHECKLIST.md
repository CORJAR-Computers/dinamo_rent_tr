# Checklist de publicación de release — Dinamo Rent ERP

> Procedimiento operativo para publicar una versión nueva en
> `github.com/CORJAR-Computers/dinamo_rent_tr`: el bump de versión, el tag que
> dispara el CI, la verificación de los assets y el anuncio. Complementa a
> `INSTALACION_OPERACIONES.md` (instalación), `DEPLOYMENT_CLIENTES.md`
> (despliegue a clientes) y `ANUNCIO_RELEASE_TEMPLATE.md` (mensajes de anuncio).
>
> **📋 Objetivo actual: v1.0.16 (en preparación, sin publicar aún).**
> Incluye la versión REAL de la app en el menú lateral y el login (comando
> `app_version`, package_info → Cargo.toml / tauri.conf.json en el build —
> antes mostraba la v3.2.0 heredada del proyecto anterior), el modo -DryRun
> de `verificar-despliegue.ps1` (validación del flujo sin tocar la máquina
> real) con su paso en `ci.yml`, la actualización de la documentación a la
> v1.0.15 y los **backups de la BD** (Fase 8 del plan): automáticos en los
> 4 horarios de `[backup] schedule_times` con rotación a 10 copias
> (gbak + fallback a copia del `.fdb`), cifrado opcional AES-256-GCM por
> chunks (PBKDF2-SHA256, `encryption_enabled`/`encryption_password`) y el
> panel `/backups` (solo Administrador) con «Crear backup ahora», listado de
> copias, estado de la última corrida y **restauración** de un backup
> (descifra si está cifrado, `gbak -r` con reinicio de la app;
> `backup_estado`/`backup_ahora`/`backup_restaurar`). Al publicar el tag, el
> auto-update (v1.0.14+) + el paso de paginación de `release.yml` + la E2E
> del auto-update en máquina real (§6) quedan probados de punta a punta.

---

**Estado de la v1.0.16 — pasos restantes antes del tag:**

Ya en `main` y validados por el CI (run #87 verde): versión REAL de la app
(`app_version`), modo -DryRun del verificador, docs a la v1.0.15 y la **Fase 8
de backups completa** (automático en 4 horarios, rotación a 10, cifrado
AES-256-GCM, panel `/backups` y restauración con `gbak -r`). El setup wizard y
el diálogo de config BD quedaron **fuera del alcance** (proyecto de uso interno
de Dinamo: la instalación con defaults basta).

Pendiente antes de crear el tag `v1.0.16`:

1. Confirmar el secret `TAURI_SIGNING_PRIVATE_KEY` en Actions (§1).
2. Bump a `1.0.16` en `package.json`, `src-tauri/Cargo.toml` y
   `src-tauri/tauri.conf.json` (§2).
3. Actualizar enlaces de descarga en `INSTALACION_OPERACIONES.md`, `README.md`
   y `ANUNCIO_RELEASE_TEMPLATE.md` (§2).
4. Verificación local opcional + `verificar-despliegue.ps1 -DryRun` (§3).
5. Commit `chore: versión 1.0.16` + push (§4).
6. Tag `v1.0.16` + push → `release.yml` publica con changelog y los 5 assets (§5).
7. Verificación §6: assets, `latest.json`, **E2E del auto-update**
   (validación sin máquina con `updater_e2e` + prueba de campo v1.0.14+),
   paginación en campo, sha256.
8. Docs de operación al día (§7) y anuncio (§8).

---

## 0. Regla de oro

**El tag `vX.Y.Z` debe apuntar a un commit donde los tres archivos de versión
ya estén bumpeados.** El CI (`release.yml`) compila el código del commit del tag
y los instaladores se nombran con la versión de `src-tauri/tauri.conf.json`,
NO con el nombre del tag. Un tag sobre un commit sin bumpear publicaría una
release `v1.0.16` con instaladores `DinamoRent_1.0.15_*` (si el bump quedara a medias).

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
| `package.json` | `"version": "1.0.16"` |
| `src-tauri/Cargo.toml` | `version = "1.0.16"` (crate `dinamo-rent`) |
| `src-tauri/tauri.conf.json` | `"version": "1.0.16"` |

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
chore: versión 1.0.16 — versión real de la app, backups de la BD (Fase 8: automático, cifrado, restauración) y documentación al día
# (ajustar el resumen a lo que incluya la release al bumpear)
```

## 5. Publicar: push + tag

```bash
git push origin main
git tag v1.0.16
git push origin v1.0.16
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

- [ ] Release `v1.0.16` existe en <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.16>
      con **5 assets**: los 2 instaladores (`DinamoRent_1.0.16_x64-setup.exe` NSIS ~21 MB y
      `DinamoRent_1.0.16_x64_en-US.msi` ~33 MB), sus firmas del updater (`*.exe.sig` / `*.msi.sig`)
      y `latest.json`. Los `.sig` son de **minisign** (verificación del updater), NO firma de
      código Authenticode.
- [ ] `latest.json` existe y `platforms.windows-x86_64.url` apunta al instalador de esta
      release (el CI elige cuál sube al publicar — en la v1.0.3 fue el `.msi`) — es lo que la
      app instalada (v1.0.3+) consulta al arrancar para auto-actualizarse. En la v1.0.16 la app
      instalada debe DETECTAR la release nueva y ofrecer instalarla (prueba de campo del
      auto-update).
- [ ] **E2E del auto-update — validación sin máquina** (antes de la prueba de campo):
      desde un árbol dev **aún en v1.0.15** (antes del bump local o desde el commit previo),
      verificar que la release publicada se detecta, su firma valida y los bytes coinciden,
      con el binario de desarrollo `updater_e2e`:

      ```bash
      cd src-tauri && cargo run --features dev --bin updater_e2e -- \
        --endpoint https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/latest/download/latest.json \
        --expect-version 1.0.16 \
        --expect-file ./DinamoRent_1.0.16_x64-setup.exe
      ```

      Debe terminar con `[OK]` (check() detecta v1.0.16, firma minisign verificada contra
      la pubkey de producción y bytes idénticos al instalador).
- [ ] **E2E del auto-update en máquina real** (la contraparte de campo del test de
      paginación de `release.yml`, que sí corre en CI): en un equipo con una **v1.0.14+
      instalada** (p. ej. la v1.0.15), abrir la app y confirmar que **detecta la v1.0.16**
      («Actualización disponible»), descarga con verificación de firma minisign, instala
      y **reinicia en la v1.0.16** — verificar la versión resultante (menú lateral / login
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
Get-FileHash .\DinamoRent_1.0.16_x64-setup.exe -Algorithm SHA256
# comparar contra el sha256 publicado por GitHub en la página de la release
```

- [ ] (Opcional) `scripts/verificar-despliegue.ps1` en un equipo de prueba → VEREDICTO OK
      (sin `-DryRun`: chequeos reales sobre la instalación; el modo `-DryRun` se valida
      en `ci.yml` y en el §3, no requiere máquina).

## 7. Actualizar la operación

Si el bump cambió algo de operación (p. ej. el check de versión del exe):

- [ ] `scripts/verificar-despliegue.ps1` — `Check "Version 1.0.16" ($ver -like '1.0.16*')`.
- [ ] `DEPLOYMENT_CLIENTES.md` — versión esperada e instaladores en la tabla de verificación.
- [ ] `RESUMEN_EJECUTIVO.md` — versión estable, assets, conteos.
- [ ] `Handsoff.md` — cabecera y nota de portada de la release nueva.
- [ ] Commitear estos ajustes y empujar.

## 8. Anunciar

- [ ] Marcar releases anteriores si aplica (la v1.0.15 pasa a "estable anterior"; la v1.0.9 ya lo es — no se descontinúa salvo motivo).
- [ ] Pegar el mensaje de `ANUNCIO_RELEASE_TEMPLATE.md` (versión larga o corta) en Slack/Teams
      con los enlaces de descarga y el resumen de la release.

---

## Checklist exprés (resumen)

```
[ ] CI verde en main (incluye verificar-despliegue.ps1 -DryRun)
[ ] Bump en package.json + Cargo.toml + tauri.conf.json (idénticos, 1.0.16)
[ ] Docs de descarga actualizadas (INSTALACION_OPERACIONES.md, README.md, ANUNCIO)
[ ] commit chore: versión 1.0.16
[ ] git push origin main && git push origin v1.0.16
[ ] Release publicada por CI con changelog y 5 assets (NSIS + MSI + .sig x2 + latest.json)
[ ] sha256 verificado contra el publicado
[ ] verificar-despliegue.ps1 → OK (equipo de prueba)
[ ] E2E auto-update en máquina real → v1.0.16 detectada, instalada y reiniciada
[ ] Docs de operación al día (ps1, DEPLOYMENT_CLIENTES, RESUMEN, Handsoff)
[ ] Anuncio en Slack/Teams
```
