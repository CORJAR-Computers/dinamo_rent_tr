//! config.rs — Configuración centralizada (puerto de core/config.py)
//!
//! Lee config.ini (mismo formato y secciones que la app Python).
//! Si no existe, usa los mismos defaults de `_DEFAULTS` y lo genera.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::error::AppError;

/// Valores por defecto — espejo de `core/config.py::_Config._DEFAULTS`
const DEFAULTS: &[(&str, &str, &str)] = &[
    // [database]
    ("database", "engine", "firebird"),
    ("database", "host", "localhost"),
    ("database", "port", "3050"),
    ("database", "user", "sysdba"),
    ("database", "password", "masterkey"),
    ("database", "database", "dinamo_rent"),
    ("database", "path", "dinamo_rent_v3.fdb"),
    ("database", "timeout", "10"),
    ("database", "pool_size", "10"),
    ("database", "pool_max_overflow", "20"),
    // [security]
    ("security", "hash_algorithm", "sha256"),
    ("security", "hash_iterations", "100000"),
    ("security", "session_timeout", "3600"),
    ("security", "max_login_attempts", "5"),
    ("security", "account_lockout_duration", "1800"),
    ("security", "login_rate_limit_window", "300"),
    ("security", "max_login_attempts_in_window", "10"),
    ("security", "db_encryption_key", ""),
    // [backup]
    ("backup", "directory", "Backups"),
    ("backup", "max_copies", "10"),
    ("backup", "schedule_times", "09:00, 13:00, 19:00, 23:00"),
    ("backup", "check_interval_ms", "60000"),
    ("backup", "encryption_enabled", "false"),
    ("backup", "encryption_password", ""),
    // [logging]
    ("logging", "directory", "logs"),
    ("logging", "max_size_mb", "5"),
    ("logging", "backup_count", "5"),
    ("logging", "level", "INFO"),
    ("logging", "audit_enabled", "true"),
    ("logging", "audit_retention_days", "30"),
    // [application]
    ("application", "name", "Dinamo Rent ERP"),
    ("application", "version", "3.2.0"),
    ("application", "author", "Dinamo Rent a Car"),
    ("application", "language", "es"),
    ("application", "timezone", "America/Bogota"),
    ("application", "production_mode", "false"),
    ("application", "setup_completed", "false"),
    // [ui]
    ("ui", "color_primario", "#1e40af"),
    ("ui", "color_fondo", "#f8fafc"),
    // [simit] — Agente de consulta automática de comparendos en SIMIT
    ("simit", "enabled", "true"),
    ("simit", "interval_hours", "2"),
    ("simit", "polite_delay_ms", "2500"),
    ("simit", "report_dir", "informes_simit"),
    ("simit", "max_retries", "3"),
    ("simit", "retry_base_delay_ms", "1000"),
    ("simit", "timeout_seconds", "30"),
    ("simit", "circuit_breaker_threshold", "5"),
    ("simit", "circuit_breaker_timeout_seconds", "300"),
    // Minutos a esperar tras el arranque antes de la primera corrida (0 = inmediata).
    // Evita que el PoW/HTTP del agente compita con el inicio de la app.
    ("simit", "start_delay_minutes", "10"),
    // [business]
    ("business", "alert_soat_days", "15"),
    ("business", "alert_tecno_mecanica_days", "15"),
    ("business", "alert_extintor_days", "15"),
    ("business", "km_alert_aceite", "500"),
    ("business", "impuesto_porcentaje", "19"),
    ("business", "roles_con_informes", "Administrador, Supervisor"),
    ("business", "roles_con_usuarios", "Administrador"),
    ("business", "roles_con_eliminar", "Administrador, Supervisor"),
    ("business", "roles_usuarios", "Administrador, Supervisor, Operador"),
    ("business", "tipos_auto", "Automóvil, Camioneta, Van, Lujo, Moto"),
    ("business", "tipos_transmision", "Automática, Mecánica"),
    ("business", "tipos_combustible", "Gasolina, Diesel, Híbrido, Eléctrico, Gas"),
    ("business", "estados_auto", "Disponible, Rentado, Mantenimiento, Vendido, Baja"),
    ("business", "tipos_adquisicion", "Propio, Leasing, Subarrendado"),
    ("business", "tipos_doc", "Cédula, Pasaporte, Cédula Extranjería, NIT, Licencia USA"),
    ("business", "estados_cliente", "Activo, Inactivo, Lista Negra, VIP"),
    ("business", "estados_reserva", "Pendiente, Confirmada, Cancelada, Completada"),
    ("business", "tipos_gasto", "Combustible, Peajes, Lavado, Mantenimiento, Repuestos, Parqueadero, Seguros, Multas, Papelería, Otros"),
    ("business", "nivel_tanque", "Lleno, 3/4, 1/2, 1/4, Reserva"),
    (
        "business",
        "tipos_mantenimiento",
        "Cambio Aceite, Frenos, Llantas, Batería, Tecno-Mecánica, Lavado General, Reparación Mecánica, Otro",
    ),
];

