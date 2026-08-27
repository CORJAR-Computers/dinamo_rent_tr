//! updater_e2e — Herramienta de DESARROLLO
//!
//! Verifica de punta a punta el flujo de auto-actualización
//! (`tauri-plugin-updater`) contra un endpoint que sirve un `latest.json`
//! firmado con la clave real de firma (`~/.tauri/dinamorent.key`).
//!
//! Uso:
//!   cargo run --features dev --bin updater_e2e -- --endpoint <URL> \
//!       [--expect-version <v>] [--expect-none] [--expect-file <path>]
//!
//! - `--endpoint <URL>`      → URL del `latest.json` a consultar.
//! - `--expect-version <v>`  → `check()` debe devolver una actualización v<v>.
//! - `--expect-none`         → `check()` debe devolver «sin actualización».
//! - `--expect-file <path>`  → además descarga el artifact y verifica que la
//!                             firma valide contra la pubkey embebida en
//!                             `tauri.conf.json` y que los bytes coincidan
//!                             exactamente con el archivo indicado.
//!
//! Códigos de salida: 0 = verificación OK · 1 = falló algo · 2 = uso incorrecto.
//!
//! ⚠️ Herramienta de desarrollo: solo se compila con `--features dev` y el
//!    `main` está detrás de `#[cfg(debug_assertions)]` (defense-in-depth).

use tauri_plugin_updater::UpdaterExt;

const USO: &str = "\
Uso:
  cargo run --features dev --bin updater_e2e -- --endpoint <URL> [--expect-version <v>] [--expect-none] [--expect-file <path>]

Opciones:
  --endpoint <URL>      URL del latest.json a consultar (obligatorio)
  --expect-version <v>  check() debe detectar una actualización v<v>
  --expect-none         check() debe devolver 'sin actualización'
  --expect-file <path>  además descarga el artifact, verifica la firma
                        (contra la pubkey embebida) y que los bytes coincidan
  --ayuda | -h          muestra esta ayuda";

struct Args {
    endpoint: String,
    expect_version: Option<String>,
    expect_none: bool,
    expect_file: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut endpoint = None;
    let mut expect_version = None;
    let mut expect_none = false;
    let mut expect_file = None;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--endpoint" => {
                endpoint = Some(
                    it.next()
                        .ok_or("--endpoint requiere un URL como argumento")?,
                );
            }
            "--expect-version" => {
                expect_version = Some(it.next().ok_or("--expect-version requiere un valor")?);
            }
            "--expect-none" => expect_none = true,
            "--expect-file" => {
                expect_file = Some(it.next().ok_or("--expect-file requiere una ruta")?);
            }
            "--ayuda" | "-h" => {
                println!("{USO}");
                std::process::exit(0);
            }
            otro => return Err(format!("argumento desconocido: {otro}")),
        }
    }

    if expect_none && expect_version.is_some() {
        return Err("--expect-none y --expect-version son mutuamente excluyentes".into());
    }

    let endpoint = endpoint.ok_or("falta --endpoint <URL> (ver --ayuda)")?;

    Ok(Args {
        endpoint,
        expect_version,
        expect_none,
        expect_file,
    })
}

/// Implementación real (solo debug).
#[cfg(debug_assertions)]
fn main() -> Result<(), String> {
    let args = parse_args()?;

    // App Tauri headless con el plugin updater registrado: el plugin lee la
    // configuración real embebida por `tauri::generate_context!()` (la pubkey
    // de tauri.conf.json). Los endpoints se sobrescriben con el local para la
    // prueba; la pubkey sigue siendo la de producción → la verificación de
    // firma en download() es la MISMA que usa la app instalada.
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(tauri::generate_context!())
        .map_err(|e| format!("no se pudo construir la app Tauri headless: {e}"))?;

    let updater = app
        .handle()
        .updater_builder()
        .endpoints(vec![args
            .endpoint
            .parse()
            .map_err(|e| format!("URL inválida: {e}"))?])
        .map_err(|e| format!("el plugin rechazó el endpoint: {e}"))?
        .build()
        .map_err(|e| format!("no se pudo construir el updater: {e}"))?;

    println!("[1/2] check() contra {}", args.endpoint);
    let update = tauri::async_runtime::block_on(updater.check())
        .map_err(|e| format!("check() falló: {e}"))?;

    match update {
        None => {
            println!("      → sin actualización disponible");
            if args.expect_none {
                println!("[OK] se esperaba y se obtuvo: sin actualización");
                return Ok(());
            }
            if let Some(v) = &args.expect_version {
                return Err(format!(
                    "[FAIL] se esperaba actualización v{v} pero no hay ninguna"
                ));
            }
            println!("[OK] (modo exploratorio, sin expectativa)");
            Ok(())
        }
        Some(u) => {
            println!(
                "      → actualización detectada: v{} (instalada: v{})",
                u.version, u.current_version
            );
            println!(
                "        notas: {}",
                u.body.as_deref().unwrap_or("(sin notas)")
            );
            println!("        url:   {}", u.download_url);

            if let Some(v) = &args.expect_version {
                if &u.version != v {
                    return Err(format!(
                        "[FAIL] se esperaba v{v} pero se detectó v{}",
                        u.version
                    ));
                }
            }
            if args.expect_none {
                return Err(format!(
                    "[FAIL] se esperaba 'sin actualización' pero se detectó v{}",
                    u.version
                ));
            }

            if let Some(path) = &args.expect_file {
                println!("[2/2] download() + verificación de firma (pubkey de producción)…");
                let bytes = tauri::async_runtime::block_on(u.download(|_, _| {}, || {}))
                    .map_err(|e| format!("download()/verificación de firma falló: {e}"))?;

                let expected =
                    std::fs::read(path).map_err(|e| format!("no se pudo leer {path}: {e}"))?;
                if bytes.len() != expected.len() {
                    return Err(format!(
                        "[FAIL] tamaño del artifact distinto: descargado {} B vs servido {} B",
                        bytes.len(),
                        expected.len()
                    ));
                }
                if bytes != expected {
                    return Err(
                        "[FAIL] los bytes descargados no coinciden con el artifact servido"
                            .to_string(),
                    );
                }
                println!(
                    "      → firma verificada y bytes idénticos al artifact ({} B)",
                    bytes.len()
                );
            }

            println!("[OK] check() detectó la v{} correctamente", u.version);
            Ok(())
        }
    }
}

/// Stub de release: esta herramienta no debe llegar a producción.
#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("updater_e2e es una herramienta de desarrollo (solo debug).");
    std::process::exit(1);
}
