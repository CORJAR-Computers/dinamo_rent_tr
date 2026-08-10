# 🖥️ Dinamo Rent ERP - Sistema de Gestión de Flota (Tauri V2)

> Sistema de gestión de flota para renta de vehículos. Administración integral: flota, clientes, rentas, reservas, finanzas, taller y más.
> **Nueva versión reescrita** utilizando Tauri V2, Rust, SvelteKit y Tailwind CSS.

---

## 📋 Configuración Rápida

### 1. Requisitos Previos
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (1.70+)
- Dependencias de sistema operativo para compilar Tauri (ver [documentación oficial](https://v2.tauri.app/start/prerequisites/)).

### 2. Instalar dependencias del Frontend
En la raíz del proyecto:
```bash
npm install
```

### 3. Ejecutar aplicación en modo desarrollo
Este comando iniciará el servidor de desarrollo del frontend (Vite) y lanzará la aplicación de escritorio de Tauri en modo debug:
```bash
npm run tauri dev
```

### 4. Compilar para producción
Para generar el instalador y el ejecutable final:
```bash
npm run tauri build
```

---

## 🏗️ Stack Tecnológico

| Capa | Tecnología |
|------|------------|
| **Frontend (UI)** | SvelteKit 2 + Svelte 5 (runes) + Tailwind CSS v4 |
| **Backend (Lógica)** | Rust (módulos `services/`) |
| **Acceso a datos** | `rsfbclient` (consultas explícitas) |
| **Base de datos** | **Firebird Embedded 5.0** (archivo portable `.fdb`) |
| **Proceso de escritorio** | Tauri V2 (WebView2 en Windows) |

---

## 📂 Estructura del Proyecto

```
Dinamo_Rent_tr/
├── data/                   # Archivos de configuración (config.ini)
├── src/                    # Frontend (SvelteKit + Tailwind)
│   ├── routes/             # Vistas de la aplicación (Dashboard, Rentas, Flota, etc.)
│   ├── lib/                # Componentes Svelte, utils de UI y estilos
│   └── app.html            # Template HTML principal
├── src-tauri/              # Backend (Rust + Tauri V2)
│   ├── src/
│   │   ├── services/       # Lógica de negocio
│   │   ├── repositories/   # Acceso a BD (rsfbclient)
│   │   └── main.rs         # Punto de entrada Tauri y registro de comandos
│   ├── Cargo.toml          # Dependencias de Rust
│   └── tauri.conf.json     # Configuración de la ventana y permisos de Tauri
├── package.json            # Dependencias Node.js / scripts
└── PLAN_IMPLEMENTACION_TAURI.md # Plan de arquitectura y migración
```

---

## 📚 Documentación

| Documento | Propósito |
|-----------|-----------|
| **[PLAN_IMPLEMENTACION_TAURI.md](PLAN_IMPLEMENTACION_TAURI.md)** | 📋 Plan completo de arquitectura y migración técnica desde Python a Tauri |
| **[Handsoff.md](Handsoff.md)** | 🤖 Registro de decisiones, automatizaciones y guías de desarrollo |

*(Para documentación histórica sobre la lógica de negocio subyacente, consultar el repositorio original de la versión Python+PySide6).*

---

## 🔒 Seguridad Implementada

- **Criptografía de Contraseñas:** Soporte para hashes antiguos (PBKDF2-SHA256) con re-hasheo automático a **Argon2id** en el primer inicio de sesión.
- **Datos Sensibles (PII):** Cifrado en reposo para datos de clientes y licencias utilizando **AES-256-GCM** gestionado desde Rust.
- **Autorización:** Control de Acceso por Roles (RBAC) aplicado de forma estricta en los Comandos de Tauri.
- **Sin Motor de BD Expuesto:** Firebird Embedded 5.0 opera dentro del mismo proceso sin puertos de red abiertos.

---

**Versión**: 4.0.0-beta (Migración Tauri V2)


---

## 🚀 Setup rápido

```bash
# 1. Configurar secrets (NUNCA commitear data/config.ini)
cp data/config.ini.example data/config.ini
# Editar data/config.ini y rellenar:
#   - database.password  -> contraseña strong de sysdba (dejar vacío en embedded)
#   - security.db_encryption_key -> generar con: openssl rand -base64 32

# 2. Generar clave PII (AES-256-GCM, base64 de 32 bytes)
openssl rand -base64 32
# Pegar el resultado en data/config.ini -> [security] db_encryption_key

# 3. Instalar dependencias frontend
bun install

# 4. Lanzar en modo desarrollo
bun run tauri dev
```

> ℹ️ Alternativamente las credenciales pueden pasarse por variables de entorno (ver `.env.example`) sin tocar `config.ini`.

---

## 🪟 Windows: exclusiones para builds sin `os error 32`

> Aplica solo en máquinas Windows con Defender / índice de búsqueda activos; en Linux/macOS no hace falta.

**Síntoma**: `error: failed to build archive at ...rlib ... (os error 32)` al compilar con cargo (típicamente en el crate `tauri`) dentro de `src-tauri\target`. La causa: Windows Defender (escaneo en tiempo real) y/o el índice de búsqueda (`SearchIndexer`) abren los archivos temporales `.tmp*.temp-archive` que `rustc` crea y borra, bloqueándolos un instante.

**Solución**: excluir la carpeta del proyecto de ambos componentes. Reemplazar `D:\dinamo_rent_tr` por la ruta real del repo en cada máquina.

### 1) Windows Defender — exclusión de ruta

GUI: **Seguridad de Windows → Protección contra virus y amenazas → Administrar la configuración → Exclusiones → Agregar o quitar exclusiones → + Agregar exclusión → Carpeta** → `D:\dinamo_rent_tr\src-tauri\target`.

O en PowerShell **como administrador**:

```powershell
Add-MpPreference -ExclusionPath 'D:\dinamo_rent_tr\src-tauri\target'
# Opcional: excluir todo el proyecto (cubre target, node_modules y la BD)
Add-MpPreference -ExclusionPath 'D:\dinamo_rent_tr'

# Verificar (requiere admin)
Get-MpPreference | Select-Object -ExpandProperty ExclusionPath
```

### 2) Windows Search (índice) — carpeta excluida

GUI: **Configuración → Privacidad y seguridad → Búsqueda en Windows → Excluir carpetas → Agregar** → `D:\dinamo_rent_tr`.

O por PowerShell (por usuario, sin admin; escribe en el crawl scope del índice):

```powershell
$base = 'HKCU:\SOFTWARE\Microsoft\Windows Search\CrawlScopeManager\Windows\DefaultGatherManager\AppScope'
$guid = [Guid]::NewGuid().ToString('B').ToUpper()
New-Item -Path "$base\$guid" -Force | Out-Null
New-ItemProperty -Path "$base\$guid" -Name 'URLOrPath' -Value 'file:///D:\dinamo_rent_tr\' -PropertyType String -Force | Out-Null
New-ItemProperty -Path "$base\$guid" -Name 'ScopeType' -Value 0 -PropertyType DWord -Force | Out-Null
New-ItemProperty -Path "$base\$guid" -Name 'Attributes' -Value 0 -PropertyType DWord -Force | Out-Null
# Reiniciar el índice para que aplique de inmediato (requiere admin):
# Restart-Service WSearch -Force
```

### 3) Verificación

Con ambas exclusiones aplicadas, el ciclo completo debe compilar sin el error:

```bash
cd src-tauri && cargo test   # build de tests + suite completa
cd .. && npm run tauri dev   # build + arranque de la app
```

> ℹ️ **Nota**: un `os error 32` en un *note* del directorio incremental (`target/debug/incremental`) es **no fatal** (cargo solo descarta ese caché y sigue); suele ser el índice tocando el directorio justo tras compilar. Para silenciarlo: `CARGO_INCREMENTAL=0 cargo build` (o `cargo test`).

---

## 🔒 Seguridad

El sistema cifra datos PII de clientes (cédula, teléfono, licencia) con **AES-256-GCM** y aplica **Argon2id** para hashes de contraseñas. Ver detalles técnicos y políticas en:

- **[SECURITY.md](SECURITY.md)** — manejo de secretos, rotación de clave PII, reporte de vulnerabilidades e historial del incidente de clave expuesta.

⚠️ **Importante**:
- `data/config.ini` **NO se commitea** — está en `.gitignore`. Usar `data/config.ini.example` como plantilla.
- La clave `db_encryption_key` debe rotarse al menos una vez al año (ver `SECURITY.md` §2).
- Si clonas este repo por primera vez, ejecuta `scripts/sanitize-repo.sh --yes` para limpiar artefactos del working tree (ver §Saneamiento abajo).

---

## 📦 Licencias de terceros

Dinamo Rent ERP redistribuye binarios de Firebird 5.0.3 (licencia dual IDPL+IPL) y VCRedist 14.3 (EULA Microsoft) en `src-tauri/resources/firebird/`. El listado completo de dependencias y sus licencias está en:

- **[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)**

---

## 🧹 Saneamiento del repositorio

El repo incluye un script para limpiar artefactos no commiteables del working tree y del índice Git:

```bash
# Ejecutar en seco (sin --yes, solo imprime qué haría)
bash scripts/sanitize-repo.sh

# Ejecutar de verdad
bash scripts/sanitize-repo.sh --yes
```

El script:
- Borra `Firebird-5.0.3.1683-0-windows-x64/` (copia duplicada, el bundle usa `src-tauri/resources/firebird/`).
- Hace `git rm --cached` de `data/dinamo_rent_v3.fdb`, `data/config.ini`, `Contrato_Dinamo.docx`, `informe_*.xlsx`, `static/preview-shots/*.pdf` (sin borrar del disco).
- Imprime instrucciones para purgar el historial con `git filter-repo` (necesario tras el incidente de clave expuesta, ver `SECURITY.md` §4).
