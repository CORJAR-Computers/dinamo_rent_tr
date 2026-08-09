# Plan de Migración — Dinamo Rent ERP → Tauri V2 + Rust + SvelteKit + Tailwind CSS

> **Autor:** Ingeniería de Software (Senior Full Stack)
> **Proyecto origen:** `Dinamo_Rent` (Python + PySide6 + SQLAlchemy) v3.2.1
> **Proyecto destino:** Aplicación de escritorio Tauri V2 con backend en Rust, frontend SvelteKit + Tailwind CSS
> **Estado:** Borrador de planificación (Rev. 2 — **Firebird Embedded 5.0 como único motor de BD**)

---

## 1. Resumen ejecutivo

**Dinamo Rent ERP** es un sistema de gestión de flota para renta de vehículos (autos, clientes, rentas, reservas, finanzas, taller, comparendos, alertas, informes, usuarios y auditoría) construido sobre **Python + PySide6 + SQLAlchemy**. El motor de base de datos productivo es **Firebird Embedded** (ya es el valor por defecto en `config.ini`: `engine = firebird`, archivo `dinamo_rent_v3.fdb` con `fbclient.dll` embebida en la carpeta `Firebird-*` del proyecto).

Esta migración reescribe la aplicación con:

| Capa | Origen | Destino |
|------|--------|---------|
| Frontend (UI) | PySide6 (QWidgets + QSS) | SvelteKit 2 + Svelte 5 (runes) + Tailwind CSS v4 |
| Backend (lógica de negocio) | Python `services/` (16 servicios) | Rust (módulos `services/`) |
| Acceso a datos | SQLAlchemy + repositorios `repositories/` | `rsfbclient` (queries explícitas) + runner de migraciones `.sql` |
| Base de datos | **Firebird Embedded 4.0.7** (`.fdb`) | **Firebird Embedded 5.0** (mismo `.fdb`, ODS 13.0 → 13.1) |
| Proceso de escritorio | Python + PyInstaller (`build_exe.py`) | Tauri V2 (WebView2 en Windows) + `tauri build` |
| Reportes PDF | Jinja2 + WeasyPrint | HTML en SvelteKit + impresión WebView (print-to-PDF) |
| Export Excel | pandas + openpyxl | SheetJS (`xlsx`) en el frontend |
| Backups | mysqldump / copia del `.fdb` + Fernet | `gbak` (Firebird) o copia del `.fdb` + AES-256-GCM |

**Decisiones estratégicas principales:**

1. **Firebird Embedded 5.0 es el ÚNICO motor de base de datos** del stack destino. Se **eliminan MySQL y SQLite**. Razones: (a) Firebird Embedded es el motor por defecto y productivo de la app actual — el `.fdb` se reutiliza tal cual; (b) Firebird 5.0 abre el `.fdb` existente (creado con Firebird 4.0.7, ODS 13.0) y lo actualiza a ODS 13.1 automáticamente; (c) Embedded = sin servidor ni instalación, un solo archivo portable, ideal para aplicación de escritorio; (d) licencia IPL/IDPL libre para distribución comercial cerrada; (e) un solo dialecto SQL elimina toda la complejidad de queries multi-motor.
2. **`rsfbclient` en lugar de SQLx**: SQLx no tiene soporte Firebird, pero **Rust sí tiene un driver maduro** — `rsfbclient` (v0.27, 58k descargas, mantenido activamente) es un wrapper del cliente nativo de Firebird (`fbclient.dll`) con **modo embedded** (`builder_native().with_embedded()`), pool vía `r2d2_firebird`, y verificación de tipos en runtime con `FromRow`. Se preserva el estilo de queries explícitas centralizadas en `repositories/`.
3. **Compatibilidad de hashes de contraseña**: los hashes existentes (PBKDF2-SHA256, formato `hex:salt`) se verifican en Rust con la crate `pbkdf2` y se **re-hashean a Argon2id** en el primer login exitoso.
4. **Cifrado de columnas PII**: Fernet (Python) se reemplaza por **AES-256-GCM** (crate `aes-gcm`) en Rust, con re-encriptación durante la migración (solo si cambia la clave).
5. **La UI se migra por módulo** (12 vistas) manteniendo **paridad funcional** con la app Python; se conservan las mismas rutas, menús, roles y colores.

---

## 2. Análisis del sistema actual

### 2.1 Stack y dependencias (`requirements.txt`)

- **UI:** PySide6 ≥ 6.6
- **BD:** SQLAlchemy ≥ 2.0.25, `firebird-driver` (cliente Python de Firebird), `sqlalchemy-firebird`, Alembic ≥ 1.13
- **Motor por defecto:** Firebird Embedded 4.0.7 (carpeta `Firebird-4.0.7.3271/fbclient.dll` en la raíz; archivo `dinamo_rent_v3.fdb`). MySQL y SQLite existen como alternativas secundarias en el código, pero Firebird es el motor configurado (`config.ini [database] engine = firebird`).
- **Validación:** Pydantic ≥ 2.5, pydantic-settings
- **Reportes:** Jinja2, WeasyPrint, reportlab
- **Excel:** pandas, openpyxl
- **Criptografía:** cryptography ≥ 41 (Fernet, PBKDF2HMAC)

### 2.2 Arquitectura por capas

```
main_qt.py                     → Punto de entrada (Splash, Login, MainWindow, menú lateral)
├── core/                      → Núcleo
│   ├── app_config.py / config.py    → Config centralizada (config.ini, configparser)
│   ├── database_sa.py               → Engine/Session Firebird Embedded + auto-migraciones
│   ├── models.py                    → Modelos SQLAlchemy (12 tablas)
│   ├── security.py                  → PBKDF2, sesiones en memoria, rate limiting, bloqueo de cuentas
│   ├── security_crypto.py           → Fernet (columnas encriptadas EncryptedString/EncryptedText)
│   ├── rbac.py                      → Decoradores require_role / require_active_session
│   ├── schemas.py                   → Schemas Pydantic
│   ├── validators.py, utils.py, logger.py, exceptions.py, worker.py
├── repositories/              → 14 repositorios SQLAlchemy (por entidad)
├── services/                  → 16 servicios (lógica de negocio + RBAC)
├── views/                     → 12 vistas + diálogos + componentes + temas QSS
├── templates/                 → 3 plantillas Jinja2 (contrato, orden de renta, orden de reserva)
├── tests/                     → ~45 archivos de test (cobertura ~98%)
└── migrations/                → Alembic (initial_schema)
```

### 2.3 Inventario de módulos (vistas ↔ servicios ↔ tablas)

