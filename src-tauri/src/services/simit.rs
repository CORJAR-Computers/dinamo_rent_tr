//! services/simit.rs — Agente de consulta automática de comparendos en SIMIT
//!
//! Consulta el portal público del SIMIT (consultasimit.fcm.org.co) por cada
//! placa de la flota, inserta los comparendos/multas nuevos en la tabla
//! `comparendos` (deduplicando por `numero_comparendo`) y genera un reporte
//! HTML imprimible. Corre en un hilo de fondo cada `simit.interval_hours`
//! (config.ini, por defecto 2 horas) mientras la app esté abierta, y también
//! puede dispararse manualmente desde la página de Comparendos.
//!
//! # Contrato del API (ingeniería inversa del portal, ver manavarrp/SimitConsulta)
//! 1. `POST https://qxcaptcha.fcm.org.co/api.php` con form `endpoint=question`
//!    → `{ error: false, data: { question, recommended_difficulty } }`
//!    Headers obligatorios en AMBOS endpoints: `Origin: https://www.fcm.org.co`,
//!    `Referer: https://www.fcm.org.co/` (+ User-Agent de navegador). Sin
//!    Origin/Referer el microservicio responde HTTP 401 "Autenticación fallida".
//! 2. Resolver el Proof-of-Work: buscar `difficulty` nonces primos cuyo
//!    SHA256 de `{"question":q,"time":t,"nonce":n}` empiece con `0000`.
//!    El token es el array JSON de esos objetos de verificación.
//! 3. `POST https://consultasimit.fcm.org.co/simit/microservices/
//!    estado-cuenta-simit/estadocuenta/consulta` con body
//!    `{"filtro":"<PLACA>","reCaptchaDTO":{"response":"<token>","consumidor":"1"}}`
//! 4. Respuesta: `{ multas: [{ comparendo, numeroComparendo, valorPagar,
//!    estadoComparendo, fechaComparendo, organismoTransito,
//!    infracciones: [{codigoInfraccion, descripcionInfraccion}] }],
//!    pazSalvo, cancelada, suspendida }`
//!
//! Nota: los servidores de SIMIT son intermitentes ("Server-unavailable") y
//! pueden cambiar el contrato; el agente reintenta en cada ciclo y registra
//! los errores por placa sin abortar el resto de la sincronización.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate};
use serde::Serialize;
use serde::Deserialize;
use sha2::Digest;
use tauri::Emitter;

use crate::core::config::AppConfig;
use crate::core::error::AppError;
use crate::core::Pool;
use crate::core::PooledConnection;
use crate::repositories::auto::AutoRepository;
use crate::repositories::comparendo::{ComparendoDatos, ComparendoRepository};

/// URL del servidor de captcha Proof-of-Work (qxcaptcha)
const CAPTCHA_URL: &str = "https://qxcaptcha.fcm.org.co/api.php";
/// URL del microservicio de consulta de estado de cuenta por placa
const CONSULTA_URL: &str = "https://consultasimit.fcm.org.co/simit/microservices/estado-cuenta-simit/estadocuenta/consulta";
/// User-Agent de navegador (el portal rechaza clientes genéricos)
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
/// Dificultad por defecto si el servidor no la reporta
const DIFICULTAD_DEFECTO: i64 = 2;
/// Máximo de nonces a probar por iteración del PoW
const MAX_ITERACIONES: i64 = 10_000_000;

// ─── Tipos de datos ───────────────────────────────────────────────────────────

/// Un comparendo/multa tal como lo devuelve el SIMIT, ya mapeado al dominio
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistroSimit {
    /// Número oficial del comparendo (clave de deduplicación)
    pub numero: Option<String>,
    pub placa: String,
    pub fecha_infraccion: String,
    pub hora_infraccion: String,
    /// Monto como string con punto decimal ("580000.00")
    pub monto: String,
    /// Estado del dominio: "Pendiente" | "Pagado" (mapeado del SIMIT)
    pub estado: String,
    pub organismo: String,
    pub codigo_infraccion: String,
    pub descripcion: String,
    /// true = comparendo de tránsito, false = otro tipo de multa
    pub es_comparendo: bool,
    /// true = se insertó en esta sincronización; false = ya estaba en la BD
    pub nuevo: bool,
}

/// Error de una placa individual (no aborta la sincronización)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPlacaSimit {
    pub placa: String,
    pub error: String,
}

