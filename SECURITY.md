# Seguridad — Dinamo Rent ERP

Este documento describe las políticas de manejo de secretos, rotación de claves y reporte de vulnerabilidades para el proyecto **Dinamo Rent ERP** (Tauri V2 + Rust + SvelteKit + Firebird Embedded).

---

## 1. Manejo de secretos

### 1.1 Archivo `data/config.ini`

El archivo `data/config.ini` contiene parámetros operativos y, en particular, secretos sensibles:

- `database.password` — contraseña del usuario `sysdba` de Firebird (en modo embedded se puede dejar vacía, pero si se migra a server debe ser una contraseña fuerte).
- `security.db_encryption_key` — clave AES-256-GCM (base64 de 32 bytes) usada para cifrar PII de clientes en reposo.

⚠️ **`data/config.ini` está en `.gitignore` y NUNCA debe commitearse.** El archivo commiteado es `data/config.ini.example` con valores placeholder.

### 1.2 Generación de la clave PII

La clave `db_encryption_key` debe ser **única por despliegue**, generada criptográficamente con:

```bash
openssl rand -base64 32
```

Alternativas equivalentes:

```bash
# Python
python3 -c "import secrets,base64; print(base64.b64encode(secrets.token_bytes(32)).decode())"

# Node.js
node -e "console.log(require('crypto').randomBytes(32).toString('base64'))"
```

### 1.3 Almacenamiento recomendado

Por orden de preferencia:

1. **OS Keyring** (recomendado para producción): Windows Credential Manager, macOS Keychain, Linux Secret Service. La aplicación la lee en runtime y nunca toca disco.
2. **Variable de entorno** `DINAMO_DB_ENCRYPTION_KEY` inyectada por el launcher del sistema operativo (Systemd, supervisor, o el instalador NSIS mediante `[Environment]::SetEnvironmentVariable`).
3. **`data/config.ini` local** (solo para desarrollo): con permisos `0600`, propiedad del usuario que ejecuta la app, fuera del control de versiones.

❌ **NUNCA** en código fuente, archivos commiteados, logs, capturas de pantalla, ni issues/PRs.

### 1.4 Override por variable de entorno

El backend Rust permite override del `config.ini` mediante variables de entorno (ver `.env.example`):

- `DINAMO_DB_ENCRYPTION_KEY` — reemplaza `security.db_encryption_key`.
- `DINAMO_FB_USER` / `DINAMO_FB_PASSWORD` — reemplazan `database.user` / `database.password`.

Esto facilita despliegues en CI o servidores donde no se desea escribir `config.ini`.

---

## 2. Rotación de clave PII (`db_encryption_key`)

La rotación debe realizarse:

- Al menos **una vez al año** (política por defecto).
- **Inmediatamente** tras sospecha de compromiso (ver §4).
- Tras cambios de personal con acceso administrativo.

### 2.1 Procedimiento de rotación

La rotación es una operación **destructiva si se hace mal**. Probar en entorno staging primero y **siempre** tener un backup cifrado de la BD antes de comenzar.

#### Paso 0 — Backup

```bash
# Asumiendo que la app está detenida
cp data/dinamo_rent_v3.fdb data/dinamo_rent_v3.fdb.pre-rotation.bak
# (el .bak está ignorado en .git)
```

#### Paso 1 — Generar nueva clave

```bash
NEW_KEY=$(openssl rand -base64 32)
echo "Nueva clave: $NEW_KEY"
# Guardar temporalmente en archivo seguro con permisos 0600
echo "$NEW_KEY" > /tmp/new_key.txt
chmod 600 /tmp/new_key.txt
```

#### Paso 2 — Script de rotación

La rotación implica **descifrar cada fila PII con la clave vieja y re-cifrarla con la nueva**. El backend expone el comando Tauri interno `rotate_pii_key(old_key, new_key)` (ver `src-tauri/src/services/security.rs`), o puede ejecutarse vía CLI de mantenimiento:

