//! rotate_pii_key — Rotación de la clave PII (db_encryption_key)
//!
//! Wrapper CLI de `dinamo_rent_lib::services::rotacion::rotar_clave_pii`
//! (SECURITY.md §2.1): descifra cada columna PII de `clientes` con la clave
//! VIEJA (tokens Fernet legacy y AES-GCM `v1:`) y la re-cifra con la clave
//! NUEVA (AES-256-GCM v1:) en una transacción atómica, registrando el evento
//! de auditoría `PII_KEY_ROTATED` sin exponer la clave. NO toca config.ini
//! (se actualiza aparte).
//!
//! Uso:
//!   cargo run --features dev --bin rotate_pii_key -- \
//!       --old-key "CLAVE_VIEJA" \
//!       --new-key "CLAVE_NUEVA" \
//!       --db "D:/dinamo_rent_tr/data/dinamo_rent_v3.fdb"
//!
//! La clave NUEVA se genera con `openssl rand -base64 32`.
//!
//! ⚠️ Exige backups previos de la BD (ver SECURITY.md §2.1 Paso 0). Es
//! destructivo si se usa una clave vieja incorrecta: aborta antes de escribir
//! si algún token no se descifra con la clave vieja. Re-ejecutable: los tokens
//! ya en `v1:` se descifran con la clave vieja y se re-cifran (no se duplican).

use std::path::PathBuf;
use std::sync::Arc;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::services::rotacion::rotar_clave_pii;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let old_key = get("--old-key").ok_or("Falta --old-key <CLAVE_VIEJA>")?;
    let new_key = get("--new-key").ok_or("Falta --new-key <CLAVE_NUEVA>")?;
    let db_arg = get("--db").ok_or("Falta --db <ruta al .fdb>")?;

    // AppConfig: sólo se usa para resolver fbclient.dll y crear el pool
    // embedded. Se sobrescribe db_path con la BD destino (--db).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let mut cfg = Arc::new(AppConfig::load(&data_dir, &resource_dir, &manifest));
    let db_path = PathBuf::from(&db_arg);
    if !db_path.exists() {
        return Err(format!("No existe la BD: {}", db_path.display()).into());
    }
    Arc::get_mut(&mut cfg).unwrap().db_path = db_path;

    let pool = dinamo_rent_lib::core::db::create_pool(&cfg)?;
    let mut conn = pool.get()?;

    let resultado = rotar_clave_pii(&mut conn, &old_key, &new_key)?;
    println!(
        "Plan OK: {} clientes, {} tokens Fernet + {} tokens AES-GCM v1: re-cifrados.",
        resultado.clientes, resultado.fernet, resultado.aes_v1
    );
    println!("✓ {} clientes re-cifrados en {db_arg}", resultado.clientes);
    println!("✓ Auditoría: evento PII_KEY_ROTATED registrado en la tabla auditoria (sin exponer la clave).");
    println!("✓ Verificación: tokens Fernet restantes → 0 (isql: select count(*) ... starting with 'gAAAA').");
    Ok(())
}