/// Representación en memoria del config.ini: section -> (key -> value)
type IniMap = HashMap<String, HashMap<String, String>>;

/// Mini-parser INI (secciones [x], claves `k = v`, comentarios ; #)
fn parse_ini(content: &str) -> IniMap {
    let mut map: IniMap = HashMap::new();
    let mut section = String::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            map.entry(section.clone()).or_default();
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let value = line[eq + 1..].trim().to_string();
            map.entry(section.clone()).or_default().insert(key, value);
        }
    }
    map
}

/// Serializa la configuración a texto INI
fn serialize_ini(map: &IniMap) -> String {
    let mut out = String::new();
    // Secciones en orden estable de DEFAULTS
    let mut sections: Vec<String> = map.keys().cloned().collect();
    sections.sort();
    for sec in sections {
        out.push_str(&format!("[{}]\n", sec));
        let mut keys: Vec<&String> = map[&sec].keys().collect();
        keys.sort();
        for k in keys {
            out.push_str(&format!("{} = {}\n", k, map[&sec][k]));
        }
        out.push('\n');
    }
    out
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    // ── Database ──
    pub db_user: String,
    pub db_password: String,
    pub db_path: PathBuf,
    pub fbclient_path: PathBuf,
    pub pool_size: usize,
    // ── Security ──
    pub session_timeout: u64,
    pub max_login_attempts: u32,
    pub account_lockout_duration: u64,
    pub login_rate_limit_window: u64,
    pub max_login_attempts_in_window: u32,
    pub db_encryption_key: String,
    // ── Simit (agente de comparendos) ──
    pub simit_enabled: bool,
    pub simit_interval_hours: u64,
    pub simit_polite_delay_ms: u64,
    pub simit_report_dir: PathBuf,
    pub simit_max_retries: u32,
    pub simit_retry_base_delay_ms: u64,
    pub simit_timeout_seconds: u64,
    pub simit_circuit_breaker_threshold: u32,
    pub simit_circuit_breaker_timeout_seconds: u64,
    /// Retraso (minutos) de la primera corrida tras el arranque de la app
    pub simit_start_delay_minutes: u64,
    // ── Business ──
    pub roles_con_informes: HashSet<String>,
    pub roles_con_usuarios: HashSet<String>,
    pub roles_con_eliminar: HashSet<String>,
    pub roles_usuarios: Vec<String>,
    pub tipos_auto: Vec<String>,
    pub tipos_transmision: Vec<String>,
    pub tipos_combustible: Vec<String>,
    pub estados_auto: Vec<String>,
    pub tipos_adquisicion: Vec<String>,
    pub tipos_doc: Vec<String>,
    pub estados_cliente: Vec<String>,
    pub estados_reserva: Vec<String>,
    pub tipos_gasto: Vec<String>,
    pub nivel_tanque: Vec<String>,
    pub tipos_mantenimiento: Vec<String>,
    pub alert_soat_days: i64,
    pub alert_tecno_mecanica_days: i64,
    pub alert_extintor_days: i64,
    pub km_alert_aceite: i64,
    /// Porcentaje de impuesto (IVA) aplicado al subtotal de las rentas
    pub impuesto_porcentaje: f64,
    // ── Application ──
    pub app_name: String,
    pub app_version: String,
    // ── UI (para el frontend) ──
    pub ui_color_primario: String,
    pub ui_color_fondo: String,
    // ── Rutas de runtime ──
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl AppConfig {
    /// Carga la configuración.
    ///
    /// - `data_dir`: directorio de datos de la app (producción: app_data_dir; dev: proyecto/data)
    /// - `resource_dir`: directorio de recursos (producción: bundle; dev: src-tauri/resources)
    /// - `manifest_dir`: directorio src-tauri en dev (CARGO_MANIFEST_DIR)
    pub fn load(data_dir: &Path, resource_dir: &Path, manifest_dir: &Path) -> Self {
        // 1) Construir config.ini en data_dir si no existe (desde defaults)
        let config_path = data_dir.join("config.ini");
        if !config_path.exists() {
            let content = build_default_ini_text();
            if let Err(e) = std::fs::write(&config_path, content) {
                log::warn!("No se pudo crear config.ini en {:?}: {}", config_path, e);
            }
        }

        // 2) Leer config.ini (o defaults si falla la lectura)
        let map = match std::fs::read_to_string(&config_path) {
            Ok(content) => parse_ini(&content),
            Err(e) => {
                log::warn!(
                    "No se pudo leer config.ini ({:?}): {} — usando defaults",
                    config_path,
                    e
                );
                parse_ini(&build_default_ini_text())
            }
        };

        // 3) Resolver fbclient.dll
        let fbclient_path = find_fbclient(resource_dir, manifest_dir);

        // 4) Resolver ruta del .fdb (relativa al data_dir)
        let db_name = get_str(&map, "database", "path", "dinamo_rent_v3.fdb");
        let db_path = if Path::new(&db_name).is_absolute() {
            PathBuf::from(&db_name)
        } else {
            data_dir.join(&db_name)
        };

        Self {
            db_user: get_str(&map, "database", "user", "sysdba"),
            db_password: get_str(&map, "database", "password", "masterkey"),
            db_path,
            fbclient_path,
            pool_size: get_usize(&map, "database", "pool_size", 10),
            session_timeout: get_u64(&map, "security", "session_timeout", 3600),
            max_login_attempts: get_u32(&map, "security", "max_login_attempts", 5),
            account_lockout_duration: get_u64(&map, "security", "account_lockout_duration", 1800),
            login_rate_limit_window: get_u64(&map, "security", "login_rate_limit_window", 300),
            max_login_attempts_in_window: get_u32(
                &map,
                "security",
                "max_login_attempts_in_window",
                10,
            ),
            db_encryption_key: get_str(&map, "security", "db_encryption_key", ""),
            simit_enabled: get_bool(&map, "simit", "enabled", true),
            simit_interval_hours: get_u64(&map, "simit", "interval_hours", 2),
            simit_polite_delay_ms: get_u64(&map, "simit", "polite_delay_ms", 2500),
            simit_report_dir: PathBuf::from(get_str(
                &map,
                "simit",
                "report_dir",
                "informes_simit",
            )),
            simit_max_retries: get_u32(&map, "simit", "max_retries", 3),
            simit_retry_base_delay_ms: get_u64(&map, "simit", "retry_base_delay_ms", 1000),
            simit_timeout_seconds: get_u64(&map, "simit", "timeout_seconds", 30),
            simit_circuit_breaker_threshold: get_u32(&map, "simit", "circuit_breaker_threshold", 5),
            simit_circuit_breaker_timeout_seconds: get_u64(&map, "simit", "circuit_breaker_timeout_seconds", 300),
            simit_start_delay_minutes: get_u64(&map, "simit", "start_delay_minutes", 10),
            roles_con_informes: get_set(&map, "business", "roles_con_informes"),
            roles_con_usuarios: get_set(&map, "business", "roles_con_usuarios"),
            roles_con_eliminar: get_set(&map, "business", "roles_con_eliminar"),
            roles_usuarios: get_list(&map, "business", "roles_usuarios"),
            tipos_auto: get_list(&map, "business", "tipos_auto"),
            tipos_transmision: get_list(&map, "business", "tipos_transmision"),
            tipos_combustible: get_list(&map, "business", "tipos_combustible"),
            estados_auto: get_list(&map, "business", "estados_auto"),
            tipos_adquisicion: get_list(&map, "business", "tipos_adquisicion"),
            tipos_doc: get_list(&map, "business", "tipos_doc"),
            estados_cliente: get_list(&map, "business", "estados_cliente"),
            estados_reserva: get_list(&map, "business", "estados_reserva"),
            tipos_gasto: get_list(&map, "business", "tipos_gasto"),
            nivel_tanque: get_list(&map, "business", "nivel_tanque"),
            tipos_mantenimiento: get_list(&map, "business", "tipos_mantenimiento"),
            alert_soat_days: get_i64(&map, "business", "alert_soat_days", 15),
            alert_tecno_mecanica_days: get_i64(&map, "business", "alert_tecno_mecanica_days", 15),
            alert_extintor_days: get_i64(&map, "business", "alert_extintor_days", 15),
            km_alert_aceite: get_i64(&map, "business", "km_alert_aceite", 500),
            impuesto_porcentaje: get_str(&map, "business", "impuesto_porcentaje", "19")
                .parse::<f64>()
                .unwrap_or(19.0),
            app_name: get_str(&map, "application", "name", "Dinamo Rent ERP"),
            app_version: get_str(&map, "application", "version", "3.2.0"),
            ui_color_primario: get_str(&map, "ui", "color_primario", "#1e40af"),
            ui_color_fondo: get_str(&map, "ui", "color_fondo", "#f8fafc"),
            config_dir: config_path.parent().unwrap_or(data_dir).to_path_buf(),
            data_dir: data_dir.to_path_buf(),
        }
    }

    /// Persiste `db_encryption_key` en la sección [security] de config.ini
    pub fn persist_db_encryption_key(&self, key: &str) -> Result<(), AppError> {
        let path = self.config_dir.join("config.ini");
        let mut map = match std::fs::read_to_string(&path) {
            Ok(content) => parse_ini(&content),
            Err(e) => {
                log::warn!("No se pudo leer config.ini para persistir la clave PII ({e}) — se usan defaults");
                parse_ini(&build_default_ini_text())
            }
        };
        map.entry("security".into())
            .or_default()
            .insert("db_encryption_key".into(), key.trim().to_string());
        let content = serialize_ini(&map);
        // Escritura atómica: temp + rename para no dejar el archivo truncado
        // si la app se cierra a mitad de la escritura.
        let tmp = path.with_extension("ini.tmp");
        std::fs::write(&tmp, &content).map_err(|e| {
            AppError::Generic(format!("No se pudo escribir config.ini: {e}"))
        })?;
        std::fs::rename(&tmp, &path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            AppError::Generic(format!("No se pudo actualizar config.ini: {e}"))
        })
    }

    /// Guarda la configuración actualizada en config.ini
    pub fn save(&self) {
        let map = match std::fs::read_to_string(self.config_dir.join("config.ini")) {
            Ok(content) => parse_ini(&content),
            Err(_) => parse_ini(&build_default_ini_text()),
        };
        let mut map = map;
        map.entry("database".into())
            .or_default()
            .insert("user".into(), self.db_user.clone());
        map.entry("database".into())
            .or_default()
            .insert("password".into(), self.db_password.clone());
        map.entry("database".into())
            .or_default()
            .insert("path".into(), self.db_path.to_string_lossy().to_string());
        let content = serialize_ini(&map);
        if let Err(e) = std::fs::write(self.config_dir.join("config.ini"), content) {
            log::warn!("No se pudo guardar config.ini: {}", e);
        }
    }
}

