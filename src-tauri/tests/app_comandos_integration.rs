//! app_comandos_integration.rs — Pruebas de integración del comando
//! `app_version` (commands/app.rs): debe devolver la versión REAL de la app
//! (Cargo.toml / tauri.conf.json en el build) y NUNCA el literal v3.2.0
//! heredado del proyecto anterior.
//!
//! A diferencia del resto de comandos, `app_version` solo recibe `AppHandle`
//! (no toca BD ni sesión), así que basta un mock de Tauri. `mock_context`
//! hardcodea una versión dummy (0.1.0), por eso aquí se usa
//! `tauri::generate_context!()` — la misma config real embebida que usa la
//! app en producción (lib.rs) — para que `package_info().version` sea la real.

use tauri::test::mock_builder;

use dinamo_rent_lib::commands::app::app_version;

#[test]
fn app_version_devuelve_la_version_de_cargo_toml() {
    let app = mock_builder()
        .build(tauri::generate_context!())
        .expect("mock app con contexto real");

    let version = app_version(app.handle().clone());

    // La versión del crate en el build (1.0.21 hoy; la misma que tauri.conf.json
    // y la que firma el updater — el checklist de release exige que coincidan).
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "app_version debe devolver la versión de Cargo.toml del build"
    );
    // Regresión: el UI mostró v3.2.0 (literal heredado del proyecto anterior).
    assert_ne!(version, "3.2.0", "nunca debe devolver el literal 3.2.0");
    assert!(!version.is_empty());
}
