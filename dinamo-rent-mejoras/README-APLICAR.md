# Dinamo Rent ERP — Paquete de Mejoras a Corto Plazo

## Aplicar con git patch (recomendado)

```bash
cd /ruta/a/dinamo_rent_tr
git apply dinamo-rent-mejoras.patch
```

Si prefieres aplicarlo como un commit:
```bash
git am dinamo-rent-mejoras.patch
```

## Aplicar manualmente (desde el zip)

Copia cada archivo a su ruta correspondiente en el repositorio:

| Archivo | Destino |
|---|---|
| `.github/CODEOWNERS` | `.github/CODEOWNERS` (nuevo) |
| `.github/ISSUE_TEMPLATE/bug_report.yml` | `.github/ISSUE_TEMPLATE/bug_report.yml` (nuevo) |
| `.github/ISSUE_TEMPLATE/feature_request.yml` | `.github/ISSUE_TEMPLATE/feature_request.yml` (nuevo) |
| `.github/PULL_REQUEST_TEMPLATE.md` | `.github/PULL_REQUEST_TEMPLATE.md` (nuevo) |
| `CONTRIBUTING.md` | `CONTRIBUTING.md` (nuevo) |
| `LICENSE` | `LICENSE` (nuevo) |
| `src-tauri/src/commands/logs.rs` | Reemplazar existente |
| `src-tauri/src/repositories/auto.rs` | Reemplazar existente |
| `src-tauri/src/repositories/cliente.rs` | Reemplazar existente |
| `src-tauri/src/repositories/reserva.rs` | Reemplazar existente |
| `src-tauri/src/repositories/usuario.rs` | Reemplazar existente |
| `src-tauri/src/services/mantenimiento.rs` | Reemplazar existente |
| `src/lib/components/StatusBadge.svelte` | Reemplazar existente |

## Eliminar (borrar del repo)

```
src/lib/components/Placeholder.svelte  (código muerto)
```

## Cambios aplicados

### Bugs críticos (Backend Rust)
1. **Bug #1 — SQL precedence en cliente.buscar()**: añadidos paréntesis alrededor de
   los 4 OR antes del `AND deleted_at IS NULL`. Sin este fix, los clientes soft-deleted
   aparecían en búsquedas por nombre o teléfono.
   Archivo: `src-tauri/src/repositories/cliente.rs`

2. **Bug #2 — Transacción incompleta en mantenimiento**: `crear()` y `actualizar()`
   ahora usan `with_transaction` para envolver el INSERT/UPDATE + la sincronización de
   `autos.proximo_aceite` en una sola transacción atómica. Si el sync falla, todo se
   revierte (antes quedaba el mantenimiento commiteado pero el km del vehículo obsoleto).
   Archivo: `src-tauri/src/services/mantenimiento.rs`

3. **Bug #3 — Log injection en frontend_errors.log**: añadida función `sanitize_log()`
   que escapa `\n`, `\r`, `\t`, `\x00`, `\x1b` en los campos `mensaje`, `url` y
   `stack` antes de escribirlos al log. Previene falsificación de entradas de log.
   Archivo: `src-tauri/src/commands/logs.rs`

4. **Bug #4 — Carácter chino `位置` stray**: reemplazado por `línea` (era un label de
   campo linea:columna con un carácter chino de una traducción accidental).
   Archivo: `src-tauri/src/commands/logs.rs`

### Quick wins (Frontend Svelte)
5. **#9 — Eliminado Placeholder.svelte**: componente nunca importado (código muerto).
6. **#11 — Limpiado `deleted_at IS NULL AND deleted_at IS NULL`**: duplicación
   residual en 5 archivos (cliente.rs, auto.rs, usuario.rs, reserva.rs).
7. **#12 — Fix ternario inútil en StatusBadge.svelte**: `capitalize ? estado : estado`
   tenía ambas ramas idénticas. Ahora capitaliza la primera letra cuando
   `capitalize=true`.

### Docs/CI (nuevos archivos)
8. **LICENSE**: declaración de copyright All Rights Reserved (faltaba).
9. **CONTRIBUTING.md**: guía de contribución con flujo, convenciones, checklist.
10. **.github/ISSUE_TEMPLATE/bug_report.yml**: plantilla de bug report estructurada.
11. **.github/ISSUE_TEMPLATE/feature_request.yml**: plantilla de feature request.
12. **.github/PULL_REQUEST_TEMPLATE.md**: checklist de PR.
13. **.github/CODEOWNERS**: responsables de review por ruta.

## Verificación post-aplicación

```bash
# Frontend
bun run lint
bun run check
bun run test

# Backend Rust
cd src-tauri
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

---
Generado por el Centro de Mejoras Dinamo Rent ERP · v1.0.25