| # | Vista (PySide6) | Servicio | Tabla(s) principal(es) |
|---|-----------------|----------|------------------------|
| 0 | Dashboard | `DashboardService` | KPIs agregados |
| 1 | Calendario | `RentaService.obtener_para_calendario` | rentas |
| 2 | Rentas | `RentaService`, `PagoService`, `InspeccionService` | rentas, pagos, inspecciones |
| 3 | Reservas | `ReservaService` | reservas |
| 4 | Clientes | `ClienteService` | clientes |
| 5 | Flota (Autos) | `AutoService` | autos |
| 6 | Taller (Mantenimiento) | `MantenimientoService` | mantenimiento_vehiculos |
| 7 | Usuarios | `UsuarioService` | usuarios |
| 8 | Informes | `InformeService` (+ RBAC) | agregados financieros |
| 9 | Comparendos | `ComparendoService` | comparendos |
| 10 | Alertas | `AlertaService`, `DashboardService` | alertas calculadas |
| 11 | Caja Menor (Gastos) | `GastoService` | gastos |
| — | Diálogos | Cierre de renta, Pagos, Config BD, Setup wizard, Forzar cambio de contraseña, Acerca de | — |
| — | Infra | `AuthService`, `BackupService`, `FinancialService` | usuarios, auditoria |

**Servicios completos (16):** alerta, auth, auto, backup, cliente, comparendo, dashboard, financial, gasto, informe, inspeccion, mantenimiento, pago, renta, reserva, usuario.

### 2.4 Esquema de base de datos (12 tablas)

`usuarios`, `autos`, `clientes`, `rentas`, `reservas`, `mantenimiento_vehiculos`, `configuracion`, `auditoria`, `inspecciones`, `comparendos`, `pagos`, `gastos` (+ `alembic_version`).

Detalles críticos:
- **Moneda:** `DECIMAL(12,2)` — en Rust debe usarse un tipo decimal exacto (ver §4.5).
- **Columnas cifradas (Fernet):** `clientes.celular`, `clientes.celular2`, `clientes.email`, `clientes.dir_residencia`, `clientes.dir_temporal`, `clientes.no_licencia`.
- **Índices compuestos** en rentas, clientes, gastos, comparendos, mantenimiento, pagos, auditoría (preservar en las migraciones).
- **FK con `ON DELETE`/`ON UPDATE`** específicos (SET NULL, CASCADE) — respetar exactamente.
- **Seed:** usuario `admin` (PBKDF2), flag `debe_cambiar_password`.

### 2.5 Seguridad actual

- **Hash contraseñas:** PBKDF2-HMAC-SHA256, 100.000 iteraciones, formato `{hex}:{salt_hex}` (salt 16 bytes hex). Verificación en tiempo constante (`secrets.compare_digest`).
- **Sesiones:** `SessionManager` en memoria (dict), token `secrets.token_urlsafe(32)`, timeout 3600 s por inactividad.
- **Rate limiting:** 5 intentos por usuario → bloqueo 30 min; ventana de 10 intentos/5 min; 20 intentos por IP/minuto. Contador persistido en `usuarios.intentos_fallidos`.
- **RBAC:** decoradores `@require_role(...)` y `@require_active_session` sobre métodos de servicio; roles: Administrador, Supervisor y otros; `roles_con_informes`, `roles_con_usuarios` configurables.
- **Cifrado at-rest:** columnas PII con Fernet (clave en `config.ini [security] db_encryption_key`).
- **Auditoría:** tabla `auditoria` + logger de auditoría (login OK/fallido, cambio de contraseña, accesos denegados).
- **Validación:** sanitización XSS (`validators.validate_no_xss`), fortaleza de contraseñas (8+ chars, mayúscula, minúscula, dígito, símbolo).

### 2.6 Reportes y backups

- **Reportes PDF:** plantillas Jinja2 (`contrato_jinja_template.html`, `orden_renta_jinja.html`, `orden_reserva_jinja.html`) + WeasyPrint.
- **Excel:** informes con pandas/openpyxl.
- **Backups:** copia del archivo `.fdb` (motor Firebird) / `mysqldump` (si se usa MySQL) vía subprocess; rotación (max 10 copias); cifrado opcional Fernet+PBKDF2 con salt de 16 bytes prefijado al archivo.

### 2.7 Tests y CI

- **pytest** ~45 archivos, cobertura ~98%. Fixture `conftest.py` con BD en memoria (SQLite para CI; producción usa Firebird).
- **GitHub Actions:** `ruff-lint.yml` (ruff + pytest), `release.yml` (tag → release con binario).

---

## 3. Arquitectura objetivo

### 3.1 Diagrama de componentes