```bash
# Detener la aplicación primero.
# Ejecutar la rotación (la app debe estar detenida para evitar escrituras concurrentes):
cargo run --bin dinamo-rotate-keys -- \
  --old-key "$(cat data/.old_key)" \
  --new-key "$NEW_KEY" \
  --config data/config.ini
```

> ⚠️ Si el binario `dinamo-rotate-keys` no existe todavía en tu versión, implementar la rotación como transacción SQL:
>
> ```sql
> -- Para cada tabla con columnas cifradas (clientes, licencias, etc.):
> -- 1. UPDATE temporal descifrando con clave vieja
> -- 2. UPDATE re-cifrando con clave nueva
> -- Esto debe hacerse en bloque Rust que use aes-gcm y maneje nonces.
> ```

#### Paso 3 — Actualizar `config.ini`

Editar `data/config.ini` y reemplazar la línea:

```ini
db_encryption_key = <NUEVA_CLAVE_BASE64>
```

Verificar permisos:

```bash
chmod 600 data/config.ini
```

#### Paso 4 — Verificar

1. Arrancar la aplicación en modo `production_mode = false`.
2. Abrir un cliente existente y verificar que la cédula, teléfono y licencia se muestran correctamente (descifrado OK).
3. Crear un cliente nuevo y verificar que sus datos se guardan y re-leen.
4. Revisar logs de auditoría (`logs/audit.log`) — debe registrar el evento `PII_KEY_ROTATED` sin exponer la clave.

#### Paso 5 — Purgar la clave vieja del historial Git

Si la clave vieja estuvo commiteada en algún momento (ver §4 — incidente del análisis), debe eliminarse del historial usando `git filter-repo`:

```bash
# Instalar git-filter-repo (one-time):
pip install --user git-filter-repo

# Listar todos los commits donde aparece el archivo sensible:
git log --all --pretty=format:'%H %s' -- data/config.ini

# Opción A: eliminar el archivo completo del historial
git filter-repo --invert-paths --path data/config.ini

# Opción B: reemplazar solo el valor de la clave (más quirúrgico)
# Crear un archivo replacements.txt con el valor viejo a eliminar:
echo "REDACTED_OLD_KEY==>REDACTED_OLD_KEY" > /tmp/replacements.txt
git filter-repo --replace-text /tmp/replacements.txt

# Forzar el push a todos los remotos:
git push --force --all
git push --force --tags

# Re-clonar en máquinas de desarrollo (las referencias locales ya no son válidas).
```

⚠️ Después de `git filter-repo`, **cualquier clone o fork existente sigue teniendo la clave vieja**. Por eso **la rotación de clave es obligatoria**, no opcional: aunque purgues el historial, debes asumir que la clave vieja fue comprometida y nunca reutilizarla.

#### Paso 6 — Destruir el backup pre-rotación

Solo cuando la app lleve ≥72h funcionando con la nueva clave sin incidentes:

```bash
shred -u data/dinamo_rent_v3.fdb.pre-rotation.bak
rm -f /tmp/new_key.txt /tmp/replacements.txt
```

---

## 3. Reporte de vulnerabilidades

### 3.1 Canal de reporte

- **Email**: seguridad@dinamorent.com (placeholder — actualizar con correo real de operaciones antes de producción).
- **PGP**: el equipo de seguridad puede proporcionar una clave pública para cifrado del reporte bajo petición.
- **Respuesta**: confirmación de recepción en **≤ 48h hábiles**. Evaluación inicial y plan de mitigación en **≤ 5 días hábiles**.

### 3.2 Política

- Agradecemos reportes responsables. No iniciaremos acciones legales contra investigadores que sigan esta política.
- **No** publicar explícitamente la vulnerabilidad hasta que se publique un fix o transcurran **90 días** desde el reporte (divulgación coordinada).
- Mantendremos informado al reportante del progreso.
- Reconocimiento público en `SECURITY.md` (si el reportante lo desea) tras la mitigación.