/// Resumen serializable de una sincronización (evento + comando de estado)
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoSincronizacion {
    /// Marca de tiempo de inicio (RFC3339 local)
    pub sincronizado_en: String,
    pub placas_consultadas: usize,
    pub placas_con_error: usize,
    /// Registros encontrados por el SIMIT (antes de deduplicar)
    pub encontrados: usize,
    /// Registros nuevos insertados en la tabla comparendos
    pub insertados: usize,
    /// Registros ya existentes (deduplicados por número o placa+fecha+monto)
    pub duplicados: usize,
    /// Suma de los comparendos pendientes encontrados en esta sincronización
    pub total_pendiente: String,
    /// Registros encontrados (para el reporte y la UI)
    pub registros: Vec<RegistroSimit>,
    /// Errores por placa (SIMIT caído, captcha rechazado, etc.)
    pub errores: Vec<ErrorPlacaSimit>,
    /// Ruta absoluta del reporte HTML generado (si se pudo escribir)
    pub reporte_html: Option<String>,
}

// ─── Cliente SIMIT (captcha PoW + consulta) ───────────────────────────────────

/// Cliente HTTP compartido con timeouts razonables (se reutiliza entre placas)
static AGENTE: once_cell::sync::Lazy<ureq::Agent> = once_cell::sync::Lazy::new(|| {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(25))
        .build()
});

fn agente() -> &'static ureq::Agent {
    &AGENTE
}

/// Aplica los headers de navegador que exige el portal SIMIT.
///
/// Sin `Origin` y `Referer` el microservicio responde HTTP 401
/// ("Autenticación fallida: Acceso denegado...") — verificado contra el
/// portal real el 10-08 y documentado en la referencia manavarrp/SimitConsulta
/// (ServiceExtensions: "Sin Origin y Referer el servidor rechaza la petición").
/// NO se envían headers `Sec-Fetch-*` ni `sec-ch-ua*`: la referencia advierte
/// que esos hacen que el servidor bloquee la conexión (ureq no los añade).
fn con_headers_browser(req: ureq::Request) -> ureq::Request {
    req.set("User-Agent", USER_AGENT)
        .set("Origin", "https://www.fcm.org.co")
        .set("Referer", "https://www.fcm.org.co/")
        .set("Accept", "*/*")
        .set("Accept-Language", "es-ES,es;q=0.9")
}

/// Resuelve el captcha Proof-of-Work y devuelve el token (array JSON de
/// objetos de verificación) listo para enviar al microservicio de consulta.
pub fn resolver_captcha() -> Result<String, AppError> {
    let respuesta = con_headers_browser(agente().post(CAPTCHA_URL))
        .send_form(&[("endpoint", "question")])
        .map_err(|e| {
            AppError::Generic(format!("No se pudo contactar el captcha SIMIT (qxcaptcha): {e}"))
        })?;
    let body: CaptchaRespuesta = respuesta.into_json().map_err(|e| {
        AppError::Generic(format!("Respuesta inválida del captcha SIMIT: {e}"))
    })?;
    if body.tiene_error() || body.data.is_none() {
        return Err(AppError::Generic(
            "El servidor de captcha SIMIT rechazó la consulta. Intenta de nuevo.".into(),
        ));
    }
    let data = body.data.expect("verificado arriba");
    let dificultad = if data.recommended_difficulty > 0 {
        data.recommended_difficulty
    } else {
        DIFICULTAD_DEFECTO
    };
    let tiempo = Local::now().timestamp();
    resolver_pow(&data.question, tiempo, dificultad)
}

/// Consulta los comparendos/multas de una placa en el SIMIT.
pub fn consultar_placa(placa: &str) -> Result<Vec<RegistroSimit>, AppError> {
    let token = resolver_captcha()?;
    let body = serde_json::json!({
        "filtro": placa.trim(),
        "reCaptchaDTO": { "response": token, "consumidor": "1" }
    });
    let respuesta = con_headers_browser(agente().post(CONSULTA_URL))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| {
            AppError::Generic(format!("SIMIT no respondió para la placa {placa}: {e}"))
        })?;
    let dto: RespuestaConsulta = respuesta.into_json().map_err(|e| {
        AppError::Generic(format!("Respuesta SIMIT inválida para la placa {placa}: {e}"))
    })?;
    Ok(mapear_registros(&dto, placa.trim()))
}

// ─── Sincronización con la base de datos ──────────────────────────────────────