```
┌──────────────────────────────────────────────────────────────────┐
│  Tauri V2 (ventana nativa, WebView2 en Windows)                  │
│                                                                  │
│  ┌──────────────────────────────┐  ┌──────────────────────────┐  │
│  │ Frontend SvelteKit (SPA)     │  │ Backend Rust (src-tauri) │  │
│  │ - Svelte 5 (runes)           │  │ - tauri::State (sesiones)│  │
│  │ - Tailwind CSS v4            │  │ - services/ (16 módulos) │  │
│  │ - @tauri-apps/api (invoke)   │  │ - repositories/          │  │
│  │ - Store sesión (localStorage)│  │   (rsfbclient)           │  │
│  └──────────────────────────────┘  │ - core/ (config, crypto) │  │
│                                    │   rbac, error            │  │
│                                    │ - migrations/ (.sql,     │  │
│                                    │   runner propio)         │  │
│                                    └───────────┬──────────────┘  │
│                                                 │ rsfbclient     │
│                                                 │ (en proceso)   │
│                                    ┌───────────▼──────────────┐  │
│                                    │ Firebird Embedded 5.0    │  │
│                                    │ fbclient.dll en proceso  │  │
│                                    │ dinamo_rent_v3.fdb       │  │
│                                    └──────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

Firebird Embedded corre **dentro del mismo proceso** (fbclient.dll se carga en memoria): no hay servidor, no hay puerto, no hay instalación. Los archivos de Firebird Embedded 5.0 se empaquetan junto al binario Tauri (ver §4.1).

### 3.2 Stack destino (versiones de referencia — ajustar a las vigentes)

| Componente | Versión recomendada |
|------------|---------------------|
| Rust | edition 2021, toolchain estable reciente |
| Tauri | **2.x** (`tauri = "2"`, `tauri-build = "2"`, CLI `@tauri-apps/cli@2`) |
| SvelteKit | **2.x** (`@sveltejs/kit`) + Svelte **5** |
| Tailwind CSS | **v4** (`tailwindcss` + `@tailwindcss/vite`, CSS-first config) |
| Driver Firebird | **`rsfbclient` 0.27.x** (features `dynamic_loading`; modo embedded con `builder_native().with_embedded()`) |
| Pool | **`r2d2` + `r2d2_firebird`** (0.27.x) |
| Firebird Embedded | **5.0.x** (zip Windows x64 → `fbclient.dll`, `firebird.msg`, `intl/`, `plugins/`, ICU dlls) |
| Tokio | 1.x (runtime de Tauri; driver síncrono → `spawn_blocking` para queries pesadas) |
| Cripto | `argon2`, `pbkdf2`, `aes-gcm`, `sha2`, `hmac`, `rand`, `base64`, `hex` |
| Decimal | `rust_decimal` (parseo desde string que entrega el driver) |
| Logging | `tracing` + `tracing-subscriber` (o `log` + `env_logger`) |
| Config | `config` crate o `ini` + `serde` |
| Backups | `gbak` (Firebird) vía `std::process` o copia del `.fdb` |
| Frontend libs | `@tauri-apps/api`, `xlsx` (SheetJS), `date-fns`, `svelte-sonner` o toasts propios |

### 3.3 Estructura de carpetas del nuevo repositorio

```
dinamo-rent-tauri/
├── package.json                  # Frontend (SvelteKit)
├── svelte.config.js              # adapter-static, ssr=false, prerender=true
├── vite.config.ts                # + @tailwindcss/vite, tauri plugin
├── src/
│   ├── app.html
│   ├── app.css                   # @import "tailwindcss"; + tokens de tema
│   ├── routes/
│   │   ├── +layout.svelte        # Shell: sidebar, header, auth guard
│   │   ├── +layout.ts            # export const ssr = false; prerender = true
│   │   ├── +page.svelte          # Redirect a /dashboard
│   │   ├── login/+page.svelte
│   │   ├── dashboard/+page.svelte
│   │   ├── calendario/+page.svelte
│   │   ├── rentas/+page.svelte
│   │   ├── rentas/[id]/+page.svelte
│   │   ├── reservas/+page.svelte
│   │   ├── clientes/+page.svelte
│   │   ├── autos/+page.svelte
│   │   ├── mantenimiento/+page.svelte
│   │   ├── usuarios/+page.svelte
│   │   ├── informes/+page.svelte
│   │   ├── comparendos/+page.svelte
│   │   ├── alertas/+page.svelte
│   │   └── gastos/+page.svelte
│   ├── lib/
│   │   ├── api.ts                # invoke() tipado (todos los comandos Tauri)
│   │   ├── stores/session.svelte.ts   # $state rune: usuario, rol, token
│   │   ├── components/           # DataTable, Modal, Toast, StatusBadge, FormField...
│   │   ├── utils/format.ts       # moneda COP, fechas
│   │   └── styles/theme.ts       # tokens de color (Claro/Oscuro)
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json           # v2 schema (+ bundle.resources para Firebird)
│   ├── capabilities/default.json
│   ├── build.rs
│   ├── icons/
│   ├── resources/firebird/       # Motor Embedded 5.0 (fbclient.dll, firebird.msg, intl/, plugins/, ICU) — NO se commitea el .fdb
│   └── src/
│       ├── main.rs               # setup: config, db pool, state, migraciones
│       ├── lib.rs                # builder tauri, registro de comandos
│       ├── core/
│       │   ├── mod.rs
│       │   ├── config.rs         # puerto de core/app_config.py
│       │   ├── db.rs             # puerto de core/database_sa.py (pool rsfbclient, init, check)
│       │   ├── migrations.rs     # runner propio (tabla schema_migrations + archivos .sql)
│       │   ├── error.rs          # AppError + mapeo a mensajes de usuario
│       │   ├── crypto.rs         # AES-256-GCM + helpers (puerto de security_crypto.py)
│       │   ├── security.rs       # Argon2id + PBKDF2 legacy + sesiones + rate limit
│       │   ├── rbac.rs           # helpers require_role / require_active_session
│       │   ├── audit.rs          # escritura tabla auditoria
│       │   └── validators.rs     # sanitización, fortaleza de password
│       ├── repositories/         # 1 módulo por entidad (rsfbclient, queries explícitas)
│       ├── services/             # 16 módulos (lógica de negocio)
│       ├── commands/             # #[tauri::command] wrappers (thin)
│       └── migrations/           # .sql de Firebird (0001_initial_schema.sql, ...)
├── scripts/
│   └── exportar_datos.py         # (opcional) re-encripta PII / valida paridad sobre el .fdb
├── tests/                        # Rust integration tests (Firebird Embedded temporal) + Vitest
└── .github/workflows/            # ci.yml + release.yml (tauri-action)
```

---

## 4. Decisiones técnicas clave (ADRs)

### 4.1 Base de datos: Firebird Embedded 5.0, ÚNICO motor (eliminar MySQL y SQLite)
**Decisión:** El stack destino usa **solo Firebird Embedded 5.0**. Se eliminan MySQL y SQLite (como motor de producción y como motor de pruebas).

Razones:
1. **Continuidad:** la app actual ya corre sobre Firebird Embedded (default en `config.ini`); el `.fdb` de producción se reutiliza sin exportar/importar datos.
2. **Compatibilidad:** Firebird 5.0 abre el `.fdb` creado por Firebird 4.0.7 (ODS 13.0) y lo actualiza a ODS 13.1 automáticamente en el primer arranque (respaldar con `gbak` antes).
3. **Embedded = cero infraestructura:** fbclient.dll se carga en el proceso; no hay servicio, ni puertos, ni instalación en el equipo del cliente. Ideal para app de escritorio portable.
4. **Licencia libre:** IPL/IDPL permite distribución comercial cerrada incluyendo los binarios de Firebird sin abrir el código propio (solo conservar avisos de licencia).
5. **Un solo dialecto:** desaparece la complejidad de queries multi-motor (fechas, `extract()`, rangos) que hoy obliga a `if DB_ENGINE == ...` en repositorios y servicios.
6. **Rust SÍ tiene driver:** el argumento original del plan anterior ("Rust no tiene driver maduro") es incorrecto — `rsfbclient` es maduro y soporta embedded nativo (ver §4.2).

Consecuencias:
- El `.fdb` se distribuye **fuera** del instalador (se crea/abre en la primera ejecución en la carpeta de datos del usuario); solo se empaquetan los binarios de Firebird Embedded.
- La config `[database] engine` se fija en `firebird`; se conservan `path`, `user` y `password` (defaults `sysdba`/`masterkey`).
- Las pruebas (CI y locales) usan un `.fdb` temporal creado por el propio motor embedded — paridad real con producción.

### 4.2 Acceso a datos: rsfbclient (driver nativo de Firebird, no ORM)
**Decisión:** `rsfbclient` con queries explícitas. Razones:
1. Es el **driver Rust estándar de facto** para Firebird: wrapper del cliente nativo (`fbclient.dll`), ~58k descargas, mantenimiento activo (v0.27, 2026).
2. **Modo embedded nativo**: `builder_native().with_dyn_load(path).with_embedded().db_name("...fdb").connect()` — carga fbclient.dll en el proceso, sin servidor.
3. Pool de conexiones con **`r2d2` + `r2d2_firebird`** (multiplexa el driver síncrono sobre hilos).
4. Preserva el estilo actual: queries explícitas centralizadas en `repositories/` con `FromRow` para mapear filas a structs.
5. Migraciones: **runner propio** (tabla `schema_migrations` + archivos `.sql` en orden), porque SQLx migrate no soporta Firebird. Firebird soporta DDL transaccional.

Alternativas evaluadas:
- **SQLx**: no tiene soporte Firebird oficial (`sqlx-firebird` es beta sin mantenimiento). **Descartado.**
- **`firebirust` / `firebird-wire`** (implementaciones Rust del wire protocol): los backends **pure-Rust** (`rsfbclient` con feature `pure_rust`, `firebird-wire`) no soportan modo embedded (requieren servidor TCP) o son muy recientes. **Descartados** para el modo embedded — por eso se usa el backend **nativo** de `rsfbclient` sobre `fbclient.dll`.
- **SeaORM / Diesel** (`rsfbclient-diesel`): agregan curva de aprendizaje sin beneficio para queries ya escritas. **Descartados.**
- **Sidecar Python** (mantener la app Python como capa de datos): fricción de distribución y arranque. **Descartado.**

Trade-off a mitigar: `rsfbclient` no tiene macros `query!` de verificación en compilación como SQLx; se compensa con tests de integración contra `.fdb` reales (§8) y revisión de queries en los PRs.

### 4.3 Hash de contraseñas: Argon2id con compatibilidad PBKDF2
- **Nuevos hashes:** Argon2id (`argon2` crate, m=19456, t=2, p=1 — o valores recomendados por OWASP), formato `$argon2id$...` (PHC string).
- **Hashes legados (PBKDF2 `hex:salt`):** se detectan por formato, se verifican con crate `pbkdf2` + `sha2` (100.000 iteraciones) y se **re-hashean a Argon2id en el primer login exitoso** (write-back).
- **Migración de datos:** no es necesario re-hashear el `.fdb`; la compatibilidad en runtime es suficiente y más segura.

### 4.4 Sesiones y RBAC
- **Sesiones:** `tauri::State<AppState>` con `Mutex<HashMap<String, SessionData>>`; token generado con `rand` (32 bytes base64url). Timeout 3600 s por inactividad (configurable). Al ser una app de escritorio de usuario único, el modelo en memoria es correcto (igual que Python).
- **RBAC:** helpers en `core::rbac` que todo comando debe llamar: `require_role(state, token, &["Administrador", ...])?` y `require_active_session(state, token)?`. Mismas reglas que `core/rbac.py` (roles, `roles_con_informes`, `roles_con_usuarios`).
- **Registro de comandos:** los `#[tauri::command]` de `commands/` son *thin wrappers* que (1) validan sesión/RBAC, (2) llaman al servicio, (3) mapean `AppError` → `Err(String)` serializable al frontend.

