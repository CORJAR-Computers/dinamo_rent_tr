# PERMISOS.md — Inventario de control de acceso (RBAC)

Inventario completo de los **77 comandos Tauri** expuestos vía IPC, con su guarda
de rol actual y los gaps detectados frente a la configuración
(`[business]` de `config.ini`).

Fecha: 2026-08-11 · Fuente: `src-tauri/src/commands/*.rs` + `src-tauri/src/lib.rs` (handler).

## Modelo de roles

Roles disponibles (`roles_usuarios`): **Administrador, Supervisor, Operador**.

| Clave de config (`[business]`) | Default | Guarda backend | Finalidad |
|---|---|---|---|
| `roles_con_usuarios` | `Administrador` | `require_usuario_admin` | Gestión de usuarios, auditoría, PII |
| `roles_con_informes` | `Administrador` | `require_informes` | Balance mensual (informes contables) |
| `roles_con_eliminar` | `Administrador, Supervisor` | `require_eliminacion` | Borrado de registros |

Guardas disponibles en `commands/mod.rs`:

| Guarda | Roles efectivos | Fallback si config vacía |
|---|---|---|
| `require_session` | Cualquier rol autenticado | — |
| `require_usuario_admin` | `roles_con_usuarios` | `[Administrador]` |
| `require_informes` | `roles_con_informes` | `[Administrador]` |
| `require_eliminacion` | `roles_con_eliminar` | `[Administrador, Supervisor]` |

## A. Públicos / pre-autenticación (sin guarda — intencional)

| Comando | Módulo | Nota |
|---|---|---|
| `app_frontend_lista`, `confirmar_cierre` | app | Ciclo de vida de la ventana (flag/destroy), no tocan datos |
| `login`, `change_password`, `get_login_status` | auth | Pre-login: credenciales propias / estado de bloqueo |

## B. Solo sesión — cualquier rol (50 comandos)

Operación normal del negocio; ningún dato restringido.

| Módulo | Comandos |
|---|---|
| auth | `logout`, `get_session`, `obtener_tema`, `guardar_tema` |
| auto | `listar_autos`, `obtener_auto`, `crear_auto`, `actualizar_auto`, `alertas_autos` |
| cliente | `listar_clientes`, `obtener_cliente`, `crear_cliente`, `actualizar_cliente` |
| comparendo | `listar_comparendos`, `obtener_comparendo`, `crear_comparendo`, `actualizar_comparendo`, `marcar_pagado_comparendo`, `totales_comparendos` |
| dashboard | `get_dashboard_data` |
| gasto | `listar_gastos`, `gastos_recientes`, `obtener_gasto`, `crear_gasto`, `actualizar_gasto`, `totales_gastos` |
| mantenimiento | `listar_mantenimientos`, `mantenimientos_recientes`, `obtener_mantenimiento`, `crear_mantenimiento`, `actualizar_mantenimiento`, `totales_mantenimiento`, `alertas_km_mantenimiento` |
| reserva | `listar_reservas`, `proximas_reservas`, `obtener_reserva`, `crear_reserva`, `actualizar_reserva`, `cancelar_reserva` |
| renta | `listar_rentas`, `obtener_renta`, `crear_renta`, `actualizar_renta`, `cerrar_renta`, `cancelar_renta`, `registrar_pago_renta`, `registrar_inspeccion_renta`, `rentas_activas` |
| simit | `simit_sync_status` |
| business | `get_business_lists` |

## C. `require_eliminacion` — roles_con_eliminar (Admin + Supervisor) · 8 comandos

| Comando | Módulo |
|---|---|
| `eliminar_renta` | renta |
| `eliminar_auto` | auto |
| `eliminar_cliente` | cliente |
| `eliminar_comparendo` | comparendo |
| `eliminar_gasto` | gasto |
| `eliminar_reserva` | reserva |
| `eliminar_mantenimiento` | mantenimiento |
| `simit_sync_now` | simit |

## D. `require_informes` — roles_con_informes (solo Admin) · 1 comando

| Comando | Módulo |
|---|---|
| `informe_mensual` | informe |

## E. `require_usuario_admin` — roles_con_usuarios (Administrador) · 13 comandos

| Módulo | Comandos |
|---|---|
| usuario | `listar_usuarios`, `crear_usuario`, `actualizar_usuario`, `eliminar_usuario`, `forzar_cambio_password_usuario`, `desbloquear_usuario` |
| auditoria | `listar_auditoria`, `acciones_auditoria`, `usuarios_auditoria` |
| pii | `get_pii_status`, `probar_clave_pii`, `guardar_clave_pii`, `eliminar_clave_pii` |

## Frontend (guards de página)

| Ruta | Guarda | Mecanismo |
|---|---|---|
| `/usuarios`, `/auditoria` | Solo Administrador | `guardRole(['Administrador'])` + `tieneRol` |
| `/informes` | `roles_con_informes` (config-driven) | Carga `rolesConEliminar`/`rolesConInformes` vía `businessApi.listas`, `guardRole` + `tieneRol` |
| Botones «Eliminar» en `/rentas`, `/autos`, `/clientes`, `/comparendos`, `/gastos`, `/reservas`, `/mantenimiento` | `roles_con_eliminar` (config-driven) | `puedeEliminar` derived + `{#if}` |
| Resto de páginas | Solo sesión | `guardSesion` |

## Gaps pendientes

Ninguno detectado. Todas las claves de config (`roles_con_usuarios`,
`roles_con_informes`, `roles_con_eliminar`) están aplicadas en los comandos y
la UI.

## Cerrado / verificado

- `roles_con_informes` ✅ aplicado en comando (`require_informes`), página
  (`/informes`) y menú lateral + paleta de comandos.
- `roles_con_usuarios` ✅ aplicado en usuarios, auditoría y PII (backend + UI).
- `roles_con_eliminar` ✅ aplicado a **todos** los borrados (rentas, autos,
  clientes, comparendos, gastos, reservas, mantenimiento) en backend (`require_eliminacion`)
  y UI (botones ocultos para Operador), cubierto por test de integración
  (`borrado_rbac_integration.rs`) y tests de visibilidad por página.
- Comando huérfano `unlock_account` (auth) ✅ **retirado** del handler de Tauri y
  del frontend: era duplicado de `desbloquear_usuario` (que sí lee
  `roles_con_usuarios`) y no lo llamaba ninguna página. El servicio
  `AuthService::unlock_account` se conserva (es la lógica real, con test).
- `simit_sync_now` ✅ restringido a `roles_con_eliminar` (Admin/Supervisor): la
  sincronización manual contra el portal SIMIT ya no la puede disparar un
  Operador; el botón «Sincronizar ahora» se oculta en `/comparendos` para
  roles sin permiso.
- `get_pii_status` ✅ restringido a `roles_con_usuarios` (Administrador): el
  estado del cifrado PII (clave configurada, clientes legacy) ya no lo ven
  Supervisor ni Operador; el botón «Configurar clave» se oculta en
  `/clientes` para roles sin permiso.
- Sin sesión → `session_expired`; rol no autorizado → `permission`
  (verificado por tests en `informes_integration.rs` y `borrado_rbac_integration.rs`).
