# Instalación de Dinamo Rent ERP — v1.0.9 (estable)

> Guía para el equipo de operaciones. **Última versión estable: v1.0.9** — con el
> **auto-update** activo desde la v1.0.3 (la app detecta y ofrece instalar las versiones
> nuevas al arrancar, ver sección 6) y, sobre las features previas (IVA por renta,
> auto-cálculo de días/horas, cambio de vehículo, combos con búsqueda, errores de BD
> visibles, fix -303), el **documento**: contrato en 2 hojas, `+57` en los celulares del
> encabezado, multa de la cláusula 4 en blanco, póliza de lucro cesante 40/50/70 mil y
> el campo **Gasolina** en el formulario de renta. Construida y validada por CI.
>
> Las instalaciones **v1.0.2 (sin updater)** se actualizan una vez a mano instalando esta
> versión encima; desde la v1.0.3 las siguientes llegan solas.

---

## 1. Descarga de los instaladores

Página de la release: <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/tag/v1.0.9>

| Instalador | Enlace directo | Tamaño | Uso |
|---|---|---|---|
| **NSIS** (`DinamoRent_1.0.9_x64-setup.exe`) | <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.9/DinamoRent_1.0.9_x64-setup.exe> | ~21 MB | **Recomendado** — instalación asistida con atajo de escritorio |
| **MSI** (`DinamoRent_1.0.9_x64_en-US.msi`) | <https://github.com/CORJAR-Computers/dinamo_rent_tr/releases/download/v1.0.9/DinamoRent_1.0.9_x64_en-US.msi> | ~33 MB | Despliegue empresarial / GPO |

> ⚠️ **No usar la v1.0.0** (descontinuada): falla en equipos sin BD previa. Si un equipo
> ya la tiene instalada **con datos**, no hay que desinstalar — la v1.0.9 abre la BD
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
1. Ejecutar `DinamoRent_1.0.9_x64-setup.exe` como usuario normal.
2. Seguir el asistente (siguiente → instalar → finalizar).

### Opción B — Silenciosa (NSIS)
```powershell
# Instala sin interacción, sin atajo ni ejecución al final
DinamoRent_1.0.9_x64-setup.exe /S
```

### Opción C — Silenciosa (MSI, para GPO/Intune)
```powershell
msiexec /i DinamoRent_1.0.9_x64_en-US.msi /qn /norestart
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

### Auto-actualización (a partir de la v1.0.3)

A partir de la **v1.0.3** la app incorpora el **updater de Tauri v2**: al arrancar comprueba
en GitHub Releases si hay una versión más nueva (`latest.json`) y, si existe, muestra el
diálogo **«Actualización disponible — vX.Y.Z»** con las notas y los botones **Instalar ahora /
Más tarde** (descarga con progreso, verifica la **firma minisign** contra la clave pública
embebida y reinicia la app al terminar).

- **Instalaciones v1.0.3+** → se actualizan **solas**. Requieren conexión a internet en el
  arranque para el chequeo; sin conexión la app funciona igual y reintenta en el siguiente
  arranque.
- **Instalaciones v1.0.2 (sin updater)** → se actualizan **una sola vez a mano**: instalar la
  v1.0.3 encima (sección 3; idempotente, sin pérdida de datos). A partir de ahí reciben las
  siguientes versiones automáticamente.
- Cada release publicada incluye los instaladores, sus firmas (`.sig`) y el `latest.json`;
  la clave privada de firma vive solo en el secret `TAURI_SIGNING_PRIVATE_KEY` del repo
  (nunca en el instalador). Publicación: ver `RELEASE_CHECKLIST.md`.

- **Actualizar desde v1.0.0**: instalar la v1.0.9 encima. Idempotente, sin pérdida de datos.
- **Actualizar desde v1.0.2**: instalar la v1.0.9 encima (transición al auto-update).
- **Actualizar desde v1.0.3+**: desde el diálogo de la app, o a mano instalando la release
  nueva encima (idempotente).
- **Rollback**: si algo fallara, desinstalar y reinstalar la versión anterior conservando
  `%APPDATA%\com.corjar.dinamorent\` (los datos están ahí, no en la carpeta de programa).
  Un rollback manual funciona igual tras un auto-update (desinstalar la versión actual e
  instalar la anterior).
- **Desinstalar**: Panel de control → Programas → Dinamo Rent ERP (o `uninstall.exe` en
  `%LOCALAPPDATA%\DinamoRent\`).

---

## 7. Soporte

- Issues del proyecto: <https://github.com/CORJAR-Computers/dinamo_rent_tr/issues>
- Detalle técnico del fix y políticas de seguridad: `SECURITY.md` y `Handsoff.md` del repo.