### 4.5 Decimales (moneda)
**Decisión:** los montos Firebird `DECIMAL(12,2)` se convierten a **`rust_decimal`** (feature `serde`) — según el mapeo del driver (string o `f64`), usando `FromStr`/parseo exacto para evitar pérdida de precisión. Se serializa como string en el IPC JSON; el frontend formatea con `Intl.NumberFormat` (COP). Evitar `f64` para montos.

### 4.6 Cifrado de columnas PII: AES-256-GCM
- Clave derivada de `config.ini [security] db_encryption_key` (se conserva el mismo campo; si es una clave Fernet base64 de 44 chars, se **deriva un nuevo key AES-256** de ella con SHA-256 — así no hay que regenerar credenciales).
- Formato de valor cifrado: `v1:{nonce_base64}:{ciphertext_base64}`.
- `scripts/exportar_datos.py` desencripta con Fernet (clave vieja) y re-encripta con el nuevo formato si se cambia la clave; si la clave se conserva, la derivación SHA-256 hace transparente la migración.

### 4.7 Reportes PDF y Excel
- **PDF:** se renderizan las plantillas HTML existentes (portadas a SvelteKit o componentes imprimibles) y se usa la **impresión del WebView** (`window.print()` → "Guardar como PDF"). Es el mecanismo nativo de Tauri, sin dependencias pesadas. Alternativa futura: plugin `tauri-plugin-print` o crate `printpdf` en Rust.
- **Excel:** `xlsx` (SheetJS) en el frontend para exportar los mismos informes.

### 4.8 Backups
- **Firebird (primario):** invocar **`gbak`** vía `std::process::Command` (se empaqueta `gbak.exe` + `zlib1.dll` junto al binario; usa el mismo `fbclient.dll`). Comando: `gbak -b -user sysdba -password *** dinamo_rent_v3.fdb Backup_Dinamo_<ts>.fbk` (formato nativo, consistente). Ejecutar con la BD sin conexiones activas de escritura.
- **Firebird (alternativa simple):** copia del archivo `.fdb` (`fs::copy`) con la app sin conexiones activas (mismo patrón de copia de archivo que usa la app actual para Firebird); suficiente para single-user embedded.
- **Cifrado:** AES-256-GCM con PBKDF2-SHA256 (salt 16 bytes prefijado) — compatible en concepto con el actual Fernet+PBKDF2.
- **Rotación:** conservar N copias (config `max_copies`, default 10).

### 4.9 Configuración
- Se **mantiene `config.ini`** (mismo formato y secciones) para no romper la migración de instalaciones existentes. Se implementa con crate `config` (o `ini` + serde) y los mismos defaults de `core/config.py` (engine fijo `firebird`, `path = dinamo_rent_v3.fdb`, user/password `sysdba`/`masterkey`).
- Alternativa a considerar (Fase 9, opcional): migrar a `tauri-plugin-store` (JSON) manteniendo un importador desde `config.ini`.

### 4.10 Tareas pesadas y asincronía
- Los comandos son `async fn` (Tauri). **`rsfbclient` es síncrono**: las operaciones de BD se ejecutan en `tokio::task::spawn_blocking` (con el pool `r2d2` como despachador) para no bloquear el runtime.
- Backups y `gbak` también usan `spawn_blocking` para no congelar la UI.
- El patrón `QRunnable`/`QTimer` de Python se reemplaza por promesas en el frontend (`invoke()` es async) + estados de carga.

---

## 5. Mapeo módulo a módulo

