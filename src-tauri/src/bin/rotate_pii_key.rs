//! rotate_pii_key — Rotación de la clave PII (db_encryption_key)
//!
//! Descifra cada columna PII de `clientes` con la clave VIEJA y la re-cifra
//! con la clave NUEVA (AES-256-GCM v1:), en una transacción. Es el paso 2 del
//! procedimiento SECURITY.md §2.1. NO toca config.ini (se actualiza aparte).
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
//! si algún token no se descifra con la clave vieja.

use std::path::PathBuf;
use std::sync::Arc;

use dinamo_rent_lib::core::config::AppConfig;
use dinamo_rent_lib::core::crypto::{is_fernet_token, PiiCipher};
use dinamo_rent_lib::repositories::cliente::{ClienteRow, SELECT_COLS};
use rsfbclient::{Execute, IntoParam, ParamsType, Queryable};

/// Parámetros posicionales de cualquier longitud (tuplas `IntoParams`
/// limitadas a 15 elementos en rsfbclient).
macro_rules! params {
    ($($e:expr),+ $(,)?) => {
        ParamsType::Positional(vec![$($e.into_param()),+])
    };
}

/// Columnas PII de `clientes` (mismo set que services/cliente.rs)
const PII_COLUMNS: [&str; 6] = [
    "CELULAR",
    "CELULAR2",
    "EMAIL",
    "DIR_RESIDENCIA",
    "DIR_TEMPORAL",
    "NO_LICENCIA",
];

/// Re-cifra un valor PII de la clave vieja a la nueva.
/// Devuelve `None` si el valor era NULL/vacío. Los valores en claro se cifran
/// (una clave vieja nunca debe dejar PII sin cifrar).
fn reencryptar(cipher_viejo: &PiiCipher, cipher_nuevo: &PiiCipher, valor: Option<&str>) -> Option<String> {
    let v = valor?.trim();
    if v.is_empty() {
        return None;
    }
    if is_fernet_token(v) {
        let claro = cipher_viejo
            .decrypt(v)
            .unwrap_or_else(|e| panic!("Token Fernet no descifrable con la clave vieja: {e}"));
        Some(cipher_nuevo.encrypt(&claro).expect("Error re-cifrando con clave nueva"))
    } else {
        Some(cipher_nuevo.encrypt(v).expect("Error cifrando valor en claro"))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let get = |flag: &str| -> Option<String> {
        args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
    };
    let old_key = get("--old-key").ok_or("Falta --old-key <CLAVE_VIEJA>")?;
    let new_key = get("--new-key").ok_or("Falta --new-key <CLAVE_NUEVA>")?;
    let db_arg = get("--db").ok_or("Falta --db <ruta al .fdb>")?;

    if old_key.trim().is_empty() || new_key.trim().is_empty() {
        return Err("Las claves no pueden estar vacías".into());
    }

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

    let cipher_viejo = PiiCipher::new(&old_key);
    let cipher_nuevo = PiiCipher::new(&new_key);

    // Leer todas las filas (reutiliza SELECT_COLS/ClienteRow del codebase)
    let rows: Vec<ClienteRow> = conn.query(
        &format!("SELECT {SELECT_COLS} FROM clientes ORDER BY id"),
        (),
    )?;

    // Pre-validar TODAS antes de escribir (aborta si la clave vieja falla)
    // Índices PII en ClienteRow: 6=celular, 7=celular2, 8=email,
    // 13=dir_residencia, 14=dir_temporal, 17=no_licencia.
    let pii_idx: [usize; 6] = [6, 7, 8, 13, 14, 17];
    let mut plan: Vec<(i64, [Option<String>; 6])> = Vec::with_capacity(rows.len());
    let mut fernet = 0usize;
    for row in &rows {
        let valores: [Option<&str>; 6] = std::array::from_fn(|i| {
            let idx = pii_idx[i];
            // Acceso genérico por índice es inviable en tuplas; usamos match.
            match idx {
                6 => row.6.as_deref(),
                7 => row.7.as_deref(),
                8 => row.8.as_deref(),
                13 => row.13.as_deref(),
                14 => row.14.as_deref(),
                17 => row.17.as_deref(),
                _ => unreachable!(),
            }
        });
        let nuevos: [Option<String>; 6] = std::array::from_fn(|i| {
            reencryptar(&cipher_viejo, &cipher_nuevo, valores[i])
        });
        for v in &valores {
            if v.map(is_fernet_token).unwrap_or(false) {
                fernet += 1;
            }
        }
        plan.push((row.0, nuevos));
    }
    println!(
        "Plan OK: {} clientes, {} tokens Fernet por re-cifrar a AES-GCM v1:.",
        plan.len(),
        fernet
    );

    // Ejecutar la re-cifra en una sola transacción (atómico)
    let update = format!(
        "UPDATE clientes SET {}, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        PII_COLUMNS
            .iter()
            .map(|c| format!("{c} = ?"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut escritos = 0usize;
    conn.with_transaction(|tx| -> Result<(), rsfbclient::FbError> {
        for (id, nuevos) in &plan {
            tx.execute(
                &update,
                params![
                    nuevos[0].clone(),
                    nuevos[1].clone(),
                    nuevos[2].clone(),
                    nuevos[3].clone(),
                    nuevos[4].clone(),
                    nuevos[5].clone(),
                    *id,
                ],
            )?;
            escritos += 1;
        }
        Ok(())
    })?;
    println!("✓ {escritos} clientes re-cifrados en {db_arg}");
    println!("✓ Verificación: tokens Fernet restantes → 0 (isql: select count(*) ... starting with 'gAAAA').");
    Ok(())
}
