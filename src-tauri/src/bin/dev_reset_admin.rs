//! dev_reset_admin — Herramienta de DESARROLLO
//!
//! Resetea la contraseña del usuario admin a una conocida (Argon2id).
//! Uso: cargo run --bin dev_reset_admin -- <nueva_password>
//! Se usa sobre la BD de desarrollo (data/dinamo_rent_v3.fdb).

use std::path::PathBuf;
use std::sync::Arc;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::security;
use dinamo_rent_lib::repositories::usuario::UsuarioRepository;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let password = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "Admin123!".to_string());

    // Localiza data_dir: primero D:/dinamo_rent_tr/data, sino CARGO_MANIFEST_DIR/../data
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = manifest.join("../data");
    let resource_dir = manifest.join("resources");
    let cfg = Arc::new(AppConfig::load(&data_dir, &resource_dir, &manifest));

    let pool = dinamo_rent_lib::core::db::create_pool(&cfg)?;
    let mut conn = pool.get()?;

    let usuario = UsuarioRepository::obtener_para_autenticacion(&mut conn, "admin")?;
    match usuario {
        Some(_) => {
            let hash = security::hash_password(&password)?;
            UsuarioRepository::actualizar_password(&mut conn, "admin", &hash)?;
            println!("✓ Contraseña de 'admin' actualizada a Argon2id (password: {password})");
        }
        None => {
            let hash = security::hash_password(&password)?;
            UsuarioRepository::insertar(
                &mut conn,
                "admin",
                &hash,
                "Administrador Principal",
                "Administrador",
                false,
            )?;
            println!("✓ Usuario 'admin' creado (password: {password})");
        }
    }
    Ok(())
}
