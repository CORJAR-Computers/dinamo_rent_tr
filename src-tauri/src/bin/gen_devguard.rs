//! gen_devguard — Genera el mapa de contratos del guardrail de desarrollo
//! (`src/lib/api/devGuard.generated.ts`) clasificando los campos de cada DTO
//! de entrada **comportamentalmente**: por cada campo hace una sonda serde
//! real (`999`, `"1"`) y registra si el DTO acepta o rechaza el tipo. No hay
//! listas manuales que desfasen: lo que produce el binario es la fuente de
//! verdad y el test `devGuard.sync.test.ts` valida la huella de las fuentes.
//!
//! Uso: `cargo run --features dev --bin gen_devguard`
//! (desde `src-tauri/`; escribe el .ts en el frontend del repo).
//!
//! Solo se compila con `--features dev` (mismo patrón que sync_dev).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use dinamo_rent_lib::repositories::auto::AutoDatos;
use dinamo_rent_lib::repositories::cliente::ClienteDatos;
use dinamo_rent_lib::repositories::comparendo::ComparendoDatos;
use dinamo_rent_lib::repositories::empresa::EmpresaConfigDatos;
use dinamo_rent_lib::repositories::gasto::GastoDatos;
use dinamo_rent_lib::repositories::mantenimiento::MantenimientoDatos;
use dinamo_rent_lib::repositories::renta::{
    ExtensionDatos, InspeccionDatos, PagoDatos, RentaCierreDatos, RentaCierreEditDatos, RentaDatos,
};
use dinamo_rent_lib::repositories::reserva::ReservaDatos;
use dinamo_rent_lib::services::usuario::{UsuarioDatos, UsuarioDatosActualizar};

/// Sonda: ¿el DTO (instancia default) acepta este JSON con el campo reemplazado?
fn acepta<T>(default: &T, payload: &Value) -> bool
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    // Serializa el default, fusiona el payload (reemplaza el campo de la
    // sonda) y deserializa de vuelta.
    let mut base = serde_json::to_value(default).expect("serializar DTO default");
    let obj = base.as_object_mut().expect("DTO debe ser objeto");
    if let Value::Object(cambios) = payload {
        for (k, v) in cambios {
            obj.insert(k.clone(), v.clone());
        }
    }
    serde_json::from_value::<T>(base).is_ok()
}

/// Clasifica cada campo con sondas serde reales sobre una instancia default.
/// Clases (por aceptación de `999` y de `"1"`):
/// - ambos → `decimal_string`/`option_decimal_string` (solo ellos aceptan
///   número Y string; un `String` nativo rechaza el número) → mapa `strings`.
/// - solo número → `i64`/`f64` nativo u `Option<i64>` → mapa `numeros`.
/// - solo string (texto plano: placa, fechas, nombres) → fuera del guardrail
///   (alcance §4.12: monetarios y numéricos).
macro_rules! clasificar {
    ($tipo:ty) => {{
        let mut aceptan_numero: Vec<String> = Vec::new();
        let mut strings: Vec<String> = Vec::new();
        let default = <$tipo>::default();
        let nombre_campos = serde_json::to_value(&default)
            .expect("serializar default")
            .as_object()
            .expect("objeto")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for campo in nombre_campos {
            let con_numero = acepta(&default, &json!({ &campo: 999 }));
            let con_string = acepta(&default, &json!({ &campo: "1" }));
            if con_numero && con_string {
                // decimal_string: el único que tolera ambas representaciones
                strings.push(campo);
            } else if con_numero && !con_string {
                // i64/f64 nativo (u Option<i64>): el número es su tipo
                aceptan_numero.push(campo);
            }
        }
        (aceptan_numero, strings)
    }};
}

fn main() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("raíz del repo")
        .to_path_buf();
    let destino = raiz.join("src/lib/api/devGuard.generated.ts");

    let mut comandos: BTreeMap<&str, (&str, Vec<String>, Vec<String>)> = BTreeMap::new();

    macro_rules! registrar {
        ($comandos:expr, $dto:ty) => {{
            let (nums, strs) = clasificar!($dto);
            for c in $comandos {
                comandos.insert(c, (stringify!($dto), nums.clone(), strs.clone()));
            }
        }};
    }

    registrar!(["crear_renta", "actualizar_renta"], RentaDatos);
    registrar!(["cerrar_renta"], RentaCierreDatos);
    registrar!(["editar_renta_cerrada"], RentaCierreEditDatos);
    registrar!(["extender_renta"], ExtensionDatos);
    registrar!(["registrar_pago_renta"], PagoDatos);
    registrar!(["registrar_inspeccion_renta"], InspeccionDatos);
    registrar!(["crear_reserva", "actualizar_reserva"], ReservaDatos);
    registrar!(["crear_gasto", "actualizar_gasto"], GastoDatos);
    registrar!(
        ["crear_comparendo", "actualizar_comparendo"],
        ComparendoDatos
    );
    registrar!(
        ["crear_mantenimiento", "actualizar_mantenimiento"],
        MantenimientoDatos
    );
    registrar!(["crear_auto", "actualizar_auto"], AutoDatos);
    registrar!(["crear_cliente", "actualizar_cliente"], ClienteDatos);
    registrar!(["guardar_empresa"], EmpresaConfigDatos);
    registrar!(["crear_usuario"], UsuarioDatos);
    registrar!(["actualizar_usuario"], UsuarioDatosActualizar);

    // Serializa el resultado como TS
    let mut ts = String::new();
    ts.push_str("// GENERATED FILE — NO EDITAR A MANO.\n");
    ts.push_str("// Generado por `cargo run --features dev --bin gen_devguard`\n");
    ts.push_str("// (clasificación conductual: sondas serde reales sobre los DTOs de entrada).\n");
    ts.push_str("// El test devGuard.sync.test.ts valida que este archivo esté al día.\n\n");
    ts.push_str("export const MAPA_GENERADO: Record<string, { numeros: readonly string[]; strings: readonly string[] }> = {\n");
    for (comando, (dto, nums, strs)) in &comandos {
        ts.push_str(&format!("\t{comando}: {{\n"));
        ts.push_str(&format!("\t\t// DTO: {dto}\n"));
        ts.push_str("\t\tnumeros: [");
        ts.push_str(
            &nums
                .iter()
                .map(|c| format!("'{c}'"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        ts.push_str("],\n");
        ts.push_str("\t\tstrings: [");
        ts.push_str(
            &strs
                .iter()
                .map(|c| format!("'{c}'"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        ts.push_str("],\n");
        ts.push_str("\t},\n");
    }
    ts.push_str("} as const;\n");

    fs::write(&destino, ts).expect("escribir devGuard.generated.ts");
    println!("generado: {}", destino.display());
    for (comando, (dto, nums, strs)) in &comandos {
        println!(
            "  {comando} ({dto}): {} numéricos, {} string",
            nums.len(),
            strs.len()
        );
    }
}
