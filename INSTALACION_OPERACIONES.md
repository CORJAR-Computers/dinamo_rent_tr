# Instalación de Dinamo Rent ERP — v1.0.2 (estable)

> Guía para el equipo de operaciones. **Última versión estable: v1.0.2** — añade el IVA
> por renta (checkbox), el auto-cálculo de días/horas al cerrar, el cambio de vehículo sin
> cerrar la renta y los combos con búsqueda de clientes y vehículos. Construida y validada
> por CI en Windows limpio.

---

## 1. Descarga de los instaladores

Página de la release: <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.2>

| Instalador | Enlace directo | Tamaño | Uso |
|---|---|---|---|
| **NSIS** (`DinamoRent_1.0.2_x64-setup.exe`) | <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.2/DinamoRent_1.0.2_x64-setup.exe> | ~21 MB | **Recomendado** — instalación asistida con atajo de escritorio |
| **MSI** (`DinamoRent_1.0.2_x64_en-US.msi`) | <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.2/DinamoRent_1.0.2_x64_en-US.msi> | ~31 MB | Despliegue empresarial / GPO |

> ⚠️ **No usar la v1.0.0** (descontinuada): falla en equipos sin BD previa. Si un equipo
> ya la tiene instalada **con datos**, no hay que desinstalar — la v1.0.2 abre la BD
> existente tal cual (arranque idempotente, solo aplica migraciones pendientes).

---

## 2. Requisitos del equipo objetivo

- **Windows x64** (Windows 10 1803+ / Windows 11). Windows 7/8 **no** son soportados.
- **Sin requisitos previos de instalación**: el runtime de Firebird y el de Visual C++
  viajan dentro del instalador (`firebird/`), y WebView2 Runtime se instala
  automáticamente si el sistema no lo tiene (requiere conexión a internet en el primer
  arranque del instalador en ese caso).
- **No** se necesita servidor de base de datos: Firebird Embedded usa un archivo `.fdb`
  local en `%APPDATA%\com.corjar.dinamorent\`.

---

## 3. Instalación

### Opción A — Asistida (NSIS, recomendada)
1. Ejecutar `DinamoRent_1.0.2_x64-setup.exe` como usuario normal.
2. Seguir el asistente (siguiente → instalar → finalizar).

### Opción B — Silenciosa (NSIS)
```powershell
# Instala sin interacción, sin atajo ni ejecución al final
DinamoRent_1.0.2_x64-setup.exe /S
```

### Opción C — Silenciosa (MSI, para GPO/Intune)
```powershell
msiexec /i DinamoRent_1.0.2_x64_en-US.msi /qn /norestart
```

---

## 4. Primer arranque (instalación nueva)

La app crea automáticamente en el primer arranque:

```
%APPDATA%\com.corjar.dinamorent\
├── config.ini            # configuración inicial
└── dinamo_rent_v3.fdb    # base de datos Firebird Embedded (portable)
```

**Credenciales iniciales** (instalación nueva):

| Campo | Valor |
|---|---|
| Usuario | `admin` |
| Contraseña | `admin123` |

> La app **obliga a cambiar la contraseña** en el primer ingreso. Registrar la nueva
> credencial en el gestor de contraseñas del cliente y **rotarla** según la política de
> `SECURITY.md` (la clave PII `db_encryption_key` se rota al menos una vez al año).

En una instalación que ya tenía BD (actualización desde v1.0.0), se conservan **todos**
los usuarios y datos — solo se aplican las migraciones pendientes.

---

## 5. Verificación post-instalación

1. La app abre el **Login** sin colgarse (el bug del v1.0.0 se manifestaba justo aquí).
2. Ingresar con `admin` / contraseña ya cambiada.
3. Confirmar en el Dashboard que la flota, clientes y rentas aparecen (datos existentes)
   o que la app funciona con la BD nueva.
4. (Opcional) El agente SIMIT aparece operativo en la sección de Comparendos.

---

## 6. Actualizar / rollback

- **Actualizar desde v1.0.0**: instalar la v1.0.2 encima. Idempotente, sin pérdida de datos.
- **Rollback**: si algo fallara, desinstalar y reinstalar la versión anterior conservando
  `%APPDATA%\com.corjar.dinamorent\` (los datos están ahí, no en la carpeta de programa).
- **Desinstalar**: Panel de control → Programas → Dinamo Rent ERP (o `uninstall.exe` en
  `%LOCALAPPDATA%\DinamoRent\`).

---

## 7. Soporte

- Issues del proyecto: <https://github.com/CORJAR-Computers/dinamo_rent_tr/issues>
- Detalle técnico del fix y políticas de seguridad: `SECURITY.md` y `Handsoff.md` del repo.
