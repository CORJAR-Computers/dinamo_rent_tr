# Changelog

Historial de cambios de **Dinamo Rent ERP** (Tauri V2 + Rust + Firebird Embedded).

Las versiones se publican como [releases en GitHub](https://github.com/CORJAR-Computers/dinamo_rent_tr/releases) con instaladores NSIS/MSI firmados y auto-update vía `tauri-plugin-updater`.

---

## [v1.2.1] — 2026-09-03

### Corregido

- **Migración de dependencias criptográficas** (rand 0.10, argon2 0.6, hmac 0.13 y
  aes-gcm 0.11): el código crypto se migró a las APIs nuevas sin cambiar los formatos de
  salida — el cifrado PII (AES-GCM), los hash de contraseñas (Argon2) y los backups
  cifrados existentes se siguen leyendo sin cambios. La suite completa queda en verde
  con las versiones nuevas.
- **Tests de backups de integración deterministas**: los 4 tests de `backup_integration`
  se serializan y reintentan la lectura de archivos recién escritos — en el runner del
  CI, Defender hacía fallar de forma intermitente la copia/lectura de la BD sembrada
  (os error 3/32) con la misma carrera que ya motivó `reintentar_io` en el servicio.
- **log 0.4.34** (dependabot).

---

## [v1.2.0] — 2026-09-02

### Añadido

- **Cierre de renta completo**: el modal de cierre ahora permite editar el **valor día
  extra** (`valor_dia_extra`), decidir si se **cobran las horas extras** (checkbox
  «Cobrar horas extras», desmarcable para hora de gracia/cortesía) y recalcula el total
  incluyendo la gasolina (`valor_gasolina`) en los extras — antes la gasolina quedaba
  fuera del recálculo.
- **Edición de rentas cerradas ampliada**: el modal de edición de renta cerrada ahora
  permite corregir el **valor día extra** y el flag de **cobrar horas extras**, con la
  auditoría ANTES→DESPUÉS incluyendo `valor_dia_extra`.
- **Contrato de renta con desglose de costos**: el contrato ahora imprime el desglose
  (días base × tarifa, horas extras, día(s) extra, otros cargos, descuento y TOTAL), la
  tarifa por hora en la cláusula de multa por retardo (antes línea en blanco) y la
  devolución real registrada (fecha/hora) cuando la renta está cerrada.

---

## [v1.1.1] — 2026-08-27

### Corregido

- **Kilometraje del auto al cerrar renta**: al cerrar una renta con `km_final`, ahora
  también se actualiza `autos.kilometraje` con el valor de devolución. Antes del fix,
  el kilometraje del auto quedaba desactualizado después del cierre.

---

## [v1.1.0] — 2026-08-27

### Añadido

- **Costo de lavado en reservas**: nuevo campo `costo_lavado` (DECIMAL 12,2) en la tabla
  `reservas` (migración 0028) que permite registrar el costo de lavado al momento de
  crear la reserva y se hereda al convertirla en renta. Incluye UI en el formulario de
  reservas con calculadora en vivo que suma el costo al total, y normalización backend
  que previene errores SQLCODE -303 al enlazar valores vacíos a DECIMAL.

---

## [v1.0.26] — 2026-08-23

### Añadido

- **Atomicidad en mantenimiento**: `crear()` y `actualizar()` del servicio de mantenimiento
  ahora ejecutan INSERT/UPDATE de mantenimiento + sincronización de `autos.proximo_aceite`
  dentro de una sola transacción (`with_transaction`). Si la sincronización falla, toda la
  operación se revierte. `MantenimientoRepository::insertar` y `actualizar` ahora aceptan
  un trait genérico `C: Execute` para poder usarse dentro de transacciones.
- **Sesiones periódicas**: hilo de fondo que purga sesiones expiradas cada 5 minutos.
  `AppState.sessions` ahora es `Arc<Mutex<SessionStore>>` compartido con el hilo de limpieza.
- **Auditoría completa**: `log_audit` en creación y edición de clientes, vehículos y reservas
  (acciones `CREAR CLIENTE`, `ACTUALIZAR CLIENTE`, `CREAR VEHICULO`, `ACTUALIZAR VEHICULO`,
  `CREAR RESERVA`, `ACTUALIZAR RESERVA`).
- **Log injection prevention**: `sanitize_log()` en `logs.rs` escapa `\n`, `\r`, `\t`,
  `\x00`, `\x1b` en los campos `mensaje`, `url` y `stack` antes de escribirlos a
  `frontend_errors.log`.
- **StatusBadge capitalize**: el componente ahora capitaliza la primera letra cuando
  `capitalize=true` (antes ambas ramas del ternario eran idénticas).

### Corregido

- **SQL precedence en `buscar()`**: paréntesis añadidos en `cliente.rs`, `auto.rs` y
  `reserva.rs` alrededor de las 4/3 condiciones OR antes de `AND deleted_at IS NULL`.
  Sin este fix, los registros soft-deleted podían aparecer en búsquedas por nombre o teléfono.
- **Filtros `deleted_at` duplicados**: eliminado `deleted_at IS NULL AND deleted_at IS NULL`
  en `auto.rs` (`obtener_todos`, `obtener_por_estado`), `reserva.rs`
  (`obtener_todos`, `obtener_por_estado`), `usuario.rs` (`obtener_todos`).
- **Carácter chino `位置`**: reemplazado por `línea` en el log de errores del frontend
  (era un label de campo linea:columna con un carácter chino de una traducción accidental).
- **Unwrap seguros**: 6 Mutex `.unwrap()` en `simit.rs` reemplazados por
  `.unwrap_or_else(|e| e.into_inner())` para prevenir panic si un hilo panica dentro del lock.

---

## [v1.0.25] — 2026-08-23

### Añadido

- **Soft-delete completo**: migración 0027 (`0027_soft_delete_entities.sql`) — usuarios,
  reservas, clientes y autos ahora usan `DELETED_AT` (TIMESTAMP) en vez de `DELETE FROM`,
  alineándose con las 5 entidades que ya lo tenían (rentas, pagos, gastos, comparendos,
  mantenimiento desde la 0006). Los registros eliminados se marcan con timestamp y los
  repositories filtran `WHERE deleted_at IS NULL` en todos los SELECT.
- **Auditoría de eliminaciones**: cada soft-delete en reservas, clientes, vehículos y usuarios
  registra evento de auditoría via `log_audit` con acciones `ELIMINAR RESERVA`,
  `ELIMINAR CLIENTE`, `ELIMINAR VEHICULO`, `USUARIO ELIMINADO`.

### Corregido

- **Migración 0026 reescrita**: `cobrar_horas_extra.sql` ahora usa `EXECUTE BLOCK` +
  `EXECUTE STATEMENT` en vez de `SET TERM` (compatibilidad con el migration runner).
- **Fix tests frontend**: `cobrarHorasExtra` añadido a mocks de rentas en tests de
  rentas, alertas, calendario y timelineVehiculo.
- **Fix usuario.rs**: `obtener_por_id` ahora filtra `deleted_at IS NULL`,
  `buscar` con paréntesis correctos en OR, `contar` con filtro.

## [v1.0.24] — 2026-08-22

### Corregido
- **Hora real de cierre en tabla de rentas** — la columna itinerario mostraba siempre la `horaRetorno` (pactada al inicio) incluso para rentas cerradas; ahora si el estado es "Cerrada" se muestra la `horaDevolucionReal` (la hora en que realmente se devolvió el vehículo)
- **Tipos TypeScript generados desincronizados** — `Renta.ts` y `RentaDatos.ts` faltaban el campo `cobrarHorasExtra` introducido en la migración 0026 / v1.0.23

---

## [v1.0.23] — 2026-08-22

### Añadido
- **Checkbox «Cobrar Horas Extra»** en el formulario de creación de renta:
  - Controla si las horas extras se cobran al cierre o se otorgan como cortesía al cliente
  - **Activado por defecto** (las horas extras se cobran como antes)
  - Si se desactiva, las horas extras no se suman al total aunque haya excedente de tiempo
  - Nuevo campo `cobrar_horas_extra` en la tabla `rentas` (migración 0026)
- **Migración 0026**: columna `COBRAR_HORAS_EXTRA` (default 1, compatibilidad con rentas existentes)

### Corregido
- **Horas extras se cobraban en la creación** — el valor de hora extra se sumaba al total desde la creación aunque no hubiera horas a cobrar; ahora solo se cobran al cierre si el checkbox está activo
- **Cierre respeta el flag** — el backend `cerrar()` fuerza `horas_extras = 0` cuando `cobrar_horas_extra` está desactivado

---

## [v1.0.22] — 2026-08-22

### Corregido
- **Edición de renta cerrada** — los inputs numéricos (`type="number"`) enviaban JSON number pero el backend esperaba `Option<String>` para `valorDia`, `valorHoraExtra` y `descuento`; ahora se convierten explícitamente antes de invocar el comando Tauri

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

[v1.2.1]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.1
[v1.2.0]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.2.0
[v1.1.1]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.1.1
[v1.1.0]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.1.0
[v1.0.26]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.26
[v1.0.25]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.25
[v1.0.23]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.23
[v1.0.22]: https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.22
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
