//! sync_dev — Herramienta de DESARROLLO
//!
//! Ejecuta la sincronización SIMIT de punta a punta contra la BD de desarrollo
//! (data/dinamo_rent_v3.fdb) SIN Tauri: config → pool → migraciones → run_sync
//! (el mismo camino que «Sincronizar ahora» en la UI, con app=None).
//!
//! Uso: `cargo run --features dev --bin sync_dev`
//!
//! Salidas:
//!   - stdout: resumen legible + snapshot de la BD antes/después
//!   - data/simit_watch/sync_result.json: resultado serializado (para los
//!     verificadores Node: Excel, watch, etc.)
//!   - data/informes_simit/simit_*.html: reporte HTML (lo genera run_sync)
//!
//! ⚠️ Escribe registros en la BD de desarrollo (tabla comparendos). No correr
//! en producción. Solo debug (mismo mecanismo que dev_reset_admin).

use std::path::PathBuf;
use std::sync::Arc;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::db::{create_pool, PooledConnection};
use dinamo_rent_lib::repositories::auto::AutoRepository;
use dinamo_rent_lib::services::simit::{run_sync, EstadoAgenteSimit};
use rsfbclient::Queryable;
use tauri::AppHandle;

/// Logger mínimo a stderr (el bin no arranca Tauri, así que `log::` no
/// tiene receptor por defecto). Muestra el progreso por placa y los avisos.
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        eprintln!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}

/// Snapshot del estado de la tabla comparendos (para validar insert/estado)
struct Snapshot {
    total: i64,
    pendientes: i64,
    pagados: i64,
    /// Suma de los montos pendientes (deuda real de la flota en la BD)
    suma_pendiente: f64,
    /// Comparendos con renta atribuida (id_renta no nulo)
    atribuidos: i64,
}

fn snapshot(conn: &mut PooledConnection) -> Result<Snapshot, Box<dyn std::error::Error>> {
    // El monto se devuelve CAST a VARCHAR (rsfbclient no soporta DECIMAL directo,
    // convención del repo) y se parsea a f64 en Rust.
    let row: Option<(i64, i64, i64, String, i64)> = conn.query_first(
        "SELECT COUNT(*), \
         COALESCE(SUM(CASE WHEN estado = 'Pendiente' THEN 1 ELSE 0 END), 0), \
         COALESCE(SUM(CASE WHEN estado = 'Pagado' THEN 1 ELSE 0 END), 0), \
         CAST(COALESCE(SUM(CASE WHEN estado = 'Pendiente' THEN monto ELSE 0 END), 0) AS VARCHAR(20)), \
         COALESCE(SUM(CASE WHEN id_renta IS NOT NULL THEN 1 ELSE 0 END), 0) \
         FROM comparendos WHERE deleted_at IS NULL",
        (),
    )?;
    let (total, pendientes, pagados, suma_raw, atribuidos) = row.unwrap_or((0, 0, 0, "0".into(), 0));
    let suma_pendiente = suma_raw.trim().parse::<f64>().unwrap_or(0.0);
    Ok(Snapshot {
        total,
        pendientes,
        pagados,
        suma_pendiente,
        atribuidos,
    })
}

