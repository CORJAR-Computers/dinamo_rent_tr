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

### 1.5 Ubicación de la clave PII y respaldo obligatorio (verificado 2026-08-11)

La clave `db_encryption_key` **no se copia ni se pega manualmente**: la aplicación la persiste y
la lee automáticamente desde `config.ini` (sección `[security]`) en cada arranque, y la mantiene
en memoria (`AppState.pii_key`). Ubicaciones actuales de la clave:

| Entorno | Archivo | Estado |
|---|---|---|
| Desarrollo | `<repo>/data/config.ini` → `[security] db_encryption_key` | Configurada (10-08-2026) |
| Producción | `%APPDATA%\com.corjar.dinamorent\config.ini` → `[security] db_encryption_key` | Configurada (10-08-2026) |

> ℹ️ La ruta de producción deriva del `identifier` de Tauri (`com.corjar.dinamorent`, ver
> `src-tauri/tauri.conf.json`). Si se cambia el identifier, la app leería un `config.ini` nuevo
> **sin** la clave y los datos PII quedarían ocultos — actualizar la ruta en esta tabla.

**Respaldo obligatorio del operador**: guardar el valor de la clave en un **gestor de contraseñas**
(Windows Credential Manager, Bitwarden, KeePass, etc.). Es la única copia fuera de esta máquina:
si ambos `config.ini` se pierden o corrompen, los datos PII cifrados (AES-256-GCM, formato `v1:`)
son **irrecuperables** — no existe mecanismo de recuperación ni puerta trasera.

⚠️ **NUNCA** escribir el valor de la clave en este repositorio (este archivo está commiteado — ver
§4, incidente de clave expuesta), ni en logs, issues, capturas ni mensajes.

**Verificación (2026-08-11)**: con la clave almacenada en los `config.ini` anteriores, un
diagnóstico de solo lectura contra ambas BD (`data/dinamo_rent_v3.fdb` y
`%APPDATA%\com.corjar.dinamorent\dinamo_rent_v3.fdb`) confirmó que **los 42 clientes / 238 tokens
PII descifran al 100% con esa clave** en cada BD (cero campos ocultos, cero tokens Fernet legacy
pendientes). La clave se carga en memoria al arrancar: tras cambiar o reconfigurar la clave,
**reiniciar la app** para que el estado en caliente se actualice.

---

## 2. Rotación de clave PII (`db_encryption_key`)

La rotación debe realizarse:

- Al menos **una vez al año** (política por defecto).
- **Inmediatamente** tras sospecha de compromiso (ver §4).
- Tras cambios de personal con acceso administrativo.

### 2.1 Procedimiento de rotación

La rotación es una operación **destructiva si se hace mal**. Probar en entorno staging primero y **siempre** tener un backup cifrado de la BD antes de comenzar.

#### Paso 0 — Backup

Con la app **detenida**, respaldar `config.ini` (contiene la clave PII) y la BD `.fdb` de los
despliegues dev y producción con el script del repo. Copia ambos archivos en
`data/Backups/pre-rotacion/<fecha-hora>/` con checksums sha256:

```bash
bash scripts/backup-antes-rotacion.sh
# Opciones: --dev-only / --prod-only / --dest <carpeta> / --force (si la app está abierta)
```

Equivalente manual mínimo (solo la BD de desarrollo):

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

La rotación implica **descifrar cada fila PII con la clave vieja y re-cifrarla con la nueva**. El repo incluye el bin de mantenimiento `rotate_pii_key` (`src-tauri/src/bin/rotate_pii_key.rs`), que re-cifra las columnas PII de `clientes` (Fernet legacy y AES-GCM `v1:` → AES-GCM `v1:`) en una transacción y aborta si la clave vieja no descifra algún token. El bin descifra los tokens `v1:` con la clave vieja (no los trata como texto plano) — la regresión del 2026-08-11 que re-cifraba tokens ya cifrados quedó corregida en `services/rotacion.rs` (ver §5.4):

```bash
# Detener la aplicación primero (evita escrituras concurrentes).
cargo run --features dev --bin rotate_pii_key -- \
  --old-key "CLAVE_VIEJA" \
  --new-key "$NEW_KEY" \
  --db "ruta/al/dinamo_rent_v3.fdb"
```

