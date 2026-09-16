# Resumen de Cambios: Calidad y Mejoras Operativas (QA & PM)

Se han implementado y validado con éxito las correcciones prioritarias de calidad, fiabilidad operativa y cálculo financiero en **Dinamo Rent**, respetando la exclusión del ciclo de garantías para su posterior integración con la web de reservas.

---

## 0. Segunda Ronda de QA (revisión posterior)

Verificación completa ejecutada tras la primera ronda: `npm run check` (0 errores), `npm test` (28 archivos, 254 pruebas), `cargo check`, `cargo test --test rentas_integration` (14 pruebas) y `cargo test --test backup_integration` (4 pruebas) — todo verde. La revisión detectó y corrigió lo siguiente:

### 2.1 El mismo bug de zona horaria existía en 7 pantallas más
- **Archivos:** [+page.svelte de rentas](file:///d:/dinamo_rent_tr/src/routes/rentas/+page.svelte), [reservas](file:///d:/dinamo_rent_tr/src/routes/reservas/+page.svelte), [autos](file:///d:/dinamo_rent_tr/src/routes/autos/+page.svelte), [mantenimiento](file:///d:/dinamo_rent_tr/src/routes/mantenimiento/+page.svelte), [gastos](file:///d:/dinamo_rent_tr/src/routes/gastos/+page.svelte), [comparendos](file:///d:/dinamo_rent_tr/src/routes/comparendos/+page.svelte), [calendario](file:///d:/dinamo_rent_tr/src/routes/calendario/+page.svelte) y [logs](file:///d:/dinamo_rent_tr/src/routes/logs/+page.svelte)
- **Problema:** El fix de `informes` (§1.2) no cubría los demás usos de `new Date().toISOString().slice(0, 10)`. El más grave: `defaultCierre()` en rentas proponía como **fecha de devolución real la de mañana** si el operador cerraba una renta después de las 7:00 PM (UTC-5), distorsionando el recálculo de días/horas del cierre. También afectaba valores por defecto de nuevas rentas/reservas, fechas de gastos/mantenimientos/comparendos y los límites de consulta del calendario.
- **Solución:** Se reemplazaron todos los usos por `formatLocalDateISO()` (ya existente en `format.ts`) y el cálculo de «mañana» por aritmética local (`new Date(y, m, d + 1)`) en lugar de `getTime() + 86400000`.

### 2.2 Hardening de `extender` contra desbordes de fecha
- **Archivo:** [renta.rs](file:///d:/dinamo_rent_tr/src-tauri/src/services/renta.rs)
- **Problema:** `retorno_dt + chrono::Duration::hours/days(cantidad)` entra en **pánico** si la cantidad es extrema (un error de digitación en la UI derrumbaría el comando Tauri en vez de devolver un error de validación).
- **Solución:** Se usa `checked_add_signed` (devuelve error de validación en vez de paniquear) y se limita la extensión a un desplazamiento máximo de 5 años.

### 2.3 Fast-fail de backups vacíos/truncados antes de restaurar
- **Archivo:** [backup.rs](file:///d:/dinamo_rent_tr/src-tauri/src/services/backup.rs)
- **Problema:** La salvaguarda pre-restauración (§1.4) copiaba la BD actual, pero un `.fbk` de staging vacío o truncado igualmente llegaba a `gbak -r`, con riesgo de dejar la BD reemplazada por datos inservibles.
- **Solución:** `restaurar_fdb_desde_fbk` rechaza de forma temprana (antes de invocar gbak y antes de la copia `pre_restore`) cualquier staging menor a 1 KB. Nuevo test `restaurar_staging_vacio_falla_sin_copias_pre_restore`.

---

## 1. Cambios Implementados

### 1.1 Corrección Financiera en Extensiones de Renta
- **Archivos:** [renta.rs](file:///d:/dinamo_rent_tr/src-tauri/src/services/renta.rs) y [rentas_integration.rs](file:///d:/dinamo_rent_tr/src-tauri/tests/rentas_integration.rs)
- **Problema:** Al extender una renta (horas o días), el sistema sumaba los días a `dias_calculados` e incrementaba el subtotal (`vdia * nuevo_dias`), y **al mismo tiempo** sumaba el valor total de la extensión a `valor_dia_extra` (que también se incluía en `extras`), cobrando la extensión por duplicado a tarifas divergentes.
- **Solución:**
  - Se preservan `actual.dias_calculados` y `actual.horas_extras` como base contractual del contrato.
  - El costo de las extensiones se acumula exclusivamente en `valor_dia_extra` y en la tabla `extensiones_renta`.
  - Las fechas/horas de retorno (`fecha_retorno`, `hora_retorno`) se actualizan a la nueva fecha de devolución para fines de disponibilidad y calendario.
  - Esto garantiza total coherencia con `calcular_totales` y la plantilla de impresión [ContratoRenta.svelte](file:///d:/dinamo_rent_tr/src/lib/components/reports/ContratoRenta.svelte).

### 1.2 Corrección de Desfase de Zona Horaria en Informes
- **Archivos:** [format.ts](file:///d:/dinamo_rent_tr/src/lib/utils/format.ts), [format.test.ts](file:///d:/dinamo_rent_tr/src/lib/utils/format.test.ts) y [+page.svelte](file:///d:/dinamo_rent_tr/src/routes/informes/+page.svelte)
- **Problema:** El selector de rango de fechas usaba `new Date().toISOString().split('T')[0]`. En Colombia (`UTC-5`), a partir de las 7:00 PM hora local, UTC entra en el día siguiente, causando que el filtro de fecha fin se inicializara con la fecha de mañana.
- **Solución:** Se implementó `formatLocalDateISO(date)` que extrae año, mes y día en hora local con padding adecuado, reemplazando el uso de `toISOString()` en la pantalla de informes.

### 1.3 Eliminación de Ruido en Vitest
- **Archivo:** [setup.ts](file:///d:/dinamo_rent_tr/src/test/setup.ts)
- **Problema:** Componentes globales que leen `app_version` generaban excepciones no interceptadas en consola durante la ejecución de pruebas debido a la ausencia de un mock por defecto.
- **Solución:** Se agregó un fallback en el mock de `invoke` para responder con `'1.2.1'` cuando se consulte `app_version`.

### 1.4 Salvaguarda Pre-Restauración de Base de Datos
- **Archivo:** [backup.rs](file:///d:/dinamo_rent_tr/src-tauri/src/services/backup.rs)
- **Problema:** Si un operador restauraba accidentalmente un archivo de respaldo desactualizado o vacío, la base de datos viva se sobrescribía sin opción de recuperación inmediata.
- **Solución:** En `restaurar_fdb_desde_fbk`, antes de aplicar el reemplazo atómico, se genera una copia preventiva en disco con timestamp (`<db_path>.pre_restore_<timestamp>.bak`).

---

## 2. Resultados de Verificación y Pruebas

| Validación | Comando | Resultado |
|---|---|---|
| **Frontend Diagnostics** | `npm run check` | **0 errors, 0 warnings** |
| **Frontend Tests** | `npm test` | **28 test files pasaron (254 pruebas pasadas)** |
| **Formato y Fechas** | `npm test -- src/lib/utils/format.test.ts` | **20 pruebas pasaron** |
| **Backend Rust Check** | `cargo check` | **Compilación OK (código 0)** |
| **Test de Integración de Extensión** | `cargo test --test rentas_integration renta_extender_horas_y_dias` | **1 passed; 0 failed (ok)** |
| **Integración de Rentas (completa)** | `cargo test --test rentas_integration` | **14 passed; 0 failed** |
| **Integración de Backups** | `cargo test --test backup_integration` | **4 passed; 0 failed** |
| **Unit Tests de Backups** | `cargo test --lib services::backup` | **20 passed; 0 failed** |
| **Suite completa de Rust** | `cargo test` | **174 passed; 0 failed (1 ignored: test manual del portal SIMIT)** |
| **Lints Rust** | `cargo clippy --all-targets -- -D warnings` | **0 errores, 0 warnings** |
| **Formato Rust** | `cargo fmt -- --check` | **OK (sin diferencias)** |

---

## 3. Tercera Ronda de QA (hardening y gate de calidad)

### 3.1 Historial de extensiones verificado de punta a punta
- **Cadena validada** (sin cambios necesarios): comando Tauri `listar_extensiones` → `ExtensionRentaRepository::listar_por_renta` (tabla `extensiones_renta`) → `rentaApi.listarExtensiones()` → carga en [rentas/+page.svelte](file:///d:/dinamo_rent_tr/src/routes/rentas/+page.svelte) al abrir el modal → render en [ModalExtenderRenta.svelte](file:///d:/dinamo_rent_tr/src/routes/rentas/components/ModalExtenderRenta.svelte) con `+Nh`/`+Nd`, valor total COP, usuario, fecha y observaciones.
- **Valor para el operador:** transparencia total del acumulado cobrado por extensiones antes de aplicar una nueva (evita dobles cobros percibidos y discusiones con el cliente).

### 3.2 `cargo clippy -D warnings` consolidado como verificación obligatoria
- **Estado:** el código ya pasa limpio con `--all-targets -- -D warnings` (0 avisos corregidos).
- **Brechas cerradas para que el gate sea consistente en todos los flujos:**
  - [scripts/test-completo.sh](file:///d:/dinamo_rent_tr/scripts/test-completo.sh): se **añadió** el paso de clippy (antes no lo ejecutaba en absoluto) con `--all-targets -- -D warnings`.
  - [.husky/pre-commit](file:///d:/dinamo_rent_tr/.husky/pre-commit): se añadió `--all-targets` (ahora linta también el código de tests, igual que el CI).
  - [CONTRIBUTING.md](file:///d:/dinamo_rent_tr/CONTRIBUTING.md): corregido bug de ruta (el comando corría tras `cd ..`, donde no hay `Cargo.toml`), añadido `--all-targets` y marcado como obligatorio.
  - [.github/PULL_REQUEST_TEMPLATE.md](file:///d:/dinamo_rent_tr/.github/PULL_REQUEST_TEMPLATE.md): checklist del PR actualizado a `--all-targets -- -D warnings`.
  - El CI ([ci.yml](file:///d:/dinamo_rent_tr/.github/workflows/ci.yml)) ya ejecutaba la variante estricta: sin cambios.

### 3.3 Suite completa de Rust en verde
`cargo test` completo (unit + las 17 suites de integración contra BD real con gbak): **174 passed, 0 failed** (92 lib + 82 de integración: rentas 14, backups 4, migraciones 11, auth 5, informes 5, comparendos 8, autos/clientes 4, gastos 4, mantenimiento 4, auditoría 3, y el resto). Único ignored: test manual del portal SIMIT (requiere token real).
