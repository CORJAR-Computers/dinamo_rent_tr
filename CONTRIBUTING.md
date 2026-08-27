# Guía de Contribución — Dinamo Rent ERP

¡Gracias por tu interés en contribuir a Dinamo Rent ERP! Este documento describe
el flujo de trabajo y las convenciones del proyecto.

## 🚀 Flujo de desarrollo

1. **Clona el repositorio**
   ```bash
   git clone https://github.com/CORJAR-Computers/dinamo_rent_tr.git
   cd dinamo_rent_tr
   bun install
   ```

2. **Crea una rama** desde `main`:
   ```bash
   git checkout -b fix/descripcion-corta
   ```

3. **Desarrolla y prueba**:
   ```bash
   bun run dev          # frontend (Vite)
   bun run tauri dev    # app completa (Tauri + Rust)
   ```

4. **Verifica antes de commitear** (el hook de husky ejecuta `bun run lint`):
   ```bash
   bun run lint         # ESLint (frontend)
   bun run check        # svelte-check (tipos)
   bun run test         # vitest (tests frontend)
   cd src-tauri && cargo test  # tests Rust + integración
   cd .. && cargo fmt --check  # formato Rust
   cargo clippy -- -D warnings # lints Rust
   ```

5. **Commitea** usando [conventional commits](https://www.conventionalcommits.org/):
   ```
   fix(core): corrige precedencia de operadores en cliente.buscar()
   feat(ui): añade componente Spinner unificado
   refactor(ui): extrae Icon.svelte con 10 iconos nuevos
   docs: sincroniza RELEASE_CHECKLIST a v1.0.26
   ```

6. **Push y PR**:
   ```bash
   git push -u origin fix/descripcion-corta
   gh pr create --title "fix(core): ..." --body "..."
   ```

## 📋 Convenciones

### Commits (conventional commits)
- `fix(scope):` corrección de bugs
- `feat(scope):` nueva funcionalidad
- `refactor(scope):` refactor sin cambio de comportamiento
- `perf(scope):` mejora de rendimiento
- `docs:` cambios en documentación
- `chore:` tareas de mantenimiento
- `test:` additions/modifications de tests

**Scopes comunes:** `core`, `ui`, `security`, `docs`, `ci`, `perf`

### Estilo de código
- **Frontend**: Svelte 5 (runes), TypeScript estricto, Tailwind CSS v4
- **Backend**: Rust, consultas explícitas con `rsfbclient`, `params![]` para SQL parametrizado
- **SQL**: siempre parametrizado (`?`), nunca `format!` con valores de usuario
- **Logs**: usar `tracing`/`log` (no `println!`), sin datos PII

### Testing
- Todo nuevo comando de Tauri debe tener tests de integración
- Todo nuevo componente Svelte debe tener tests con `@testing-library/svelte`
- Los tests de Rust corren contra una BD Firebird Embedded sembrada por `seed_ci`

## 🔒 Seguridad
- **Nunca** commitear `data/config.ini` (está en `.gitignore`)
- **Nunca** commitear claves, tokens o credenciales
- Reportar vulnerabilidades a `seguridad@dinamorent.com` (ver `SECURITY.md`)
- La clave `db_encryption_key` debe rotarse al menos una vez al año

## 🌿 Ramas
- `main` — estable, siempre deployable
- `fix/*` — correcciones de bugs
- `feat/*` — nuevas funcionalidades

## ✅ Checklist del PR
- [ ] `bun run lint` pasa sin errores
- [ ] `bun run check` pasa sin errores
- [ ] `bun run test` pasa
- [ ] `cargo test` pasa (incl. integración con BD sembrada)
- [ ] `cargo clippy` sin warnings
- [ ] `cargo fmt --check` pasa
- [ ] Si hay migraciones: son idempotentes (EXECUTE BLOCK con guards)
- [ ] Si hay cambios de UI: responsive y accessible (aria-label, focus-visible)
- [ ] El PR describe el qué, por qué y cómo verificar