| Módulo Python | Equivalente Rust | Equivalente SvelteKit |
|---------------|------------------|-----------------------|
| `core/app_config.py` + `core/config.py` | `core/config.rs` | `src/lib/stores/config.svelte.ts` (solo lectura de UI) |
| `core/database_sa.py` | `core/db.rs` (+ `core/migrations.rs`) | — |
| `core/models.py` | `repositories/*.rs` + `migrations/*.sql` | tipos TS en `src/lib/api.ts` |
| `core/security.py` | `core/security.rs` | — |
| `core/security_crypto.py` | `core/crypto.rs` | — |
| `core/rbac.py` | `core/rbac.rs` | `src/lib/stores/session.svelte.ts` (guard de rutas) |
| `core/validators.py` | `core/validators.rs` | validación de formularios en `src/lib/components/` |
| `core/exceptions.py` | `core/error.rs` (`AppError`) | manejo de errores en `src/lib/api.ts` |
| `core/logger.py` | `core/audit.rs` + `tracing` | toasts/notificaciones |
| `repositories/*_sa.py` (14) | `repositories/*.rs` | — |
| `services/auth_service.py` | `services/auth.rs` | `src/routes/login` |
| `services/auto_service.py` | `services/auto.rs` | `src/routes/autos` |
| `services/cliente_service.py` | `services/cliente.rs` | `src/routes/clientes` |
| `services/renta_service.py` | `services/renta.rs` | `src/routes/rentas` |
| `services/reserva_service.py` | `services/reserva.rs` | `src/routes/reservas` |
| `services/mantenimiento_service.py` | `services/mantenimiento.rs` | `src/routes/mantenimiento` |
| `services/usuario_service.py` | `services/usuario.rs` | `src/routes/usuarios` |
| `services/informe_service.py` | `services/informe.rs` | `src/routes/informes` |
| `services/comparendo_service.py` | `services/comparendo.rs` | `src/routes/comparendos` |
| `services/alerta_service.py` | `services/alerta.rs` | `src/routes/alertas` |
| `services/gasto_service.py` | `services/gasto.rs` | `src/routes/gastos` |
| `services/pago_service.py` | `services/pago.rs` | componente en `rentas/[id]` |
| `services/inspeccion_service.py` | `services/inspeccion.rs` | componente en `rentas/[id]` |
| `services/dashboard_service.py` | `services/dashboard.rs` | `src/routes/dashboard` |
| `services/financial_service.py` | `services/financial.rs` | lógica compartida de cálculos |
| `services/backup_service.py` | `services/backup.rs` | diálogo de backups |
| `views/*_view.py` (12) | — | `src/routes/*` |
| `views/dialogs/*` | — | modales en `src/lib/components/` |
| `views/themes/*` (QSS) | — | `src/app.css` + `src/lib/styles/theme.ts` (dark/light) |
| `templates/*.jinja.html` | — | componentes imprimibles en `src/lib/components/reports/` |
| `main_qt.py` (menú) | — | `src/routes/+layout.svelte` (sidebar, RBAC por rol) |

---

## 6. Migración de datos

### 6.1 Estrategia general
1. **El `.fdb` se reutiliza directamente**: Firebird 5.0 Embedded abre el archivo existente (creado con Firebird 4.0.7) y actualiza el formato interno (ODS 13.0 → 13.1) automáticamente. **No hay exportación/importación de datos entre motores.**
2. **Esquema:** se portan `migrations/versions/*_initial_schema.py` y el esquema real del `.fdb` a migraciones `.sql` de Firebird (runner propio, §4.2). El esquema ya es 100% Firebird, así que el portado es directo.
3. **PII (si cambia la clave):** `scripts/exportar_datos.py` desencripta las columnas Fernet (clave de `config.ini` origen) y re-encripta con AES-GCM (formato destino). Si la clave se conserva, la derivación SHA-256 (§4.6) hace la migración transparente.
4. **Usuarios:** se conservan tal cual (hash PBKDF2) — la verificación legacy + re-hash Argon2id ocurre en runtime (§4.3).
5. **Validación:** script de conteo por tabla y spot-checks de montos totales (origen vs destino) después del primer arranque con el motor nuevo.

### 6.2 Prerrequisito obligatorio
Antes de abrir el `.fdb` por primera vez con Firebird 5.0: **backup con `gbak`** del archivo actual (copia de seguridad del ODS 13.0) y verificación de que restaura correctamente.

### 6.3 Verificación de paridad
Script de auditoría que compara: recuento por tabla, suma de montos (`pagos.monto`, `rentas.total`, `gastos.monto`), y conteo de usuarios activos — tomando como referencia el `.fdb` original.

---

## 7. Plan de implementación por fases

> Cada fase termina con un **hito verificable** (build + tests + criterio de aceptación).
> Comandos base: `npm run tauri dev` (desarrollo), `npm run tauri build` (producción), `cargo test` / `npm run test` (tests).

### Fase 0 — Bootstrap del proyecto (½ día)

**Objetivo:** repo nuevo con frontend SvelteKit + Tauri V2 compilando end-to-end.

Pasos:
1. Crear app SvelteKit: `npm create svelte@latest dinamo-rent-tauri` (template minimal, TypeScript, opciones de lint/format activadas).
2. Agregar `adapter-static`:
   ```bash
   npm i -D @sveltejs/adapter-static @tauri-apps/cli@^2
   npm i @tauri-apps/api@^2
   ```
3. `svelte.config.js`:
   ```js
   import adapter from '@sveltejs/adapter-static';
   export default {
     kit: {
       adapter: adapter({ fallback: 'index.html' }), // SPA mode
     },
   };
   ```
4. `src/routes/+layout.ts`:
   ```ts
   export const ssr = false;
   export const prerender = true;
   ```
5. Tailwind v4:
   ```bash
   npm i tailwindcss @tailwindcss/vite
   ```
   `vite.config.ts`: agregar `tailwindcss()` a plugins; `src/app.css`: `@import "tailwindcss";`
6. Inicializar Tauri: `npx tauri init` (o crear `src-tauri/` a mano). Verificar `tauri.conf.json` (schema v2) con **recursos de Firebird Embedded**:
   ```json
   {
     "$schema": "https://schema.tauri.app/config/2",
     "productName": "DinamoRent",
     "version": "0.1.0",
     "identifier": "com.corjar.dinamorent",
     "build": {
       "beforeDevCommand": "npm run dev",
       "devUrl": "http://localhost:5173",
       "beforeBuildCommand": "npm run build",
       "frontendDist": "../build"
     },
     "app": {
       "windows": [{ "title": "Dinamo Rent ERP", "width": 1366, "height": 768 }],
       "security": { "csp": null }
     },
     "bundle": {
       "active": true,
       "targets": ["nsis"],
       "icon": ["icons/icon.ico"],
       "resources": ["resources/firebird/**/*"]
     }
   }
   ```
7. Descargar **Firebird 5.0 Embedded** (zip Windows x64 de firebirdsql.org) y copiar a `src-tauri/resources/firebird/`: `fbclient.dll`, `ib_util.dll`, `firebird.msg`, `firebird.conf` (opcional), carpetas `intl/` y `plugins/`, DLLs de ICU, y los runtimes MSVC si no vienen con el instalador.
8. Verificar hello-world Tauri: `npm run tauri dev`.

**Criterio de aceptación:** ventana Tauri abre con página SvelteKit + Tailwind aplicado; `cargo build` sin errores; Firebird Embedded se carga en proceso (`SELECT 1 FROM RDB$DATABASE` desde un comando de prueba).

### Fase 1 — Núcleo Rust (config, BD, errores, crypto, auditoría) (2–3 días)

**Objetivo:** infraestructura de backend equivalente a `core/` (sin lógica de negocio aún).

1. `Cargo.toml` — dependencias:
   ```toml
   [dependencies]
   tauri = { version = "2", features = [] }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   rsfbclient = { version = "0.27", features = ["dynamic_loading"] }
   r2d2 = "0.8"
   r2d2_firebird = "0.27"
   tokio = { version = "1", features = ["full"] }
   rust_decimal = { version = "1", features = ["serde"] }
   chrono = { version = "0.4", features = ["serde"] }
   argon2 = "0.5"
   pbkdf2 = { version = "0.12", features = ["sha2"] }
   sha2 = "0.10"
   aes-gcm = "0.10"
   rand = "0.8"
   base64 = "0.22"
   hex = "0.4"
   tracing = "0.1"
   tracing-subscriber = "0.3"
   config = "0.14"
   ```
