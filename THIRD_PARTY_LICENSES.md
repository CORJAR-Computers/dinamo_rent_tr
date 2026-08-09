# Licencias de Terceros — Dinamo Rent ERP

Este documento enumera las dependencias de terceros redistribuidas con el binario de Dinamo Rent ERP, sus licencias y dónde encontrar el texto completo.

> Última actualización: 2026-08-09 · Versión ERP: 4.0.0-beta

---

## 1. Firebird SQL 5.0.3 (motor de base de datos embebido)

- **Licencia**: Dual **IDPL** (Initial Developer's Public License) + **IPL** (InterBase Public License). Ambas aprobadas por OSI.
- **Estado**: Redistribuible. Se incluye el motor *embedded* (`fbclient.dll`, `fbembed.dll`, `icudt*.dll`, `icuuc*.dll`, `icuin*.dll`, `engine13.dll`, etc.) dentro del instalador de Dinamo Rent ERP.
- **Ubicación en el repo**: `src-tauri/resources/firebird/` (recursos empaquetados por Tauri).
- **Textos de licencia**: los archivos `IDPLicense.txt` y `IPLicense.txt` acompañan a los binarios en el directorio mencionado y también se incluyen en la carpeta de instalación del producto final.
- **Web oficial / licencias**: https://firebirdsql.org/en/legal/licensing/

## 2. Microsoft Visual C++ Redistributable 14.3 (VCRedist)

- **Archivos redistribuidos**: `msvcp140.dll`, `vcruntime140.dll`, `vcruntime140_1.dll`, `vccrt143_x64.msi` (este último como prerequisito opcional del instalador NSIS/MSI).
- **Licencia**: EULA de Microsoft Visual C++ Redistributable. Redistribuible bajo sus términos (ver `<vcruntime>\Licenses` o el documento `Microsoft_VC142_CRT_Redist.License.rtf` del Windows SDK).
- **Uso**: dependencia runtime de Firebird y de los binarios Rust/Tauri compilados con MSVC.
- **Web oficial**: https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist

## 3. SheetJS (Community Edition)

- **Uso**: generación y lectura de archivos `.xlsx` en informes y exportaciones.
- **Licencia**: Apache-2.0.
- **Web oficial**: https://sheetjs.com/

## 4. Tauri V2

- **Uso**: framework de aplicación de escritorio (Rust + WebView).
- **Licencia**: MIT OR Apache-2.0 (dual).
- **Web oficial**: https://tauri.app/

## 5. SvelteKit 2 / Svelte 5

- **Uso**: framework frontend (SSR/SPA) y runtime de UI con runes.
- **Licencia**: MIT.
- **Web oficial**: https://kit.svelte.dev/ y https://svelte.dev/

## 6. Otros componentes frontend

| Paquete | Licencia | Uso |
|---------|----------|-----|
| Vite | MIT | Bundler / dev server |
| Tailwind CSS v4 | MIT | Estilos utilitarios |
| TypeScript | Apache-2.0 | Tipado estático |

## 7. Crates Rust (cargo)

Todas las crates listadas en `src-tauri/Cargo.toml` se distribuyen bajo licencias permisivas **MIT** o **Apache-2.0** (dual en la mayoría). Las relevantes para el core de seguridad y datos:

| Crate | Licencia | Uso |
|-------|----------|-----|
| `serde` / `serde_json` | MIT OR Apache-2.0 | Serialización |
| `tokio` | MIT | Runtime async |
| `argon2` | MIT OR Apache-2.0 | Hashing de contraseñas (Argon2id) |
| `aes-gcm` | MIT OR Apache-2.0 | Cifrado AES-256-GCM de PII |
| `rsfbclient` | MIT OR Apache-2.0 | Driver Firebird |
| `r2d2` | MIT OR Apache-2.0 | Pool de conexiones |
| `r2d2-firebird` | MIT OR Apache-2.0 | Pool de conexiones Firebird |
| `ring` / `rand` | MIT OR Apache-2.0 | Primitivas criptográficas |
| `thiserror` / `anyhow` | MIT OR Apache-2.0 | Manejo de errores |
| `tauri` | MIT OR Apache-2.0 | Framework desktop |

Para consultar el árbol completo de licencias transitive, ejecutar desde `src-tauri/`:

```bash
cargo install cargo-license
cargo license --avoid-dev-deps --do-not-bundle
```

## 8. WebView2 Runtime (Microsoft)

Tauri requiere WebView2 Runtime en Windows. Se distribuye como prerequisito del instalador bajo los términos de Microsoft. Más información: https://developer.microsoft.com/microsoft-edge/webview2/

---

## Notas

- Los binarios de Firebird redistribuidos se encuentran en `src-tauri/resources/firebird/` junto a los archivos `IDPLicense.txt` e `IPLicense.txt`.
- Este archivo debe actualizarse cada vez que se añada una nueva dependencia redistribuible al bundle.
- En caso de conflicto entre este resumen y el texto completo de cada licencia, prevailce el texto completo de la licencia original.
