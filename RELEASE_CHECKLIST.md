# Checklist de publicación de release — Dinamo Rent ERP

> Procedimiento operativo para publicar una versión nueva (v1.0.3 en adelante)
> en `github.com/CORJAR-Computers/dinamo_rent_tr`. Cubre el bump de versión,
> el tag que dispara el CI, la verificación de los assets y el anuncio.
> Complementa a `INSTALACION_OPERACIONES.md` (instalación), `DEPLOYMENT_CLIENTES.md`
> (despliegue a clientes) y `ANUNCIO_RELEASE_TEMPLATE.md` (mensajes de anuncio).

---

## 0. Regla de oro

**El tag `vX.Y.Z` debe apuntar a un commit donde los tres archivos de versión
ya estén bumpeados.** El CI (`release.yml`) compila el código del commit del tag
y los instaladores se nombran con la versión de `src-tauri/tauri.conf.json`,
NO con el nombre del tag. Un tag sobre un commit sin bumpear publicaría una
release `v1.0.3` con instaladores `DinamoRent_1.0.2_*`.

---

## 1. Pre-requisitos

- [ ] `ci.yml` verde en el tope de `main` (lint, svelte-check, vitest, cargo test --lib, importador).
      El workflow de release NO valida: un tag sobre un commit roto publicaría igual.
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
| `package.json` | `"version": "1.0.3"` |
| `src-tauri/Cargo.toml` | `version = "1.0.3"` (crate `dinamo-rent`) |
| `src-tauri/tauri.conf.json` | `"version": "1.0.3"` |

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

## 4. Commit del bump

Mensaje con el estilo del repo (español, prefijo `chore:`):

```text
chore: versión 1.0.3 — <resumen corto de los cambios>
```

## 5. Publicar: push + tag

```bash
git push origin main
git tag v1.0.3
git push origin v1.0.3
```

El push del tag dispara `release.yml` (GitHub Actions, `windows-latest`):
`checkout` (fetch-depth 0) → changelog automático → `tauri build` (NSIS + MSI) →
crea la release **publicada** (no draft) y sube los assets. ~10 minutos (referencia v1.0.2: 11 min · v1.0.3: ~10 min).

> El body de la release se genera solo: lista los commits entre el tag anterior
> y el nuevo, con hash corto y mensaje. Si quieres verlo antes de publicar,
> cambia `releaseDraft: true` en `release.yml` y publícala a mano.

> **Auto-actualización (v1.0.3+):** la app instalada comprueba al arrancar si hay una
> release más nueva (endpoint `latest.json` de GitHub) y pide permiso para instalarla.
> Las instalaciones **v1.0.2 no tienen updater**: se actualizan UNA vez a mano con el
> instalador de la v1.0.3 y de ahí en adelante reciben las siguientes automáticamente.

## 6. Verificar la release (no confiar a ciegas en el CI)

- [ ] Release `v1.0.3` existe en <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.3>
      con **4+ assets**: los 2 instaladores (`DinamoRent_1.0.3_x64-setup.exe` NSIS ~21 MB y
      `DinamoRent_1.0.3_x64_en-US.msi` ~31 MB), sus firmas del updater (`*.exe.sig` / `*.msi.sig`)
      y `latest.json`. Los `.sig` son de **minisign** (verificación del updater), NO firma de
      código Authenticode.
- [ ] `latest.json` existe y `platforms.windows-x86_64.url` apunta al instalador de esta
      release (el CI elige cuál sube al publicar — en la v1.0.3 fue el `.msi`) — es lo que la
      app instalada (v1.0.3+) consulta al arrancar para auto-actualizarse.
- [ ] El **body contiene el changelog** (commits del rango).
- [ ] Los enlaces responden HTTP 200 y el tamaño coincide:

```powershell
# En el PC objetivo
Get-FileHash .\DinamoRent_1.0.3_x64-setup.exe -Algorithm SHA256
# comparar contra el sha256 publicado por GitHub en la página de la release
```

- [ ] (Opcional) `scripts/verificar-despliegue.ps1` en un equipo de prueba → VEREDICTO OK.

## 7. Actualizar la operación

Si el bump cambió algo de operación (p. ej. el check de versión del exe):

- [ ] `scripts/verificar-despliegue.ps1` — `Check "Version 1.0.3" ($ver -like '1.0.3*')`.
- [ ] `DEPLOYMENT_CLIENTES.md` — versión esperada e instaladores en la tabla de verificación.
- [ ] `RESUMEN_EJECUTIVO.md` — versión estable, assets, conteos.
- [ ] `Handsoff.md` — cabecera y nota de portada de la release nueva.
- [ ] Commitear estos ajustes y empujar.

## 8. Anunciar

- [ ] Marcar releases anteriores si aplica (la v1.0.2 sigue siendo "estable anterior", no se descontinúa salvo motivo).
- [ ] Pegar el mensaje de `ANUNCIO_RELEASE_TEMPLATE.md` (versión larga o corta) en Slack/Teams
      con los enlaces de descarga y el resumen de la release.

---

## Checklist exprés (resumen)

```
[ ] CI verde en main
[ ] Bump en package.json + Cargo.toml + tauri.conf.json (idénticos)
[ ] Docs de descarga actualizadas (INSTALACION_OPERACIONES.md, README.md, ANUNCIO)
[ ] commit chore: versión X.Y.Z
[ ] git push origin main && git push origin vX.Y.Z
[ ] Release publicada por CI con changelog y 2 assets (NSIS + MSI)
[ ] sha256 verificado contra el publicado
[ ] verificar-despliegue.ps1 → OK (equipo de prueba)
[ ] Docs de operación al día (ps1, DEPLOYMENT_CLIENTES, RESUMEN, Handsoff)
[ ] Anuncio en Slack/Teams
```