2. `core/config.rs`: leer `config.ini` con crate `config` (o `ini`), replicar secciones: database (engine fijo `firebird`, path, user, password, pool_size), security, backup, logging, application, ui, business, email, whatsapp, reports. Exponer tipos `AppConfig` con getters tipados. Los defaults deben copiarse de `core/config.py::_DEFAULTS`.
3. `core/db.rs`:
   ```rust
   pub async fn connect(cfg: &AppConfig) -> Result<r2d2::Pool<FirebirdConnectionManager>> {
       let fbclient_path = cfg.fbclient_path(); // resources/firebird/fbclient.dll
       let manager = FirebirdConnectionManager::new(
           rsfbclient::builder_native()
               .with_dyn_load(&fbclient_path)   // carga fbclient.dll en proceso
               .with_embedded()                  // modo embedded (sin servidor)
               .db_name(cfg.db_path())           // dinamo_rent_v3.fdb
               .user(cfg.db_user())
               .password(cfg.db_password()),
       );
       let pool = r2d2::Pool::builder()
           .max_size(cfg.pool_size())
           .build(manager)?;
       run_migrations(&pool)?; // runner propio (§4.2)
       Ok(pool)
   }
   ```
   - Registrar `AppState { db: Pool, sessions: Mutex<HashMap<String, SessionData>>, config: Arc<AppConfig> }` en `tauri::Builder::manage()`.
   - Replicar `check_connection()` (query `SELECT CURRENT_USER FROM RDB$DATABASE`) y el seed de `admin` (solo si la tabla está vacía; hash Argon2id, `debe_cambiar_password=1`).
   - Queries síncronas → envolver con `tauri::async_runtime::spawn_blocking` en los comandos.
4. `core/migrations.rs`: runner propio — crea `schema_migrations(version VARCHAR PRIMARY KEY, applied_at TIMESTAMP)` si no existe; aplica en orden los `.sql` de `src-tauri/migrations/` no ejecutados, cada uno en su transacción (DDL transaccional de Firebird).
5. `core/error.rs`: enum `AppError` (NotFound, Validation, Permission, Locked, RateLimited, Db, Crypto, Generic) con `mensaje_usuario` — espejo de `core/exceptions.py`. Implementar `Serialize` para devolver `Err(serde_json::Value)` desde comandos.
6. `core/crypto.rs`: AES-256-GCM encrypt/decrypt con formato `v1:{nonce}:{ct}` y derivación de clave desde `db_encryption_key`.
7. `core/audit.rs`: helper `log_audit(db, usuario, accion, mensaje, ip)` insertando en `auditoria`.
8. Migraciones iniciales (`migrations/0001_initial_schema.sql`): crear las 12 tablas en dialecto Firebird (tomar como referencia el esquema real generado por SQLAlchemy sobre el `.fdb`).

**Hito:** `cargo test` con tests unitarios de config (lectura de `config.ini.example`), crypto (round-trip), y conexión a un `.fdb` temporal con Firebird Embedded. `npm run tauri dev` inicia sin errores.

### Fase 2 — Autenticación, sesiones y RBAC (2–3 días)

**Objetivo:** `AuthService` portado, primer login funcional con el frontend.

1. `core/security.rs`:
   - `hash_password(pwd) -> String` (Argon2id, PHC string).
   - `verify_password(stored, provided) -> bool`: detecta `$argon2id$` → argon2; formato `hex:salt` → pbkdf2 100k; retorna `NeedsRehash` para el write-back.
   - `validate_password_strength(pwd) -> Vec<String>` (mismas reglas que Python).
2. `services/auth.rs`:
   - `login(username, password, ip) -> LoginResult`: bloqueo por intentos (5) + lockout 30 min, rate limit por IP (20/min) y por usuario (10/5min) — puerto de `LoginAttemptTracker`; persiste `intentos_fallidos` en `usuarios`.
   - `cambiar_password_obligatorio(username, current, new)`.
   - `unlock_account(username)` (admin).
   - `sync_tracker_from_db()` al iniciar app.
   - `logout(session_id)`.
3. `core/rbac.rs`:
   ```rust
   pub fn require_role(state: &AppState, token: &str, roles: &[&str]) -> Result<SessionData, AppError>
   pub fn require_active_session(state: &AppState, token: &str) -> Result<SessionData, AppError>
   ```
   (expiración por inactividad 3600 s, `last_activity` refresh).
4. `commands/auth.rs`: `login`, `logout`, `change_password`, `get_login_status`, `unlock_account`.
5. Frontend: `src/routes/login/+page.svelte` (formulario, errores, bloqueo/espera), `src/lib/api.ts` con `invoke('login', ...)`, `src/lib/stores/session.svelte.ts` (persistencia del token en localStorage, guard en `+layout.svelte`: sin token → redirect a `/login`).
6. Forzar cambio de contraseña (flag `debe_cambiar_password`) como modal obligatorio tras login.

**Hito:** login con usuario admin (creado por seed), logout, cambio de contraseña, bloqueo tras 5 intentos fallidos — todo probado manualmente y con tests Rust de `services/auth.rs`.

### Fase 3 — Capa de repositorios + datos (3–5 días)

**Objetivo:** los 14 repositorios en Rust + apertura real del `.fdb`.

1. Portar cada `repositories/*_sa.py` a `repositories/*.rs` con `rsfbclient` (`query_iter`/`query_first` + `FromRow`). Mantener **exactamente** los mismos nombres de métodos y firmas. Empezar por `usuario`, `auto`, `cliente` (las 3 bases), luego `renta`, `reserva`, `pago`, `inspeccion`, `comparendo`, `gasto`, `mantenimiento`, `alerta`, `informe`.
2. Las queries se escriben en **dialecto Firebird** (los repos ya fueron corregidos para Firebird en la app Python: `extract()`, rangos de fecha, `first/last`, etc. — se portan tal cual).
3. Prueba de humo con el `.fdb` real: abrir la BD productiva (respaldada con `gbak`) con Firebird 5.0 Embedded y validar ODS + conteos (§6.3).
4. Escribir `tests/` de integración con un **`.fdb` temporal** (creado con la sentencia `CREATE DATABASE` del driver o copiando un fixture, aplicando luego el runner de migraciones en un directorio temporal) replicando los casos de `tests/test_repositories_unit.py` / `test_repositories_restantes.py`.

**Hito:** todos los repos con tests verdes sobre Firebird Embedded; el `.fdb` real abre correctamente con el motor 5.0 y paridad verificada.

### Fase 4 — Servicios de negocio (5–8 días)

**Objetivo:** 16 servicios en Rust con la misma lógica, validaciones y RBAC.