> ⚠️ Hacer backup previo (§2.1 Paso 0) y ejecutar UNA vez por cada instalación
> (dev y producción), cada una con su `--db`. El bin solo se construye con
> `--features dev` para no llegar al bundle de release.

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
4. Revisar la tabla `auditoria` de la BD (vista Auditoría de la app o consulta SQL directa) — el bin
   `rotate_pii_key` registra el evento `PII_KEY_ROTATED` (usuario `sistema`, ip `local`) en la misma
   transacción que la re-cifra, **sin exponer la clave** en el mensaje.
5. **Verificar que NO quedó doble cifrado** (lección del 2026-08-11, §5): después de la re-cifra,
   ningún token `v1:` debe quedar anidado dentro de otro. **Política automatizada** — ejecutar el
   script de verificación, que corre el dry-run de normalización y valida la auditoría:

   ```bash
   bash scripts/verificar-rotacion.sh                      # dev + producción
   bash scripts/verificar-rotacion.sh --dev-only           # solo desarrollo
   bash scripts/verificar-rotacion.sh --exige-evento-rotacion   # además exige PII_KEY_ROTATED en auditoria
   ```

   El script devuelve **exit 0 solo si** el dry-run reporta **"0 a normalizar"** y **0 indescifrables**
   (y, con `--exige-evento-rotacion`, existe al menos un evento `PII_KEY_ROTATED` en la BD — incluye
   los registrados retroactivamente). Si falla (**exit ≠ 0**), la rotación habría vuelto a duplicar
   capas (regresión del binario): detener la app y recuperar con §5.3 antes de continuar. Ver §5.2
   para las señales adicionales de cifrado anidado.

   > ⚠️ Nota: el evento `PII_KEY_ROTATED` se registra en cada BD **a partir de la rotación ejecutada
   > con el binario corregido** (2026-08-11+). En la BD dev existe también el retroactivo del 10-08;
   > en **producción aún no hay ninguno**, por lo que `--exige-evento-rotacion` sin `--dev-only`
   > fallará en prod hasta que esa BD tenga su evento (o se inserte uno retroactivo).

   Alternativa manual equivalente:

   ```bash
   python scripts/normalizar_doble_cifrado.py   # reporte: "campos PII con cifrado anidado a normalizar: 0"
   ```

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

### 4.4 Mitigación aplicada posteriormente (2026-08-10)

6. ✅ **Historial Git purgado** (2026-08-10) con `git filter-repo` (invert-paths + replace-text) y force-push.
7. ✅ **Clave PII rotada** (2026-08-10): nueva clave generada con `openssl rand -base64 32`, los 42 clientes re-cifrados (Fernet → AES-GCM v1:) en las BDs de dev y producción con `rotate_pii_key`, y `db_encryption_key` actualizada en ambos `config.ini` (dev y `%APPDATA%`). Verificación OK: la app lista los clientes sin PII oculto y sin errores.

> ⚠️ **Esta verificación resultó insuficiente** (ver §5): el binario usado re-cifraba de nuevo los
> tokens `v1:` ya cifrados (doble capa con la misma clave). El defecto se detectó el 2026-08-11
> porque la app mostraba los tokens en el modal de edición; la BD dev se normalizó entonces
> (sección §5). La BD de producción no resultó afectada.

### 4.5 Pendientes del operador

1. ⚠️ **Cambiar la contraseña `sysdba`** de Firebird si existe cualquier despliegue en modo server (en embedded no aplica).
2. ⚠️ **Revisar logs de auditoría** en busca de accesos sospechosos posteriores a la fecha de exposición (2026-08-09).
3. ⚠️ Si la BD contiene datos reales de clientes y hubo acceso no autorizado al repo, considerar **notificación a la SIC** (Superintendencia de Industria y Comercio de Colombia) bajo la Ley 1581.

### 4.6 Fecha del hallazgo

- Detectado: 2026-08-09 (Grupo A — Saneamiento del repo).
- Estado: mitigación técnica + rotación de clave + purga de historial **COMPLETADOS** (2026-08-10).

---

## 5. Lección aprendida — doble cifrado por rotación con binario defectuoso (2026-08-11)

### 5.1 Qué pasó

La rotación de clave del 2026-08-10 (§4.4) se ejecutó con una versión del binario
`rotate_pii_key` que **solo descifraba tokens Fernet legacy** (`gAAAA...`) y trataba los tokens
AES-GCM `v1:` como si fueran texto en claro, **re-cifrándolos de nuevo** (doble capa con la MISMA
clave). La clave nueva quedó en `config.ini`, de modo que:

- **Capa externa** (`v1:...`) → descifra con la clave actual → produce...
- **Capa interna** (`v1:g7kgKY18aDrKGnDe:...`) → también descifra con la clave actual → produce
  el texto real (teléfono, correo, dirección).