/// Ejecuta una sincronización completa: consulta todas las placas de la flota,
/// inserta los comparendos nuevos y genera el reporte HTML.
pub fn sincronizar(
    conn: &mut PooledConnection,
    cfg: &Arc<AppConfig>,
) -> Result<ResultadoSincronizacion, AppError> {
    let placas = AutoRepository::placas_activas(conn)?;
    let mut resultado = ResultadoSincronizacion {
        sincronizado_en: Local::now().to_rfc3339(),
        ..Default::default()
    };

    for placa in &placas {
        match consultar_placa(placa) {
            Ok(registros) => {
                resultado.placas_consultadas += 1;
                for mut reg in registros {
                    resultado.encontrados += 1;
                    // ¿Ya existe? (número oficial o placa+fecha+monto). Si el
                    // SIMIT reporta pagado un comparendo ya registrado, se
                    // sincroniza el estado (la BD converge con el SIMIT).
                    if ya_existe(conn, &reg)? {
                        if reg.estado == "Pagado" {
                            if let Some(num) = reg.numero.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
                                ComparendoRepository::marcar_pagado_por_numero(conn, num)?;
                            }
                        }
                        resultado.duplicados += 1;
                        resultado.registros.push(reg);
                        continue;
                    }
                    // Fecha inválida → se omite el registro (no aborta la placa)
                    if NaiveDate::parse_from_str(&reg.fecha_infraccion, "%Y-%m-%d").is_err() {
                        log::warn!(
                            "Agente SIMIT: fecha inválida para {} ({}), registro omitido",
                            reg.placa,
                            reg.fecha_infraccion
                        );
                        resultado.duplicados += 1;
                        continue;
                    }
                    let datos = ComparendoDatos {
                        placa: reg.placa.clone(),
                        fecha_infraccion: reg.fecha_infraccion.clone(),
                        hora_infraccion: reg.hora_infraccion.clone(),
                        monto: reg.monto.clone(),
                        numero_comparendo: reg.numero.clone(),
                        id_renta: None,
                        id_cliente: None,
                        estado: reg.estado.clone(),
                        observaciones: Some(observaciones_para(&reg)),
                    };
                    ComparendoRepository::insertar(conn, &datos)?;
                    resultado.insertados += 1;
                    reg.nuevo = true;
                    resultado.registros.push(reg);
                }
            }
            Err(e) => {
                resultado.placas_con_error += 1;
                resultado.errores.push(ErrorPlacaSimit {
                    placa: placa.clone(),
                    error: e.mensaje_usuario(),
                });
                log::warn!("Agente SIMIT: error consultando {placa}: {e}");
            }
        }
        // Espera corta entre placas para no saturar el portal
        if cfg.simit_polite_delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(cfg.simit_polite_delay_ms));
        }
    }

    // Total pendiente = suma de TODOS los registros encontrados (nuevos y ya
    // registrados), para que el reporte refleje la deuda real de la flota.
    resultado.total_pendiente = format!(
        "{:.2}",
        resultado
            .registros
            .iter()
            .filter(|r| r.estado == "Pendiente")
            .map(|r| r.monto.parse::<f64>().unwrap_or(0.0))
            .sum::<f64>()
    );

    match generar_reporte_html(cfg, &resultado) {
        Ok(path) => resultado.reporte_html = Some(path.to_string_lossy().to_string()),
        Err(e) => log::warn!("Agente SIMIT: no se pudo escribir el reporte HTML: {e}"),
    }

    Ok(resultado)
}

/// ¿El registro ya existe en la BD? Deduplica por número oficial y, como
/// respaldo, por placa + fecha + monto.
fn ya_existe(conn: &mut PooledConnection, reg: &RegistroSimit) -> Result<bool, AppError> {
    if let Some(num) = reg.numero.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        if ComparendoRepository::existe_por_numero(conn, num)? {
            return Ok(true);
        }
    }
    if !reg.fecha_infraccion.is_empty()
        && ComparendoRepository::existe_duplicado(conn, &reg.placa, &reg.fecha_infraccion, &reg.monto)?
    {
        return Ok(true);
    }
    Ok(false)
}

