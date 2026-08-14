# Plan de despliegue en equipos de clientes — Dinamo Rent v1.0.2

> Procedimiento operativo para instalar la **v1.0.2** (única versión estable) en los
> equipos de los clientes: instalación silenciosa, verificación post-instalación y
> rollback. Complementa a `INSTALACION_OPERACIONES.md` (que tiene los enlaces de descarga
> y las credenciales iniciales).

---

## 0. Reglas de oro

1. **Siempre la v1.0.2** — la v1.0.0 está descontinuada (falla en instalaciones nuevas).
2. **Los datos viven en `%APPDATA%\com.corjar.dinamorent\`**, NO en la carpeta de
   programa. Nunca borrar esa carpeta: es la BD del cliente.
3. La instalación **no requiere desinstalar** la versión anterior ni borrar nada
   previamente — el instalador reemplaza la app y el arranque migra la BD.
4. **Credenciales iniciales** (solo instalación nueva): `admin` / `admin123`, con cambio
   forzado en el primer ingreso.

---

## 1. Inventario previo (por cada equipo)

| Dato | Cómo obtenerlo |
|---|---|
| Versión de Windows (debe ser x64, 10 1803+ / 11) | `winver` o `systeminfo` |
| ¿Versión anterior instalada? (v1.0.0) | `Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*' , 'HKLM:\SOFTWARE\WOW6432Node\...' | Where-Object DisplayName -like '*Dinamo*'` |
| ¿BD existente? (`%APPDATA%\com.corjar.dinamorent\dinamo_rent_v3.fdb`) | `Test-Path "$env:APPDATA\com.corjar.dinamorent\dinamo_rent_v3.fdb"` |
| ¿Backup reciente de la BD? | Crearlo antes de tocar nada (ver §4) |

> Si el equipo **ya tiene la v1.0.0 con datos**: no hay nada especial — instalar la
> v1.0.2 encima y verificar. Solo hay que confirmar el backup antes.

---

## 2. Instalación silenciosa

### Equipo a equipo (PowerShell, como usuario con derechos de instalación)

```powershell
# NSIS — silenciosa total (sin atajos, sin ejecutar al final)
& "D:\deploy\DinamoRent_1.0.2_x64-setup.exe" /S
# Esperar a que termine (NSIS /S es síncrono al esperar al proceso)
# Start-Process -Wait -FilePath "D:\deploy\DinamoRent_1.0.2_x64-setup.exe" -ArgumentList "/S"
```

```powershell
# MSI — para GPO / Intune / SCCM
msiexec /i "D:\deploy\DinamoRent_1.0.2_x64_en-US.msi" /qn /norestart
```

> **WebView2**: si el equipo no lo tiene, el instalador lo descarga e instala
> automáticamente (requiere internet). En equipos sin internet, instalar WebView2
> Runtime primero desde
> <https://developer.microsoft.com/en-us/microsoft-edge/webview2/>.

### Lote (varios equipos, opcional)

Guardar los instaladores en un share (`\\srv\deploy\`), dar permisos de lectura a los
equipos y ejecutar con una herramienta de gestión (Intune, SCCM, GPO `msi` + `cmd`, o
`psexec`):

```powershell
# Ejemplo con psexec (máquina de operaciones):
psexec \\PC-CLIENTE-01 -s -d "D:\deploy\DinamoRent_1.0.2_x64-setup.exe" /S
```

---

## 3. Verificación post-instalación

> Script listo: **`scripts/verificar-despliegue.ps1`** — corre en el equipo objetivo,
> escribe un veredicto (OK / FALLOS) y una lista de comprobaciones. Ejecutar como
> usuario normal:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verificar-despliegue.ps1
```

### Lo que comprueba (equivalente manual)

| # | Comprobación | Esperado |
|---|---|---|
| 1 | Exe instalado (`%LOCALAPPDATA%\DinamoRent\dinamo-rent.exe`) | existe, versión **1.0.2** |
| 2 | Arranque: proceso vivo a los 10 s | **no** se cuelga ni muere (el bug del v1.0.0) |
| 3 | `%APPDATA%\com.corjar.dinamorent\` | existe (la crea el **primer arranque**; por eso se comprueba después del arranque) |
| 4 | `config.ini` | existe |
| 5 | `dinamo_rent_v3.fdb` | existe y pesa > 0 (BD creada o migrada) |
| 6 | Migraciones: `schema_migrations` tiene 19 versiones | 19 (comprobación opcional con tooling dev) |
| 7 | Login manual | `admin` + contraseña del cliente (primer ingreso: cambio forzado) |

### Si algo falla

| Síntoma | Acción |
|---|---|
| Exe no aparece / versión no es 1.0.2 | Reinstalar (¿el instalador correcto? ¿se descargó una versión anterior?) |
| `config.ini` pero NO la BD | No borrar nada: reinstalar la v1.0.2 (el arranque crea la BD). Si persiste, revisar exclusión de Defender sobre la carpeta |
| Proceso muere en <10 s | Capturar Event Log de Aplicación (módulo con errores) y volcar aquí |
| La BD existente "no abre" | Nunca borrar la carpeta. Restaurar el backup (ver §4) y reinstalar |

---

## 4. Backup y rollback

### 4.1 Backup de la BD (antes de cualquier despliegue sobre equipo con datos)

```powershell
# Copia del archivo (Firebird Embedded: copiar solo con la app cerrada)
Stop-Process -Name dinamo-rent -ErrorAction SilentlyContinue
Copy-Item "$env:APPDATA\com.corjar.dinamorent\dinamo_rent_v3.fdb" "D:\backups\dinamo_$(Get-Date -Format yyyyMMdd_HHmmss).fdb"
```

> **Importante**: copiar el `.fdb` **con la app cerrada** (Firebird Embedded usa WAL y
> una copia en caliente puede quedar inconsistente). Alternativa robusta: usar `gbak`
> del runtime de Firebird (`firebird\gbak.exe` en la carpeta de instalación) para un
> backup consistente:
> `"$env:LOCALAPPDATA\DinamoRent\firebird\gbak.exe" -b -user SYSDBA -password <pass> "$env:APPDATA\com.corjar.dinamorent\dinamo_rent_v3.fdb" "D:\backups\dinamo_$(Get-Date -Format yyyyMMdd_HHmmss).fbk"`

### 4.2 Rollback (volver a una versión anterior o recuperarse)

1. **Cerrar la app** (`Stop-Process -Name dinamo-rent`).
2. **Restaurar la BD** desde el backup (reemplazar `dinamo_rent_v3.fdb`).
3. **Reinstalar la versión deseada** (desinstalar e instalar, o instalar encima).
4. Arrancar y verificar login + datos.

> En la práctica, con la v1.0.2 **no debería hacer falta un rollback de BD**: el arranque
> es idempotente y las migraciones se auto-reparan. El rollback real es solo para
> emergencias (p. ej. corrupción del archivo, nunca por el fix de instalación limpia).

---

## 5. Checklist de despliegue (por equipo)

```
[ ] Backup de la BD creado (si el equipo tiene datos)
[ ] Instalador v1.0.2 descargado (verificar hash/tamaño ~21 MB)
[ ] Instalación silenciosa OK (código 0)
[ ] scripts\verificar-despliegue.ps1 → VEREDICTO: OK
[ ] Login con el usuario del cliente (no admin123 salvo primer ingreso)
[ ] Flota / clientes / rentas visibles (datos correctos)
[ ] Agente SIMIT operativo (si aplica)
[ ] Credenciales iniciales registradas y contraseña rotada si era admin123
```
