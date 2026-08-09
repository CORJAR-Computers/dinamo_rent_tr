fn main() {
    tauri_build::build();

    // Los binarios de test que enlazan el stack de Tauri (wry/webview2) importan
    // TaskDialogIndirect de comctl32.dll. La v5.82 de System32 NO exporta ese
    // símbolo (vive en la v6 vía SxS), así que el exe de test muere al arrancar
    // con STATUS_ENTRYPOINT_NOT_FOUND a menos que tenga el mismo manifest de
    // Common Controls v6 que Tauri incrusta en la app. Lo incrustamos en todos
    // los targets de test con cargo:rustc-link-arg-tests (flag de MSVC).
    #[cfg(all(windows, target_env = "msvc"))]
    embed_test_manifest();

    // Con toolchain GNU el flag /MANIFEST:EMBED no aplicaría y los tests de
    // Tauri morirían con ENTRYPOINT_NOT_FOUND de forma confusa. Mejor fallar
    // en compilación con un mensaje claro.
    #[cfg(all(windows, not(target_env = "msvc")))]
    compile_error!(
        "Los tests que usan tauri::test en Windows requieren la toolchain MSVC \
         (el manifest de Common Controls v6 se incrusta con flags del linker MSVC)"
    );
}

#[cfg(all(windows, target_env = "msvc"))]
fn embed_test_manifest() {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), r"\tests-common-controls.manifest");
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}", manifest);
}