Orden recomendado (por dependencia):
1. `financial.rs` (cálculos de totales: `calcular_total_renta`, `calcular_total_cierre`, `roi_flota`) — probarlo de forma aislada con tablas de casos.
2. `renta.rs` (crear, cerrar, extender, cambiar vehículo, obtener, activas, documento, calendario) — el más complejo; portar con cuidado los cálculos de días/horas extras/descuentos.
3. `pago.rs`, `inspeccion.rs`, `reserva.rs`.
4. `auto.rs` (CRUD + alertas de vencimientos SOAT/tecnomecánica/extintor), `cliente.rs` (búsqueda, geografía), `mantenimiento.rs`, `gasto.rs`, `comparendo.rs`.
5. `alerta.rs` + `dashboard.rs` (KPIs: `kpi_y_financiero`, `kpi_globales`, rentas por vencer, documentos por vencer, alertas flota) — portar **exactamente** las agregaciones SQL (ya en dialecto Firebird).
6. `usuario.rs` (listar/crear/actualizar/eliminar/forzar cambio — solo roles permitidos), `informe.rs` (`balance_mensual_real` con RBAC).
7. `backup.rs` (§4.8: `gbak` o copia del `.fdb`).

Regla por comando: validar RBAC en el wrapper `commands/`, lógica pura en `services/`, datos en `repositories/`.

**Hito:** cobertura de lógica de negocio con tests Rust (portar casos de `test_services*.py`, `test_financial_service.py`, `test_rentas_widget.py` — la parte de cálculo). Comparación de resultados numéricos contra la app Python para un set de fixtures.

### Fase 5 — Shell de UI y tema (3–4 días)

**Objetivo:** layout global (sidebar, header, routing, tema claro/oscuro) con los 12 módulos como placeholders.

1. `src/routes/+layout.svelte`: sidebar colapsable con las mismas categorías y filtrado por rol que `MainWindow._MENU_STRUCTURE` (PRINCIPAL, OPERACIÓN, ADMINISTRACIÓN, FINANZAS, SISTEMA). Footer con usuario/rol, botón tema (Claro/Oscuro), "Acerca de", "Cerrar sesión".
2. Tokens de tema en `src/app.css` (variables CSS) + `src/lib/styles/theme.ts`: colores actuales de `config.ini [ui]` (primario `#004aad`, éxito, peligro, alerta, estados de auto, calendario). Persistencia en localStorage.
3. Componentes base (`src/lib/components/`): `DataTable`, `Modal`, `Toast`, `StatusBadge`, `FormField`, `ConfirmDialog`, `LoadingSpinner`, `EmptyState`, `StatCard` — espejo de `views/components/*`.
4. `src/lib/api.ts`: función `invoke<T>(cmd, args)` tipada + `ApiError` con `mensaje_usuario`.
5. Ruteo: 12 rutas con páginas placeholder; guard de sesión y rol.

**Hito:** navegación completa entre los 12 módulos con sidebar filtrada por rol, tema claro/oscuro funcionando, toasts y modales base.

### Fase 6 — Migración de UI por módulo (8–12 días)

**Objetivo:** paridad funcional completa. Un módulo por iteración (orden: Autos → Clientes → Rentas → Reservas → Mantenimiento → Gastos → Comparendos → Usuarios → Alertas → Dashboard → Calendario → Informes).

Plantilla de iteración (por módulo):
1. Listar funcionalidades de la vista Python (tablas, filtros, CRUD, validaciones, diálogos).
2. Portar queries/servicios ya listos (Fase 4).
3. Construir la página Svelte con DataTable/Modal/forms; formatos COP y fechas con `src/lib/utils/format.ts`.
4. Diálogos especiales: **Cierre de renta** (cálculo de totales, pagos, inspección), **Pagos**, **Configurar Base de Datos** (ruta del `.fdb`, credenciales), **Setup wizard** (primera ejecución: crear/abrir `.fdb`, crear admin), **Forzar cambio de contraseña**, **Acerca de**.
5. Calendario: grid mensual con colores por estado (Disponible/Rentado/Reservado/Taller) — CSS, sin dependencia de librería de calendarios (o `svelte-calendar` si se prefiere).
6. Test con Vitest + Testing Library (render, interacciones clave).

**Criterio:** checklist de paridad (§9) por módulo; pruebas manuales contra datos reales.

### Fase 7 — Reportes, PDF y Excel (2–3 días)

1. Portar las 3 plantillas HTML de `templates/` a componentes imprimibles en `src/lib/components/reports/` (contrato, orden de renta, orden de reserva).
2. Diálogo de impresión: `window.print()` con `@media print` (ocultar sidebar, A4, logos).
3. Export Excel: función `exportToExcel(data, filename)` con SheetJS para informes.
4. `informes`: `balance_mensual_real` con filtros y exportación PDF/Excel.

**Hito:** contrato/órdenes imprimibles con datos reales; Excel descargable desde Informes.

### Fase 8 — Backups, config DB y setup (2–3 días)

1. `services/backup.rs` + diálogo de backups en UI (crear con `gbak`, listar, rotar, desencriptar/restaurar).
2. `database_config_dialog`: probar conexión al `.fdb` y guardar credenciales en `config.ini` (sin selector de motor — Firebird Embedded es el único).
3. Setup wizard de primera ejecución (crear/abrir `.fdb`, crear admin, probar conexión) — puerto de `views/setup_wizard.py`.
4. Backup automático programado (4 horarios configurables) usando `tauri::async_runtime` o un hilo con timer en Rust; estado visible en UI.

**Hito:** backups automáticos y manuales funcionando (gbak/copia); setup desde cero con Firebird Embedded 5.0.

### Fase 9 — Testing, CI/CD y empaquetado (3–4 días)

1. **Tests Rust:** unit (core), integración (repos/servicios sobre un `.fdb` temporal de Firebird Embedded), ~paridad con `test_services*.py`. Meta: cobertura ≥ 90% de `core/` y `services/` con `tarpaulin`.
2. **Tests frontend:** Vitest + Testing Library (`tests/frontend/`): login flow, guards de rol, formato COP, componentes clave.
3. **GitHub Actions:**
   ```yaml
   # .github/workflows/ci.yml — on push/PR
   jobs:
     rust:   # cargo fmt --check, cargo clippy -D warnings, cargo test
             #   + descargar Firebird 5.0 Embedded (zip x64) en el runner Windows
     web:    # npm ci, npm run check, npm run test
     build:  # tauri-apps/tauri-action@v0 → upload artifact (Windows, incluye resources/firebird)
   ```
   - `release.yml`: on tag `v*` → `tauri-action` publica release con instaladores (NSIS `.exe`/`.msi`).
4. **Empaquetado:** `npm run tauri build` → `src-tauri/target/release/bundle/nsis/*.exe` con `resources/firebird/` incluido. Configurar `productName`, iconos, `identifier`, versionado semántico alineado con `CHANGELOG.md` (partir de v4.0.0).
5. **Docs:** actualizar README con instrucciones de build; migrar `CONFIGURACION.md` y `SEGURIDAD.md` al nuevo contexto (Firebird Embedded 5.0).

