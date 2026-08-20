# Changelog

Historial de cambios de **Dinamo Rent ERP** (Tauri V2 + Rust + Firebird Embedded).

Las versiones se publican como [releases en GitHub](https://github.com/CORJAR-Computers/dinamo_rent_tr/releases) con instaladores NSIS/MSI firmados y auto-update vía `tauri-plugin-updater`.

---

## [v1.0.21] — 2026-08-20

### Corregido
- **Nombre de empresa** actualizado a "DINAMO RENT A CAR" en fallback, informes Excel y test
- Sección **Empresa ocultada** del sidebar (uso interno de Dinamo Rent a Car)
- Cargo.lock sincronizado con la versión 1.0.21

---

## [v1.0.20] — 2026-08-20

### Corregido
- **Crash de tracing_subscriber** en Windows sin consola — cambiado a `try_init()` silencioso
- Resolución de conflicto entre `tracing_subscriber` y `tauri_plugin_log`

### Añadido
- **Sistema de logs** para diagnóstico de errores y bugs:
  - Comandos Tauri: `leer_logs`, `leer_errores_frontend`, `registrar_error_frontend`, `exportar_logs`, `limpiar_logs`
  - Captura global de errores JS (`window.onerror`, `unhandledrejection`) con debounce
  - Página `/logs` (admin only) con vista, exportación y truncado
  - Icono `logs` (terminal) en el sidebar

---

## [v1.0.19] — 2026-08-20

### Corregido
- **Crash en app GUI** — `tracing_subscriber::fmt().init()` causaba panic en Windows sin consola

### Añadido
- Documentación de los Bloques 1-4 en Handsoff.md

---

## [v1.0.18] — 2026-08-20

### Añadido

#### ⚡ Bloque 1 — Performance
- **Informes optimizados** con `UNION ALL` (13→5 round-trips a Firebird)
- **Store global `BusinessLists`** con TTL 5 min para listas de config
- **`async spawn_blocking`** en `listar_rentas` e `informe_mensual`

#### 🏗️ Bloque 3 — Code Quality
- **`core::repository`** centraliza helpers DRY (`map_fb_error`, `opt_str`, `parse_fecha/hora`, `params!`)
- **`domain/`** scaffold para value objects
- **Migración 0025** `audit_inmutable` (excepciones nombradas + triggers append-only)
- **`ts-rs`** para contratos TypeScript

#### ♿ Bloque 4 — Accesibilidad
- **Modal** focus trap + autofocus + restore
- **FormField** ARIA (label, `aria-describedby`, `aria-invalid`)
- **Skip-link** para naveación por teclado
- **Página de error global** (`+error.svelte`)
- **Tracing estructurado** (spans en login/cerrar/pago)

#### 🤖 Infraestructura
- **Dependabot** para npm, cargo y CI
- Verificador de despliegue `-DryRun` en CI

### Corregido
- **Normalización de fechas** en cálculo de vencimiento de rentas (medianoche local)
- **RBAC Informes**: solo Administrador (Supervisor ya no ve informes contables)

---

## [v1.0.17] — 2026-08-19

### Añadido
- **Edición de rentas cerradas** (solo Admin/Supervisor, auditoría ANTES→DESPUÉS)
- **Extensiones acumulables** de rentas (migración 0024, historial de horas/días extra)
- **Mayúsculas automáticas** en campos de texto (excepto email, rol, web)
- **Validación case-insensitive** en login y búsquedas

### Corregido
- Selects de **categoría/tipo** en mantenimiento y gastos alineados con la DB
- Tabla `extensiones_renta` asegurada en tests de integración

---

## [v1.0.16] — 2026-08-19

### Añadido
- **Versión real de la app** en el menú lateral y el login (comando `app_version`, antes mostraba v3.0)
- **Backups automáticos programados** (config `[backup]`: 4 horarios, rotación a 10 copias)
- **Cifrado AES-256-GCM** de backups (opcional, por chunks con salt PBKDF2)
- **Comando `backup_ahora`** + panel de backups en la UI (crear, listar, estado, restaurar)
- **Restauración de backups** (descifrar si cifrado, gbak -r, rename atómico con reintentos)
- **Verificador de despliegue** `-DryRun` (valida flujo sin tocar máquina real)
- Test de integración `app_version` (verifica que devuelve la versión de Cargo.toml)

### Corregido
- Compartición transitoria (Defender) en backups — reintentos automáticos
- Migración 0025 reescrita (excepciones nombradas Firebird, triggers append-only)

---

## [v1.0.15] — 2026-08-17

### Añadido
- **Comisión por renta** (checkbox + valor; neto = total − comisión) visible en:
  - Informe mensual con comisiones y valor neto
  - Balance general
  - Listado de rentas y timeline por vehículo
- **Comparendos con procedencia persistente** (origen SIMIT/Manual, `ultimo_visto_simit`)
- **Filtros de comparendos**: "No confirmadas por SIMIT" + "Solo nuevos de la última sincronización" (combinables)
- **Persistencia del último resultado** del Agente SIMIT en la BD (sobrevive al reinicio)
- **Verificador de despliegue** `-DryRun` en CI
- CI actualizado a Node 24

### Corregido
- Flaky test de SIMIT con servidor TCP local (elimina dependencia externa)

---

## [v1.0.14] — 2026-08-16

### Añadido
- **Botón "Buscar actualización"** — check manual del updater con feedback en la barra superior

### Corregido
- **Auto-update bloqueado por ACL** — permisos `updater:default` + `process:default` en capabilities
- Auto-update nunca funcionó en ≤v1.0.13, ahora funciona desde esta versión

---

## [v1.0.13] — 2026-08-16

### Añadido
- **Crear renta desde reserva** — acción en reservas y precarga del formulario con `?desdeReserva=<id>`
- **Completar reserva automáticamente** al crear la renta asociada (misma transacción)

### Corregido
- Cálculo unificado de días/horas entre rentas y reservas
- Formulario de reserva estilo renta (consistencia UX)
- Semilla CI determinista para tests de integración

---

## [v1.0.12] — 2026-08-15

### Corregido
- **Contrato de renta** — espacio amplio para firmar (44px sobre la línea) sin romper 2 hojas

---

## [v1.0.11] — 2026-08-15

### Corregido
- **Kilometraje impreso** sin cola de ceros (42000 en vez de 42000.000000000000)

---

## [v1.0.10] — 2026-08-15

### Corregido
- Etiqueta de versión en verificar-despliegue

---

## [v1.0.9] — 2026-08-15

### Corregido
- **Contrato a 2 hojas** — logo reducido (70px) y encabezado compacto

---

## [v1.0.8] — 2026-08-15

### Corregido
- **INSERT de rentas** con SQLCODE -804 — conteo exacto de placeholders al agregar `VALOR_GASOLINA`

---

## [v1.0.7] — 2026-08-15

### Corregido
- **Contrato de renta en 2 hojas** — tipografía final 6.2pt / interlineado 0.98

---

## [v1.0.6] — 2026-08-15

### Añadido
- **Contrato a 2 hojas** con cláusulas legales completas
- **Prefijo +57** automático en teléfonos colombianos
- **Multa en blanco** para comparendos sin valor
- **Pólizas** con valores configurables (40/50/70 mil)
- **Cargo de gasolina** en rentas (campo `valor_gasolina`)

### Corregido
- Orden de reserva más legible (sin firmas, tipografía amplia)

---

## [v1.0.5] — 2026-08-14

### Corregido
- **SQLCODE -303** al crear/editar rentas — montos vacíos ya no rompen el CAST DECIMAL

---

## [v1.0.4] — 2026-08-14

### Añadido
- **Errores de BD visibles** en la UI y en log de archivo

---

## [v1.0.3] — 2026-08-14

### Añadido
- **Auto-actualización** de la app con `tauri-plugin-updater` (vía `latest.json` en GitHub Releases)
- **Orden de renta más legible** — sin firmas, tipografía amplia
- Test E2E del flujo de auto-actualización

---

## [v1.0.2] — 2026-08-13

### Añadido
- **Rentas con IVA** por checkbox (19% configurable)
- **Auto-cálculo de días/horas** en rentas y reservas
- **Cambiar vehículo** en renta activa
- **Combos con búsqueda** (autos, clientes, reservas)
- **Setup inicial de la empresa** con branding dinámico (logo + datos, white-label)
- **Campo ciudad** de la empresa y cláusula compromisoria dinámica
- **Importador** de autos y clientes desde dump SQL o Excel
- Changelog automático en el body de las releases

---

## [v1.0.1] — 2026-08-12

### Corregido
- **Instalación limpia sin cuelgues** — `SetDllDirectoryW` para el runtime VC++ de Firebird
- **BD Firebird se crea** en instalación limpia (bug del release v1.0.0)
- Feature linking de `rsfbclient` desactivado en release build
- Migraciones embebidas (no dependen de archivos en disco)

### Añadido
- Workflows de validación (CI) y publicación automática de releases (GitHub Actions)
- Node 24 en workflows (jsdom 30 requiere Node ≥22.4)

---

## [v1.0.0] — 2026-08-12

Primera release estable. Migración completa de Python a Tauri V2 + Rust + Firebird Embedded.

### Módulos incluidos
- **Autos**: CRUD, estados, alertas de vencimientos (SOAT, técnico, extintor, batería, aceite)
- **Clientes**: CRUD con PII cifrado (AES-256-GCM)
- **Rentas**: CRUD, cálculo de días/horas, pagos, inspecciones, cierre con devolución
- **Reservas**: CRUD, confirmación, cancelación
- **Mantenimiento**: CRUD, alertas por kilometraje
- **Gastos** (Caja Menor): CRUD con categorías
- **Comparendos**: CRUD con atribución a renta/cliente
- **Informes**: Mensual consolidado (ingresos, gastos, balance)
- **Dashboard**: KPIs operacionales (rentas activas, vehículos disponibles, ingresos, alertas)
- **Usuarios**: CRUD con RBAC (Administrador, Supervisor, Operador)
- **Auditoría**: Trail de acciones por usuario
- **Alertas**: Panel consolidado de vencimientos, km y rentas por vencer
- **Calendario**: Vista de rentas/reservas en calendario

### Infraestructura
- Pool de conexiones Firebird Embedded (r2d2)
- Migraciones SQL idempotentes (EXECUTE BLOCK con guards)
- Backup de la BD con gbak
- Seed CI determinista para tests de integración
- Verificador de despliegue post-instalación

---

[v1.0.21]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.21
[v1.0.20]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.20
[v1.0.19]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.19
[v1.0.18]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.18
[v1.0.17]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.17
[v1.0.16]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.16
[v1.0.15]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.15
[v1.0.14]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.14
[v1.0.13]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.13
[v1.0.12]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.12
[v1.0.11]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.11
[v1.0.10]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.10
[v1.0.9]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.9
[v1.0.8]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.8
[v1.0.7]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.7
[v1.0.6]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.6
[v1.0.5]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.5
[v1.0.4]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.4
[v1.0.3]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.3
[v1.0.2]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.2
[v1.0.1]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.1
[v1.0.0]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.0