### 3.3 Alcance

- Aplicación Dinamo Rent ERP (binarios Windows distribuidos).
- Backend Rust y repositorios (`src-tauri/src/`).
- Configuración por defecto del instalador.
- Manejo de PII de clientes.

### 3.4 Fuera de alcance

- Vulnerabilidades en dependencias de terceros (reportarlas upstream y aplicar el fix cuando esté disponible).
- Ataques que requieran acceso físico al equipo del usuario final.
- Ingeniería social al personal.

### 3.5 Modelo de severidad

Usamos CVSS v3.1. Los reportes Critical (≥9.0) y High (7.0–8.9) se priorizan con hotfix en **≤ 7 días**. Los Medium (4.0–6.9) y Low (<4.0) entran en el backlog del próximo release.

---

## 4. Incidente histórico (clave expuesta en el repositorio)

### 4.1 Descripción

Durante el análisis de seguridad del repositorio (`worklog.md`,Grupo A) se detectó que el archivo `data/config.ini` estaba commiteado y contenía:

- `database.password = masterkey` (credencial por defecto de Firebird).
- `security.db_encryption_key = REDACTED_OLD_KEY=` (clave AES-256-GCM real usada para cifrar PII de clientes).

Ambos valores están presentes en el historial Git del repositorio.

### 4.2 Impacto

Cualquiera con acceso al repositorio (incluyendo clones, forks y snapshots en servicios CI) puede:

1. Conectarse a la BD Firebird si se migra a server (con la contraseña `masterkey`).
2. **Descifrar todos los datos PII de clientes** almacenados en la BD (`clientes`, `licencias`, etc.) usando la clave expuesta.

Esto constituye un incidente **Critical** (CVSS ~9.1) bajo RGPD/Ley 1581 de Colombia si la BD contiene datos reales de clientes.

### 4.3 Mitigación aplicada (Grupo A — este PR)

1. ✅ `data/config.ini` agregado a `.gitignore` (no se commitea más).
2. ✅ Creado `data/config.ini.example` con valores placeholder.
3. ✅ Creado `.env.example` con variables de entorno opcionales.
4. ✅ Creado `scripts/sanitize-repo.sh` para `git rm --cached` y purga del historial con `git filter-repo`.
5. ✅ Esta documentación (`SECURITY.md`) describe el procedimiento completo de rotación.

### 4.4 Mitigación pendiente — ACCIÓN REQUERIDA del operador

⚠️ **El operador del sistema debe ejecutar manualmente** (no se puede automatizar):

1. **Rotar la clave PII** siguiendo el procedimiento de §2 (generar nueva clave, descifrar + re-cifrar, actualizar `config.ini`, verificar).
2. **Purgar el historial Git** con `git filter-repo` (§2.5) en todos los remotos.
3. **Forzar re-clone** en todos los entornos de desarrollo/CI.
4. **Cambiar la contraseña `sysdba`** de Firebird si existe cualquier despliegue en modo server (en embedded no aplica).
5. **Revisar logs de auditoría** en busca de accesos sospechosos posteriores a la fecha de exposición.
6. Si la BD contiene datos reales de clientes y hubo acceso no autorizado al repo, considerar **notificación a la SIC** (Superintendencia de Industria y Comercio de Colombia) bajo la Ley 1581.

### 4.5 Fecha del hallazgo

- Detectado: 2026-08-09 (Grupo A — Saneamiento del repo).
- Estado: mitigación técnica aplicada; rotación de clave y purga de historial **PENDIENTES** (requieren acción humana).

---

## 5. Referencias

- `.gitignore` (raíz del repo)
- `data/config.ini.example`
- `.env.example`
- `THIRD_PARTY_LICENSES.md`
- `scripts/sanitize-repo.sh`
- `worklog.md` (bitácora de análisis — sección Grupo A)