**Criterio de aceptación final:** CI verde completo, instalador generado (Firebird Embedded incluido y funcional en máquina limpia), checklist de paridad 100%.

---

## 8. Estrategia de pruebas

| Capa | Herramienta | Objetivo |
|------|-------------|----------|
| Core Rust (config, crypto, security) | `cargo test` | round-trips, formatos, casos límite |
| Repositorios | `cargo test` (`.fdb` temporal, Firebird Embedded) | paridad con `test_repositories_*.py` |
| Servicios | `cargo test` (`.fdb` temporal + fixtures) | paridad numérica con `test_services*.py` |
| Frontend | Vitest + Testing Library | flujos críticos, guards RBAC, formatos |
| E2E (opcional, Fase 9+) | Playwright contra `npm run dev` | humo de las 12 rutas |
| Manual | Checklist de paridad (§9) | validación con datos reales |

**Fixtures compartidos:** dump `.fdb` de prueba con autos/clientes/rentas (generado aplicando `0001_initial_schema.sql` + seeds sobre un `.fdb` temporal; misma herramienta que CI). Nota: las pruebas de la app Python usan SQLite en memoria; el destino usa Firebird Embedded — el `.fdb` temporal replica el comportamiento productivo real.

---

## 9. Checklist de paridad funcional

- [ ] Login, logout, bloqueo de cuenta, rate limiting, cambio obligatorio de contraseña
- [ ] RBAC: menú e informes/usuarios filtrados por rol
- [ ] Dashboard: KPIs financieros, rentas por vencer, alertas flota
- [ ] Calendario: colores por estado, solapamiento de fechas
- [ ] Autos: CRUD, estados, alertas de vencimientos (SOAT, técnico, extintor, aceite)
- [ ] Clientes: búsqueda, CRUD, campos cifrados (PII), geografía
- [ ] Rentas: crear, cierre con cálculo de totales, extender, cambiar vehículo, documento
- [ ] Reservas: CRUD, cancelar, orden imprimible
- [ ] Pagos: registrar, listar por renta, saldo pendiente
- [ ] Inspecciones: entrada/salida por renta
- [ ] Mantenimiento: historial, registrar, alertas por km
- [ ] Gastos (Caja Menor): registrar, listar por placa/recientes
- [ ] Comparendos: registrar, cambiar estado
- [ ] Usuarios: CRUD (solo roles permitidos), desbloquear, forzar cambio
- [ ] Informes: balance mensual real, export PDF + Excel
- [ ] Alertas: listado global
- [ ] Backups: automático (4 horarios) y manual con `gbak`/copia, cifrado, rotación, restaurar
- [ ] Config BD: probar conexión al `.fdb`, cambiar ruta/credenciales (Firebird Embedded único motor)
- [ ] Setup wizard primera ejecución (crear/abrir `.fdb`, admin inicial)
- [ ] Temas claro/oscuro con colores actuales
- [ ] Toasts, confirmaciones, estados vacíos/carga en todas las vistas
- [ ] Auditoría: eventos de login, cambios de contraseña, accesos denegados

---

## 10. Riesgos y mitigación

| Riesgo | Impacto | Mitigación |
|--------|---------|------------|
| Queries heredadas escritas para otros motores (MySQL/SQLite) en repos/servicios | Medio | Los repos ya fueron corregidos a dialecto Firebird en la app Python (CHANGELOG); portar tal cual + tests contra `.fdb` real |
| Compatibilidad ODS al abrir el `.fdb` con Firebird 5.0 (13.0 → 13.1) | Medio | `gbak` de respaldo antes del primer arranque; validación de paridad (§6.3); si falla, restaurar y migrar vía backup/restore |
| Driver `rsfbclient` menos difundido que SQLx (sin macros de verificación en compilación) | Medio | Tests de integración sobre `.fdb` real, revisión de queries en PRs, pool `r2d2`; comunidad activa (v0.27, 2026) |
| Empaquetado incompleto de Firebird Embedded (faltan DLLs ICU/plugins) | Alto | Fase 0 con lista de archivos verificada en máquina limpia; test de humo post-instalación (NSIS) |
| Driver síncrono bloquea el runtime async de Tauri | Medio | `tokio::task::spawn_blocking` para todas las operaciones de BD y `gbak` (§4.10) |
| Pérdida de datos PII durante migración de cifrado | Alto | Script de re-encriptación con validación de paridad (§6.3); si la clave no cambia, derivación SHA-256 transparente |
| Diferencias de precisión decimal en cálculos financieros | Alto | `rust_decimal` + suite de fixtures comparada contra Python |
| Hashes de contraseña legacy | Medio | Verificación PBKDF2 + re-hash Argon2id en login (§4.3) |
| Paridad de UI (12 módulos) subestimada | Alto | Fases 5–6 por módulo con checklist; empezar por módulos simples |
| `gbak` no disponible o bloqueado | Medio | Rutas configurables + fallback a copia del `.fdb` con la app sin conexiones activas |
| IPC JSON overhead en tablas grandes (calendario/informes) | Medio | Paginación en `DataTable`, queries agregadas en SQL |

---

## 11. Referencias

- Tauri V2: https://tauri.app/start/ · CLI: https://v2.tauri.app/reference/cli/ · `tauri.conf.json`: https://v2.tauri.app/reference/config/
- SvelteKit: https://kit.svelte.dev/docs/introduction · Svelte 5 runes: https://svelte.dev/docs/svelte/what-are-runes
- Tailwind CSS v4: https://tailwindcss.com/docs/installation/using-vite
- **rsfbclient** (driver Rust Firebird): https://github.com/fernandobatels/rsfbclient · docs: https://docs.rs/rsfbclient
- **r2d2_firebird** (pool): https://crates.io/crates/r2d2_firebird
- **Firebird 5.0** (descargas, zip Windows x64 — kit embedded): https://www.firebirdsql.org/en/firebird-5-0/ · Manual de referencia: https://www.firebirdsql.org/en/reference-manuals/
- **gbak** (backup/restore Firebird): https://www.firebirdsql.org/file/documentation/html/en/refdocs/fbutils/gbak.html
- Argon2 (Rust): https://docs.rs/argon2 · OWASP password storage: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- Tauri + frontend SvelteKit (guía oficial de framework): https://tauri.app/start/frontend/sveltekit/
- SheetJS: https://docs.sheetjs.com/
- `tauri-action` (release CI): https://github.com/tauri-apps/tauri-action

---

## 12. Próximos pasos inmediatos

1. Crear el repo `dinamo-rent-tauri` y ejecutar la **Fase 0** (incluye descargar Firebird 5.0 Embedded y verificar el hello-world con conexión al `.fdb`).
2. Confirmar con el equipo las decisiones de §4 (especialmente: **Firebird Embedded 5.0 único motor**, rsfbclient, Argon2id, gbak).
3. **Respaldar el `.fdb` de producción con `gbak`** y validar el restore antes de la primera apertura con Firebird 5.0.
4. Definir el alcance del primer hito entregable: **login + dashboard + autos + clientes** (Fases 0–4 + módulos base).
