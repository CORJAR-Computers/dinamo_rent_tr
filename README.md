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

