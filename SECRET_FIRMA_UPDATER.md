# Secret de firma del auto-update (TAURI_SIGNING_PRIVATE_KEY)

La clave privada que firma los instaladores para el auto-update vive **solo** en la
máquina que la generó (`~/.tauri/dinamorent.key`). Para que el CI (`release.yml`)
firme los bundles y suba los `.sig` + `latest.json` hay que copiarla a GitHub como
secret — **nunca se commitea al repo**.

## 1. Requisitos

- `gh` CLI instalado y autenticado con acceso **admin** al repo
  (los secrets de Actions requieren `repo` + admin en el repo):
  ```bash
  gh auth login
  ```
  En una máquina sin `gh auth`, se puede usar el token del Git Credential Manager:
  ```bash
  GH_TOKEN="$(printf 'protocol=https\nhost=github.com\n' | git credential fill | sed -n 's/^password=//p')"
  ```
  (El token debe pertenecer a un usuario con admin sobre el repo; verifícalo antes de continuar.)
- La clave existe: `ls ~/.tauri/dinamorent.key`
- Permiso de verificación: `gh repo view CORJAR-Computers/dinamo_rent_tr` responde OK.

## 2. Configurar el secret

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY \
  --repo CORJAR-Computers/dinamo_rent_tr \
  --body "$(cat ~/.tauri/dinamorent.key)"
```

> En Git Bash `~` se expande a `C:\Users\<usuario>`; si `$HOME` no se resuelve
> (rutas MSYS), usa la ruta Windows: `--body "$(cat 'C:\Users\TU_USUARIO\.tauri\dinamorent.key')"`.

**Solo si la clave tiene password** (este proyecto la generó sin password — no hace
falta): además, configurar `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

El valor del secret **no aparece en ningún output** de `gh` — si en algún momento se
imprime la clave, el secret quedó mal configurado.

## 3. Verificar que quedó activo

```bash
# El nombre debe aparecer en la lista (el valor nunca se muestra):
gh secret list --repo CORJAR-Computers/dinamo_rent_tr

# Confirmación por API de que existe (200 = OK, 404 = no existe):
gh api repos/CORJAR-Computers/dinamo_rent_tr/actions/secrets/TAURI_SIGNING_PRIVATE_KEY \
  --jq '{name: .name, updated_at: .updated_at}'
```

Salida esperada:

```json
{ "name": "TAURI_SIGNING_PRIVATE_KEY", "updated_at": "2026-08-14T..." }
```

La prueba definitiva de que el CI lo usa es la propia release: al publicar el tag,
la release debe incluir los **`.sig`** y el **`latest.json`** (si faltan, el secret
no estaba configurado al correr el workflow). Localmente, `scripts/verificar-updater-e2e.sh`
valida que la clave local y la pubkey de `tauri.conf.json` coinciden.

## 4. Respaldo y rotación

- **Respaldar** `~/.tauri/dinamorent.key` fuera del repo (ej. gestor de contraseñas).
  Si se pierde, las instalaciones v1.0.3+ dejarían de poder actualizarse.
- Si se rota la clave: regenerar el par (`bunx tauri signer generate`), actualizar el
  secret **y** la pubkey en `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`), y
  publicar una release nueva — las instalaciones que ya tienen la pubkey anterior
  dejarán de validar las firmas hasta actualizarse.