/// Observaciones legibles para el registro insertado (trazabilidad SIMIT)
fn observaciones_para(reg: &RegistroSimit) -> String {
    let tipo = if reg.es_comparendo {
        "Comparendo"
    } else {
        "Multa"
    };
    let mut partes = vec![format!("Importado SIMIT ({tipo})")];
    if let Some(n) = reg.numero.as_deref().filter(|n| !n.trim().is_empty()) {
        partes.push(format!("N° {n}"));
    }
    if !reg.organismo.is_empty() {
        partes.push(reg.organismo.clone());
    }
    if !reg.codigo_infraccion.is_empty() {
        partes.push(format!("Código {}", reg.codigo_infraccion));
    }
    if !reg.descripcion.is_empty() {
        partes.push(reg.descripcion.clone());
    }
    partes.join(" · ")
}

// ─── Reporte HTML ─────────────────────────────────────────────────────────────

/// Escribe el reporte HTML imprimible en `data_dir/informes_simit/simit_*.html`
fn generar_reporte_html(
    cfg: &Arc<AppConfig>,
    r: &ResultadoSincronizacion,
) -> Result<PathBuf, AppError> {
    let dir = cfg.data_dir.join(&cfg.simit_report_dir);
    std::fs::create_dir_all(&dir)?;
    let stamp = Local::now().format("%Y%m%d_%H%M");
    let path = dir.join(format!("simit_{stamp}.html"));

    let mut filas = String::new();
    for reg in &r.registros {
        let numero = reg.numero.as_deref().unwrap_or("-");
        let tipo = if reg.es_comparendo {
            "Comparendo"
        } else {
            "Multa"
        };
        let estado = if reg.estado == "Pagado" {
            "Pagado"
        } else {
            "Pendiente"
        };
        let marca_nuevo = if reg.nuevo { " 🆕" } else { "" };
        filas.push_str(&format!(
            "<tr><td>{}{}</td><td class=\"num\">{}</td><td>{}</td><td>{}</td><td>{}</td>\
             <td>{}</td><td>{}</td><td class=\"valor\">${}</td><td>{}</td></tr>",
            esc_html(&reg.placa),
            marca_nuevo,
            esc_html(numero),
            esc_html(&reg.fecha_infraccion),
            esc_html(&reg.hora_infraccion),
            esc_html(tipo),
            esc_html(&reg.descripcion),
            esc_html(&reg.organismo),
            esc_html(&reg.monto),
            esc_html(estado),
        ));
    }

    let mut errores_html = String::new();
    for e in &r.errores {
        errores_html.push_str(&format!(
            "<li><strong>{}</strong>: {}</li>",
            esc_html(&e.placa),
            esc_html(&e.error)
        ));
    }

    let (total_estado, total_color) = if r.errores.is_empty() {
        ("Completado", "#1b5e20")
    } else if r.registros.is_empty() {
        ("Con errores", "#b71c1c")
    } else {
        ("Completado con errores parciales", "#e65100")
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
<meta charset="utf-8">
<title>Reporte SIMIT — Comparendos de la flota</title>
<style>
  body {{ font-family: 'Segoe UI', Arial, sans-serif; margin: 24px; color: #1f2937; }}
  h1 {{ font-size: 20px; margin: 0 0 4px; color: #111827; }}
  .sub {{ color: #6b7280; font-size: 13px; margin-bottom: 18px; }}
  .tarjetas {{ display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 18px; }}
  .tarjeta {{ border: 1px solid #e5e7eb; border-radius: 10px; padding: 10px 16px; min-width: 110px; }}
  .tarjeta .n {{ font-size: 22px; font-weight: 800; }}
  .tarjeta .t {{ font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: #6b7280; }}
  table {{ border-collapse: collapse; width: 100%; font-size: 13px; }}
  th, td {{ border: 1px solid #e5e7eb; padding: 6px 8px; text-align: left; }}
  th {{ background: #eef2ff; font-weight: 700; }}
  .num {{ font-variant-numeric: tabular-nums; }}
  .valor {{ text-align: right; font-variant-numeric: tabular-nums; }}
  .vacio {{ color: #6b7280; font-style: italic; padding: 14px 0; }}
  .errores {{ margin-top: 18px; font-size: 12px; color: #7f1d1d; background: #fef2f2; border: 1px solid #fecaca; border-radius: 8px; padding: 10px 14px; }}
  .pie {{ margin-top: 20px; font-size: 11px; color: #9ca3af; }}
</style>
</head>
<body>
  <h1>Reporte de Comparendos — SIMIT</h1>
  <div class="sub">{sincronizado} · Estado: <strong style="color:{total_color}">{total_estado}</strong></div>
  <div class="tarjetas">
    <div class="tarjeta"><div class="n">{consultadas}</div><div class="t">Placas consultadas</div></div>
    <div class="tarjeta"><div class="n">{encontrados}</div><div class="t">Encontrados</div></div>
    <div class="tarjeta"><div class="n">{insertados}</div><div class="t">Nuevos en la BD</div></div>
    <div class="tarjeta"><div class="n">{duplicados}</div><div class="t">Ya registrados</div></div>
    <div class="tarjeta"><div class="n">${total_pendiente}</div><div class="t">Total pendiente</div></div>
  </div>
  <table>
    <thead><tr><th>Placa</th><th>N° Comparendo</th><th>Fecha</th><th>Hora</th><th>Tipo</th><th>Infracción</th><th>Organismo</th><th>Valor</th><th>Estado</th></tr></thead>
    <tbody>{filas}</tbody>
  </table>
  <p class="pie">🆕 = registrado en la BD en esta sincronización.</p>
  {vacio}
  {errores}
  <div class="pie">Generado automáticamente por el Agente SIMIT de Dinamo Rent · {sincronizado}</div>
</body>
</html>"#,
        sincronizado = esc_html(&r.sincronizado_en),
        total_estado = total_estado,
        total_color = total_color,
        consultadas = r.placas_consultadas,
        encontrados = r.encontrados,
        insertados = r.insertados,
        duplicados = r.duplicados,
        total_pendiente = esc_html(&r.total_pendiente),
        filas = if filas.is_empty() {
            String::new()
        } else {
            filas
        },
        vacio = if r.registros.is_empty() {
            r#"<p class="vacio">No se encontraron comparendos ni multas para la flota en esta consulta.</p>"#
                .to_string()
        } else {
            String::new()
        },
        errores = if r.errores.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="errores"><strong>Placas con error ({})</strong><ul>{}</ul></div>"#,
                r.errores.len(),
                errores_html
            )
        },
    );

    std::fs::write(&path, html)?;
    Ok(path)
}

/// Escapa HTML (los datos vienen del SIMIT / placas de la BD)
fn esc_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ─── Estado en memoria y scheduler ────────────────────────────────────────────

/// Wrapper manejado por Tauri (app.manage) para que los comandos accedan al
/// estado del agente sin ampliar AppState (evita tocar los tests de integración).
pub struct EstadoAgenteSimitManaged(pub Arc<EstadoAgenteSimit>);

/// Estado en memoria del agente (visible para el frontend)
#[derive(Default)]
pub struct EstadoAgenteSimit {
    interno: Mutex<EstadoAgenteSimitInner>,
    /// Evita sincronizaciones concurrentes (manual + programada)
    pub ejecutando: AtomicBool,
}

#[derive(Debug, Clone, Default)]
struct EstadoAgenteSimitInner {
    ultima_sincronizacion: Option<String>,
    ultimo_resultado: Option<ResultadoSincronizacion>,
    ultimo_error: Option<String>,
}

/// Info serializable del agente para la UI
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoAgenteSimit {
    pub habilitado: bool,
    pub interval_hours: u64,
    pub ejecutando: bool,
    pub ultima_sincronizacion: Option<String>,
    pub ultimo_resultado: Option<ResultadoSincronizacion>,
    pub ultimo_error: Option<String>,
}

impl EstadoAgenteSimit {
    pub fn esta_ejecutando(&self) -> bool {
        self.ejecutando.load(Ordering::SeqCst)
    }

    /// Intenta tomar la ejecución de forma atómica (CAS). Solo un hilo gana:
    /// evita que la sincronización programada y la manual corran a la vez.
    pub fn claimar(&self) -> bool {
        self.ejecutando
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Libera la ejecución (siempre tras `claimar`).
    pub fn liberar(&self) {
        self.ejecutando.store(false, Ordering::SeqCst);
    }

    pub fn info(&self, cfg: &AppConfig) -> InfoAgenteSimit {
        let interno = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        InfoAgenteSimit {
            habilitado: cfg.simit_enabled,
            interval_hours: cfg.simit_interval_hours,
            ejecutando: self.esta_ejecutando(),
            ultima_sincronizacion: interno.ultima_sincronizacion.clone(),
            ultimo_resultado: interno.ultimo_resultado.clone(),
            ultimo_error: interno.ultimo_error.clone(),
        }
    }

    fn registrar_ok(&self, resultado: ResultadoSincronizacion) {
        let mut interno = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        interno.ultima_sincronizacion = Some(resultado.sincronizado_en.clone());
        interno.ultimo_resultado = Some(resultado);
        interno.ultimo_error = None;
    }

    fn registrar_error(&self, error: &str) {
        let mut interno = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        interno.ultimo_error = Some(error.to_string());
    }
}

/// Ejecuta una sincronización y actualiza el estado del agente.
/// Es la única entrada compartida por el scheduler y el comando manual.
pub fn run_sync(
    pool: &Pool,
    cfg: &Arc<AppConfig>,
    estado: &EstadoAgenteSimit,
) -> Result<ResultadoSincronizacion, AppError> {
    let mut conn = pool.get()?;
    let resultado = sincronizar(&mut conn, cfg)?;
    estado.registrar_ok(resultado.clone());
    Ok(resultado)
}

/// Lanza el hilo de fondo del agente. Consulta al arrancar la app y después
/// cada `interval_hours`.
///
/// Reintento: si la sincronización falla a nivel de BD (no se pudo ni listar
/// las placas), NO se marca la última ejecución y se reintenta en el siguiente
/// tick (60 s). Si el SIMIT está caído, las placas fallan individualmente y la
/// corrida termina con `errores` (sin error fatal): la siguiente consulta
/// ocurre en el ciclo normal (2 h), evitando saturar el portal.
pub fn spawn_scheduler(
    app: tauri::AppHandle,
    pool: Pool,
    cfg: Arc<AppConfig>,
    estado: Arc<EstadoAgenteSimit>,
) {
    std::thread::spawn(move || {
        let mut ultima_ejecucion: Option<Instant> = None;
        loop {
            if cfg.simit_enabled {
                let debe_ejecutar = match ultima_ejecucion {
                    None => true,
                    Some(t) => {
                        t.elapsed() >= Duration::from_secs(cfg.simit_interval_hours.saturating_mul(3600))
                    }
                };
                if debe_ejecutar && estado.claimar() {
                    match run_sync(&pool, &cfg, &estado) {
                        Ok(resultado) => {
                            ultima_ejecucion = Some(Instant::now());
                            log::info!(
                                "Agente SIMIT: sincronización OK — {} placas, {} nuevos",
                                resultado.placas_consultadas,
                                resultado.insertados
                            );
                            let _ = app.emit("simit-sync-complete", &resultado);
                        }
                        Err(e) => {
                            estado.registrar_error(&e.to_string());
                            log::error!("Agente SIMIT: falló la sincronización: {e}");
                            // No se marca ultima_ejecucion → reintento en el siguiente tick
                        }
                    }
                    estado.liberar();
                }
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

// ─── DTOs del SIMIT ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CaptchaRespuesta {
    /// El servidor devuelve bool (false/true) o int (0/1)
    error: serde_json::Value,
    data: Option<CaptchaDatos>,
}

impl CaptchaRespuesta {
    fn tiene_error(&self) -> bool {
        match &self.error {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) != 0,
            _ => true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CaptchaDatos {
    question: String,
    #[serde(rename = "recommended_difficulty")]
    recommended_difficulty: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RespuestaConsulta {
    #[serde(default)]
    multas: Vec<MultaDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultaDto {
    comparendo: Option<bool>,
    numero_comparendo: Option<String>,
    valor_pagar: Option<f64>,
    estado_comparendo: Option<String>,
    fecha_comparendo: Option<String>,
    organismo_transito: Option<String>,
    #[serde(default)]
    infracciones: Vec<InfraccionDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InfraccionDto {
    codigo_infraccion: Option<String>,
    descripcion_infraccion: Option<String>,
}

/// Traduce la respuesta del SIMIT a registros del dominio. El SIMIT mezcla
/// comparendos y multas en `multas` (campo `comparendo` true/false); se
/// conservan ambos para la flota.
fn mapear_registros(dto: &RespuestaConsulta, placa: &str) -> Vec<RegistroSimit> {
    dto.multas
        .iter()
        .filter_map(|m| {
            let (fecha, hora) = parsear_fecha_hora(m.fecha_comparendo.as_deref().unwrap_or(""));
            let monto = m
                .valor_pagar
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "0.00".into());
            let numero = m
                .numero_comparendo
                .clone()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            // Registro sin datos útiles → se omite
            if numero.is_none() && fecha.is_empty() && monto == "0.00" {
                return None;
            }
            let inf = m.infracciones.first();
            Some(RegistroSimit {
                numero,
                placa: placa.to_string(),
                fecha_infraccion: fecha,
                hora_infraccion: hora,
                monto,
                estado: mapear_estado(m.estado_comparendo.as_deref()),
                organismo: m.organismo_transito.clone().unwrap_or_default(),
                codigo_infraccion: inf
                    .and_then(|i| i.codigo_infraccion.clone())
                    .unwrap_or_default(),
                descripcion: inf
                    .and_then(|i| i.descripcion_infraccion.clone())
                    .unwrap_or_default(),
                es_comparendo: m.comparendo.unwrap_or(false),
                nuevo: false,
            })
        })
        .collect()
}

/// Estados del dominio: PAGADO/COBRADO → "Pagado"; lo demás → "Pendiente"
fn mapear_estado(estado: Option<&str>) -> String {
    match estado {
        Some(e) => {
            let u = e.to_uppercase();
            // Cubre PAGADO / COBRADO / EN COBRO / COBRO COACTIVO…
            if u.contains("PAGA") || u.contains("COBR") {
                "Pagado".into()
            } else {
                "Pendiente".into()
            }
        }
        None => "Pendiente".into(),
    }
}

/// Extrae fecha (AAAA-MM-DD) y hora (HH:MM) de formatos como
/// "2026-01-15", "2026-01-15 14:30:00" o ISO con 'T'.
fn parsear_fecha_hora(raw: &str) -> (String, String) {
    let raw = raw.trim();
    if raw.is_empty() {
        return ("".into(), "00:00".into());
    }
    // Separa fecha y hora por 'T' (ISO) o por espacio; la hora puede traer
    // un 'T' o espacios sobrantes que se recortan antes de tomar HH:MM.
    let mut partes = raw.splitn(2, ['T', ' ']);
    let fecha = partes.next().unwrap_or("").trim().to_string();
    let hora_raw = partes.next().unwrap_or("").trim();
    let hora: String = hora_raw
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take(5)
        .collect();
    let hora = if hora.is_empty() { "00:00".into() } else { hora };
    (fecha, hora)
}

// ─── Proof-of-Work (captcha) ──────────────────────────────────────────────────

fn sha256_hex(input: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn es_primo(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    let mut i = 3;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i += 2;
    }
    true
}

/// JSON exacto que espera el SIMIT: orden question, time, nonce
fn construir_json_verificacion(question: &str, time: i64, nonce: i64) -> String {
    format!(r#"{{"question":"{question}","time":{time},"nonce":{nonce}}}"#)
}

/// Resuelve el PoW `difficulty` veces (nonces primos crecientes cuyo hash
/// SHA256 empiece con "0000") y devuelve el token (array JSON).
fn resolver_pow(question: &str, time: i64, difficulty: i64) -> Result<String, AppError> {
    let mut partes = Vec::with_capacity(difficulty as usize);
    let mut ultimo_nonce = 1i64;
    for _ in 0..difficulty {
        ultimo_nonce = resolver_iteracion(question, time, ultimo_nonce)?;
        partes.push(construir_json_verificacion(question, time, ultimo_nonce));
    }
    Ok(format!("[{}]", partes.join(",")))
}

fn resolver_iteracion(question: &str, time: i64, inicio: i64) -> Result<i64, AppError> {
    for nonce in (inicio + 1)..MAX_ITERACIONES {
        if !es_primo(nonce) {
            continue;
        }
        let json = construir_json_verificacion(question, time, nonce);
        if sha256_hex(&json).starts_with("0000") {
            return Ok(nonce);
        }
    }
    // Prácticamente inalcanzable (dificultad 4 hex); se reporta como error en
    // lugar de devolver un token inválido en silencio.
    Err(AppError::Generic(
        "No se pudo resolver el captcha Proof-of-Work del SIMIT.".into(),
    ))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_abc_conocido() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn primos_y_compuestos() {
        assert!(es_primo(2));
        assert!(es_primo(3));
        assert!(es_primo(97));
        assert!(!es_primo(1));
        assert!(!es_primo(4));
        assert!(!es_primo(100));
    }

    #[test]
    fn json_verificacion_formato_exacto() {
        assert_eq!(
            construir_json_verificacion("abc", 1700000000, 3),
            r#"{"question":"abc","time":1700000000,"nonce":3}"#
        );
    }

    #[test]
    fn pow_produce_nonces_validos() {
        let time = Local::now().timestamp();
        let token = resolver_pow("question-de-prueba", time, 2).expect("pow resoluble");
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&token).expect("array JSON");
        assert_eq!(parsed.len(), 2, "un objeto de verificación por nivel");
        let mut prev_nonce = 0i64;
        for obj in &parsed {
            let question = obj["question"].as_str().unwrap();
            let nonce = obj["nonce"].as_i64().unwrap();
            assert!(nonce > prev_nonce, "nonces estrictamente crecientes");
            prev_nonce = nonce;
            assert!(es_primo(nonce), "nonce debe ser primo");
            let json = construir_json_verificacion(question, time, nonce);
            assert!(
                sha256_hex(&json).starts_with("0000"),
                "el hash debe empezar con 0000"
            );
        }
    }

    #[test]
    fn mapeo_estado_simit() {
        assert_eq!(mapear_estado(None), "Pendiente");
        assert_eq!(mapear_estado(Some("PENDIENTE")), "Pendiente");
        assert_eq!(mapear_estado(Some("VIGENTE")), "Pendiente");
        assert_eq!(mapear_estado(Some("PAGADO")), "Pagado");
        assert_eq!(mapear_estado(Some("EN COBRO COACTIVO")), "Pagado");
    }

    #[test]
    fn parseo_fechas_horas() {
        assert_eq!(parsear_fecha_hora(""), ("".into(), "00:00".into()));
        assert_eq!(
            parsear_fecha_hora("2026-01-15"),
            ("2026-01-15".into(), "00:00".into())
        );
        assert_eq!(
            parsear_fecha_hora("2026-01-15 14:30:00"),
            ("2026-01-15".into(), "14:30".into())
        );
        assert_eq!(
            parsear_fecha_hora("2026-01-15T08:05:00"),
            ("2026-01-15".into(), "08:05".into())
        );
    }

    #[test]
    fn mapeo_respuesta_completa() {
        let json = r#"{
            "multas": [
                {
                    "comparendo": true,
                    "numeroComparendo": "250010000000123",
                    "valorPagar": 580000.5,
                    "estadoComparendo": "PENDIENTE",
                    "fechaComparendo": "2026-03-12 09:15:00",
                    "organismoTransito": "Medellín",
                    "infracciones": [
                        { "codigoInfraccion": "C24", "descripcionInfraccion": "Exceso de velocidad" }
                    ]
                },
                {
                    "comparendo": false,
                    "numeroComparendo": null,
                    "valorPagar": 0,
                    "estadoComparendo": null,
                    "fechaComparendo": null,
                    "organismoTransito": null,
                    "infracciones": []
                }
            ],
            "pazSalvo": false,
            "cancelada": false,
            "suspendida": false
        }"#;
        let dto: RespuestaConsulta = serde_json::from_str(json).expect("DTO válido");
        let registros = mapear_registros(&dto, "ABC123");
        assert_eq!(registros.len(), 1, "el registro vacío se omite");
        let r = &registros[0];
        assert!(r.es_comparendo);
        assert_eq!(r.numero.as_deref(), Some("250010000000123"));
        assert_eq!(r.monto, "580000.50");
        assert_eq!(r.estado, "Pendiente");
        assert_eq!(r.fecha_infraccion, "2026-03-12");
        assert_eq!(r.hora_infraccion, "09:15");
        assert_eq!(r.organismo, "Medellín");
        assert_eq!(r.codigo_infraccion, "C24");
        assert_eq!(r.descripcion, "Exceso de velocidad");
        assert_eq!(r.placa, "ABC123");
    }

    #[test]
    fn observaciones_trazabilidad() {
        let reg = RegistroSimit {
            numero: Some("N1".into()),
            placa: "ABC123".into(),
            fecha_infraccion: "2026-01-01".into(),
            hora_infraccion: "10:00".into(),
            monto: "100.00".into(),
            estado: "Pendiente".into(),
            organismo: "Bogotá".into(),
            codigo_infraccion: "C24".into(),
            descripcion: "Exceso de velocidad".into(),
            es_comparendo: true,
            nuevo: false,
        };
        let obs = observaciones_para(&reg);
        assert!(obs.contains("Comparendo"));
        assert!(obs.contains("N° N1"));
        assert!(obs.contains("Bogotá"));
        assert!(obs.contains("C24"));
        assert!(obs.contains("Exceso de velocidad"));
    }

    #[test]
    fn escapa_html() {
        assert_eq!(esc_html("<script>&\"x\"</script>"), "&lt;script&gt;&amp;&quot;x&quot;&lt;/script&gt;");
    }
}