/// Implementación real (sólo debug).
#[cfg(debug_assertions)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `--solo-total`: solo imprime el estado de la BD dev y sale (sin tocar el
    // portal ni escribir registros) — útil para comparar con la sonda.
    let solo_total = std::env::args().any(|a| a == "--solo-total");

    // Mismas rutas de desarrollo que lib.rs (debug_assertions)
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let cfg = Arc::new(AppConfig::load(&data_dir, &resource_dir, &manifest));

    let pool = create_pool(&cfg)?;
    // Migraciones idempotentes (mismo arranque que lib.rs)
    dinamo_rent_lib::core::migrations::run_migrations(&pool, &manifest.join("migrations"))?;

    let mut conn = pool.get()?;

    let placas = AutoRepository::placas_activas(&mut conn)?;
    let snap = snapshot(&mut conn)?;
    println!("== Placas activas ({}): {}", placas.len(), placas.join(", "));
    println!(
        "== Comparendos: total={} pendientes={} pagados={} atribuidos={} suma_pendiente=${:.2}",
        snap.total, snap.pendientes, snap.pagados, snap.atribuidos, snap.suma_pendiente
    );

    if solo_total {
        // ¿La placa de prueba de la sonda (AAA000) está en la flota?
        let (cnt,): (i64,) = conn
            .query_first("SELECT COUNT(*) FROM autos WHERE placa = ?", ("AAA000",))?
            .unwrap_or((0,));
        println!("== AAA000 en autos: {}", if cnt > 0 { "SÍ" } else { "NO" });
        // Diagnóstico de la atribución (cruce comparendos↔rentas)
        let (m16,): (i64,) = conn
            .query_first(
                "SELECT COUNT(*) FROM schema_migrations \
                 WHERE version = '0016_atribucion_comparendo_renta.sql'",
                (),
            )?
            .unwrap_or((0,));
        let (rentas,): (i64,) = conn
            .query_first("SELECT COUNT(*) FROM rentas WHERE deleted_at IS NULL", ())?
            .unwrap_or((0,));
        println!("== Migración 0016 (backfill atribución): {}", if m16 > 0 { "APLICADA" } else { "pendiente" });
        println!("== Rentas activas (deleted_at nulo): {}", rentas);
        println!("(modo --solo-total: no se tocó el portal ni la BD)");
        return Ok(());
    }

    let antes = snap;

    let _ = log::set_boxed_logger(Box::new(StderrLogger));
    log::set_max_level(log::LevelFilter::Info);

    let estado = EstadoAgenteSimit::default();
    match run_sync(&pool, &cfg, &estado, None::<&AppHandle>) {
        Ok(resultado) => {
            let despues = snapshot(&mut conn)?;
            println!("\n== RESULTADO SINCRONIZACIÓN ==");
            println!("  placas_consultadas  : {}", resultado.placas_consultadas);
            println!("  encontrados         : {}", resultado.encontrados);
            println!("  insertados (nuevos) : {}", resultado.insertados);
            println!("  duplicados          : {}", resultado.duplicados);
            println!("  placas_con_error    : {}", resultado.placas_con_error);
            println!("  total_pendiente     : ${}", resultado.total_pendiente);
            println!(
                "  reporte_html        : {}",
                resultado.reporte_html.as_deref().unwrap_or("(no generado)")
            );
            println!(
                "  tiempo_total_ms     : {} (captcha {} ms · consulta {} ms · reintentos {})",
                resultado.metricas.tiempo_total_ms,
                resultado.metricas.tiempo_captcha_ms,
                resultado.metricas.tiempo_consulta_ms,
                resultado.metricas.total_reintentos
            );
            println!(
                "  circuit_breaker     : {}",
                resultado.metricas.circuit_breaker_state
            );
            if !resultado.errores.is_empty() {
                println!("  errores por placa:");
                for e in &resultado.errores {
                    println!("    - {}: {}", e.placa, e.error);
                }
            }
            println!(
                "\n== Comparendos DESPUÉS: total={} pendientes={} pagados={} atribuidos={} suma_pendiente=${:.2} (delta total {:+})",
                despues.total,
                despues.pendientes,
                despues.pagados,
                despues.atribuidos,
                despues.suma_pendiente,
                despues.total - antes.total
            );

            // Dump JSON para los verificadores Node (Excel, watch, etc.)
            let json = serde_json::to_string_pretty(&resultado)?;
            let out = data_dir.join("simit_watch").join("sync_result.json");
            std::fs::create_dir_all(out.parent().unwrap())?;
            std::fs::write(&out, json)?;
            println!("\nJSON completo → {}", out.display());
        }
        Err(e) => {
            eprintln!("\n❌ La sincronización falló: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Stub de release (defense-in-depth, igual que dev_reset_admin)
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("sync_dev solo está disponible en builds de debug.");
    eprintln!("Compila con: cargo run --features dev --bin sync_dev");
    std::process::exit(1);
}