/// Busca fbclient.dll: primero en resource_dir/firebird, luego en manifest_dir/resources/firebird
fn find_fbclient(resource_dir: &Path, manifest_dir: &Path) -> PathBuf {
    for candidate in [
        resource_dir.join("firebird").join("fbclient.dll"),
        resource_dir.join("fbclient.dll"),
        manifest_dir.join("resources").join("firebird").join("fbclient.dll"),
    ] {
        if candidate.exists() {
            return candidate;
        }
    }
    log::warn!("No se encontró fbclient.dll (se probaron: resources/firebird y manifest resources)");
    // Fallback: devuelve la ruta más probable para dar un error claro al conectar
    manifest_dir.join("resources").join("firebird").join("fbclient.dll")
}

/// Texto INI con todos los defaults (espejo de `_DEFAULTS`)
fn build_default_ini_text() -> String {
    let mut map: IniMap = HashMap::new();
    for (section, key, value) in DEFAULTS {
        map.entry(section.to_string())
            .or_default()
            .insert(key.to_string(), value.to_string());
    }
    serialize_ini(&map)
}

// ─── Helpers de lectura tipada (espejo de _Config.get*()) ───────────────────

fn get_str(map: &IniMap, section: &str, key: &str, fallback: &str) -> String {
    map.get(section)
        .and_then(|s| s.get(key))
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn get_i64(map: &IniMap, section: &str, key: &str, fallback: i64) -> i64 {
    map.get(section)
        .and_then(|s| s.get(key))
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn get_u64(map: &IniMap, section: &str, key: &str, fallback: u64) -> u64 {
    map.get(section)
        .and_then(|s| s.get(key))
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn get_u32(map: &IniMap, section: &str, key: &str, fallback: u32) -> u32 {
    map.get(section)
        .and_then(|s| s.get(key))
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(fallback)
}

fn get_bool(map: &IniMap, section: &str, key: &str, fallback: bool) -> bool {
    map.get(section)
        .and_then(|s| s.get(key))
        .and_then(|v| v.trim().parse::<bool>().ok())
        .unwrap_or(fallback)
}

fn get_usize(map: &IniMap, section: &str, key: &str, fallback: usize) -> usize {
    map.get(section)
        .and_then(|s| s.get(key))
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
}

/// Listas de negocio serializables para el frontend (formularios, filtros)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessLists {
    pub tipos_auto: Vec<String>,
    pub tipos_transmision: Vec<String>,
    pub tipos_combustible: Vec<String>,
    pub estados_auto: Vec<String>,
    pub tipos_adquisicion: Vec<String>,
    pub tipos_doc: Vec<String>,
    pub estados_cliente: Vec<String>,
    pub estados_reserva: Vec<String>,
    pub tipos_gasto: Vec<String>,
    pub nivel_tanque: Vec<String>,
    pub tipos_mantenimiento: Vec<String>,
    pub roles_con_informes: Vec<String>,
    pub roles_con_usuarios: Vec<String>,
    pub roles_con_eliminar: Vec<String>,
    pub roles_usuarios: Vec<String>,
    /// Porcentaje de impuesto (IVA) configurado; el formulario de rentas lo usa
    /// para la vista previa cuando el checkbox «cobrar IVA» está activo.
    pub impuesto_porcentaje: f64,
}

impl AppConfig {
    /// Listas de negocio para exponer al frontend
    pub fn business_lists(&self) -> BusinessLists {
        BusinessLists {
            tipos_auto: self.tipos_auto.clone(),
            tipos_transmision: self.tipos_transmision.clone(),
            tipos_combustible: self.tipos_combustible.clone(),
            estados_auto: self.estados_auto.clone(),
            tipos_adquisicion: self.tipos_adquisicion.clone(),
            tipos_doc: self.tipos_doc.clone(),
            estados_cliente: self.estados_cliente.clone(),
            estados_reserva: self.estados_reserva.clone(),
            tipos_gasto: self.tipos_gasto.clone(),
            nivel_tanque: self.nivel_tanque.clone(),
            tipos_mantenimiento: self.tipos_mantenimiento.clone(),
            roles_con_informes: self.roles_con_informes.iter().cloned().collect(),
            roles_con_usuarios: self.roles_con_usuarios.iter().cloned().collect(),
            roles_con_eliminar: self.roles_con_eliminar.iter().cloned().collect(),
            roles_usuarios: self.roles_usuarios.clone(),
            impuesto_porcentaje: self.impuesto_porcentaje,
        }
    }
}

fn get_set(map: &IniMap, section: &str, key: &str) -> HashSet<String> {
    map.get(section)
        .and_then(|s| s.get(key))
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Listas de negocio (tipos de auto, estados, etc.) para exponer al frontend
pub fn get_list(map: &IniMap, section: &str, key: &str) -> Vec<String> {
    map.get(section)
        .and_then(|s| s.get(key))
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_ini() {
        let ini = parse_ini("[database]\nengine = firebird\npath = test.fdb\n\n[security]\nfoo=bar\n");
        assert_eq!(get_str(&ini, "database", "engine", ""), "firebird");
        assert_eq!(get_str(&ini, "database", "path", ""), "test.fdb");
        assert_eq!(get_str(&ini, "security", "foo", ""), "bar");
        assert_eq!(get_str(&ini, "missing", "key", "fb"), "fb");
    }

    #[test]
    fn comments_and_empty() {
        let ini = parse_ini("# comentario\n; otro\n\n[database]\n# inline no\nengine = firebird\n");
        assert_eq!(get_str(&ini, "database", "engine", ""), "firebird");
    }

    #[test]
    fn defaults_roundtrip() {
        let text = build_default_ini_text();
        let ini = parse_ini(&text);
        assert_eq!(get_str(&ini, "business", "roles_con_informes", ""), "Administrador, Supervisor");
        assert_eq!(get_str(&ini, "business", "roles_con_eliminar", ""), "Administrador, Supervisor");
        assert_eq!(get_u64(&ini, "security", "session_timeout", 0), 3600);
        // Retraso inicial del Agente SIMIT (10 min): no debe competir con el arranque
        assert_eq!(get_u64(&ini, "simit", "start_delay_minutes", 0), 10);
    }
}