La app descifraba una sola capa y mostraba la capa interna como si fuera el dato: el modal
**Editar cliente** exhibía tokens `v1:...` en lugar del celular/correo/dirección. Fue un fallo de
calidad del binario (no pérdida de clave ni compromiso): los datos **no se perdieron**.

> ⚠️ Si el doble cifrado hubiera usado **dos claves distintas** (p. ej. una rotación con bug
> ejecutada dos veces con claves diferentes), la recuperación requeriría AMBAS claves y el dato
> sería irrecuperable si la intermedia se perdiera. Por eso la pre-validación del bin aborta
> antes de escribir si algún token no descifra con la clave vieja (§2.1 Paso 2).

> ℹ️ La normalización real de la BD dev ejecutada el **2026-08-11** se hizo **antes** de que el
> script registrara el evento de auditoría, por lo que esa operación concreta no aparece en la
> tabla `auditoria`. El evento `PII_NORMALIZADA` queda registrado a partir de esa mejora, en
> futuras normalizaciones.

### 5.2 Cómo detectarlo

| Señal | Dónde | Qué indica |
|---|---|---|
| Campos PII con texto `v1:...` en la UI | Modal Editar cliente / tabla Clientes | La app descifró 1 capa y mostró la capa interna (token anidado) |
| Descifrado que devuelve otro token `v1:` o `gAAAA` | Diagnóstico SQL / script | El valor almacenado tiene más de una capa de cifrado |
| Dry-run del script de normalización > 0 | `scripts/normalizar_doble_cifrado.py` | Existen campos con cifrado anidado que necesitan normalización |

Verificación de lectura directa (sin tocar la BD) con la clave de `config.ini`:

```bash
python scripts/normalizar_doble_cifrado.py            # dry-run: reporta "a normalizar"
python scripts/normalizar_doble_cifrado.py --commit   # solo si el dry-run encontró > 0 campos
```

### 5.3 Cómo recuperarlo

1. **Respaldo previo** con la app detenida (§2.1 Paso 0): `bash scripts/backup-antes-rotacion.sh`.
2. **Dry-run** para confirmar el alcance: `python scripts/normalizar_doble_cifrado.py` — debe
   reportar los campos anidados y **cero indescifrables**.
3. **Normalizar**: `python scripts/normalizar_doble_cifrado.py --commit` — des-envuelve todas las
   capas con la clave actual y re-cifra **una sola vez** (formato sano). Registra el evento de
   auditoría `PII_NORMALIZADA` (usuario `sistema`, ip `local`, mensaje con conteos **sin exponer
   la clave**) en la misma transacción.
4. **Verificar**: re-ejecutar el dry-run (debe quedar en **0**) y comprobar que los clientes se ven
   descifrados en la app.
5. **Destruir** la copia de trabajo y el backup pre-normalización cuando la app lleve **≥72h**
   estable sin incidentes (análogo a §2.1 Paso 6).

### 5.4 Blindaje aplicado en el código

1. ✅ `PiiCipher::decrypt` des-envuelve **todas** las capas `v1:`/Fernet de forma iterativa hasta
   el texto en claro (límite de seguridad `MAX_CAPAS_CIFRADO = 8`). Aunque se vuelva a producir
   un cifrado anidado accidental, la vista seguirá mostrando los datos correctos.
2. ✅ `rotate_pii_key` / `services/rotacion.rs` descifran tokens `v1:` y Fernet con la clave vieja
   (no los tratan como texto plano) y pre-validan toda la tabla antes de escribir.
3. ✅ Tests de regresión en `core/crypto.rs`: doble capa, triple capa, Fernet-tras-v1 y límite de
   capas; y `tests/rotacion_integration.rs` cubre la rotación y el evento `PII_KEY_ROTATED`.
4. ✅ Script `scripts/normalizar_doble_cifrado.py` (reparación) con dry-run, transacción atómica
   y auditoría `PII_NORMALIZADA`.

---

## 6. Referencias

- `.gitignore` (raíz del repo)
- `data/config.ini.example`
- `.env.example`
- `THIRD_PARTY_LICENSES.md`
- `scripts/sanitize-repo.sh`
- `scripts/backup-antes-rotacion.sh`
- `scripts/verificar-rotacion.sh`
- `scripts/normalizar_doble_cifrado.py`
- `src-tauri/src/services/rotacion.rs`
- `worklog.md` (bitácora de análisis — sección Grupo A)
