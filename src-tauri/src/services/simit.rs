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
//! # Fase 1 — sesión HTTP como navegador (cookie jar compartido)
//! El gateway de seguridad externo (ADC) exige cookies de sesión
//! (`ADC_CONN_*`/`ADC_REQ_*`): sin ellas responde 401 "Autenticación fallida:
//! Acceso denegado. No se puede definir la política de seguridad." Para
//! replicar la sesión de un navegador: (1) el agente ureq mantiene un **jar de
//! cookies persistente** compartido entre siembra, captcha y consulta
//! (feature `cookies`); (2) se hace un `GET https://www.fcm.org.co/` previo
//! (como navegar a la SPA) para que el gateway emita sus cookies; (3) el token
//! PoW se envía con **una sola solución** (`solo_primera_solucion`), igual que
//! el servicio Python de referencia (API-Runt-simit/app/procesos/simit/service.py)
//! que se verificó exitoso (`debug_response.json`); (4) ante un 401 se re-siembra
//! la sesión y se reintenta una vez con token fresco.
//!
//! Nota: los servidores de SIMIT son intermitentes ("Server-unavailable") y
//! pueden cambiar el contrato; el agente reintenta en cada ciclo y registra
//! los errores por placa sin abortar el resto de la sincronización.

use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate};
use cookie_store::CookieStore;
use rsfbclient::{Execute, Queryable};
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

// ─── Circuit Breaker ──────────────────────────────────────────────────────────

/// Estado del circuit breaker para proteger contra fallos prolongados del SIMIT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal: permite requests
    Closed,
    /// Abierto: bloquea requests (demasiados fallos recientes)
    Open,
    /// Semi-abierto: permite un request de prueba
    HalfOpen,
}

/// Circuit breaker para el portal SIMIT
pub struct CircuitBreaker {
    state: Mutex<CircuitState>,
    failure_count: AtomicU32,
    last_failure_time: Mutex<Option<Instant>>,
    threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout_secs: u64) -> Self {
        Self {
            state: Mutex::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            last_failure_time: Mutex::new(None),
            threshold,
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Verifica si se permite hacer un request
    pub fn allow_request(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Verificar si ya pasó el timeout para cambiar a HalfOpen
                let last_failure = self.last_failure_time.lock().unwrap();
                if let Some(last) = *last_failure {
                    if last.elapsed() >= self.timeout {
                        *state = CircuitState::HalfOpen;
                        log::info!("Circuit Breaker SIMIT: cambiando a Half-Open");
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Registra un exito
    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        self.failure_count.store(0, Ordering::SeqCst);
        if *state == CircuitState::HalfOpen {
            *state = CircuitState::Closed;
            log::info!("Circuit Breaker SIMIT: cerrado (recuperado)");
        }
    }

    /// Registra un fallo
    pub fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure_time.lock().unwrap() = Some(Instant::now());
        
        if count >= self.threshold {
            let mut state = self.state.lock().unwrap();
            if *state != CircuitState::Open {
                *state = CircuitState::Open;
                log::warn!(
                    "Circuit Breaker SIMIT: ABIERTO tras {} fallos consecutivos",
                    count
                );
            }
        }
    }

    /// Obtiene el estado actual
    pub fn current_state(&self) -> CircuitState {
        *self.state.lock().unwrap()
    }
}

/// Instancia global del circuit breaker
static CIRCUIT_BREAKER: once_cell::sync::Lazy<CircuitBreaker> = once_cell::sync::Lazy::new(|| {
    CircuitBreaker::new(5, 300) // Default: 5 fallos, 5 min timeout
});

/// Inicializa el circuit breaker con la configuración
pub fn init_circuit_breaker(threshold: u32, timeout_secs: u64) {
    // Como es Lazy, se inicializa en el primer acceso
    // Pero podemos forzar la inicialización aquí
    let _ = &*CIRCUIT_BREAKER;
    log::info!(
        "Circuit Breaker SIMIT: umbral={}, timeout={}s",
        threshold,
        timeout_secs
    );
}

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
/// Timeout por defecto para requests HTTP (segundos)
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

// ─── Tipos de datos ───────────────────────────────────────────────────────────

/// Un comparendo/multa tal como lo devuelve el SIMIT, ya mapeado al dominio
/// (Serialize/Deserialize: se persiste en la BD como JSON y se restaura al
/// arrancar para que el filtro «Solo nuevos» sobreviva al reinicio).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// id en la tabla `comparendos` (Some si existe/insertó en esta corrida);
    /// permite al frontend marcar en la tabla cuáles son nuevos vs existentes
    pub id: Option<i64>,
}

/// Error de una placa individual (no aborta la sincronización)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPlacaSimit {
    pub placa: String,
    pub error: String,
}

/// Resumen serializable de una sincronización (evento + comando de estado).
/// Serialize/Deserialize: se persiste en la BD tras cada corrida (tabla
/// `agente_simit_ultimo_resultado`) y se restaura al arrancar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Métricas de rendimiento
    pub metricas: MetricasSimit,
}

/// Métricas de rendimiento de la sincronización
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricasSimit {
    /// Tiempo total de la sincronización (ms)
    pub tiempo_total_ms: u64,
    /// Tiempo promedio por placa (ms)
    pub tiempo_promedio_placa_ms: u64,
    /// Tiempo total resolviendo captchas (ms)
    pub tiempo_captcha_ms: u64,
    /// Tiempo total consultando placa (ms)
    pub tiempo_consulta_ms: u64,
    /// Número total de reintentos realizados
    pub total_reintentos: u32,
    /// Estado del circuit breaker al finalizar
    pub circuit_breaker_state: String,
    /// Placas exitosas
    pub placas_exitosas: usize,
    /// Placas con timeout
    pub placas_timeout: usize,
    /// Placas con error de red
    pub placas_error_red: usize,
}

/// Evento de progreso para streaming al frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventoProgreso {
    /// Tipo de evento
    pub tipo: String,
    /// Placa actual siendo procesada
    pub placa_actual: Option<String>,
    /// Progreso (0.0 - 1.0)
    pub progreso: f64,
    /// Mensaje descriptivo
    pub mensaje: String,
    /// Timestamp
    pub timestamp: String,
    /// Número de placa actual / total
    pub indice_placa: usize,
    pub total_placas: usize,
}

/// Nivel de severidad del log
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
}

/// Evento de log para streaming en tiempo real al frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventoLogSimit {
    /// Timestamp del log
    pub timestamp: String,
    /// Nivel: info, success, warn, error
    pub level: LogLevel,
    /// Mensaje descriptivo del evento
    pub message: String,
    /// Placa asociada (si aplica)
    pub placa: Option<String>,
    /// Detalles adicionales (opcional)
    pub detail: Option<String>,
}

// ─── Cliente SIMIT (captcha PoW + consulta) ───────────────────────────────────

/// Cliente HTTP compartido con timeouts configurables (se reutiliza entre placas)
///
/// Fase 1: el jar de cookies (`cookie_store`) es persistente y se comparte
/// entre la siembra del sitio, el captcha y la consulta — como el `httpx`
/// con cookie jar del servicio Python de referencia. Sin él, el gateway ADC
/// del SIMIT rechaza la consulta con 401.
static AGENTE: once_cell::sync::Lazy<ureq::Agent> = once_cell::sync::Lazy::new(|| {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS))
        .cookie_store(CookieStore::default())
        .build()
});

fn agente() -> &'static ureq::Agent {
    &AGENTE
}

// ─── Sesión del sitio (siembra de cookies del gateway ADC) ───────────────────

/// Página principal del portal (SPA del SIMIT). Visitarla una vez, como un
/// navegador, hace que el gateway ADC emita sus cookies de sesión
/// (`ADC_CONN_*`/`ADC_REQ_*`) en el jar compartido.
const SITIO_URL: &str = "https://www.fcm.org.co/";

/// true si ya se sembró la sesión del sitio al menos una vez en este proceso
static SESION_SITIO_LISTA: AtomicBool = AtomicBool::new(false);

/// `GET` al sitio (con headers de navegador; sin `Origin`/`Referer`, que una
/// navegación de nivel superior no lleva) reutilizando el agente con jar.
/// Sigue hasta 2 redirecciones y captura las cookies de cada respuesta.
/// Best-effort: un 4xx/5xx o un fallo de red no aborta la sincronización
/// (el captcha/consulta pueden funcionar igual según el estado del gateway).
fn sembrar_cookies_sitio_con(agente: &ureq::Agent, url_inicial: &str) -> Result<(), AppError> {
    let mut url = url_inicial.to_string();
    for _ in 0..3 {
        let mut req = agente.get(&url);
        req = req
            .set("User-Agent", USER_AGENT)
            .set(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .set("Accept-Language", "es-ES,es;q=0.9");
        match req.call() {
            Ok(resp) => {
                if let Some(loc) = resp.header("location").map(str::to_string) {
                    url = loc;
                    continue;
                }
                log::debug!(
                    "Sesión SIMIT sembrada: GET {url_inicial} → HTTP {}",
                    resp.status()
                );
                return Ok(());
            }
            // El WAF puede responder 4xx con Set-Cookie; la cookie ya quedó en
            // el jar aunque el status no sea 2xx (ureq la almacena al crear la
            // respuesta). Se trata como "siembra hecha".
            Err(ureq::Error::Status(code, resp)) => {
                if let Some(loc) = resp.header("location").map(str::to_string) {
                    url = loc;
                    continue;
                }
                log::warn!(
                    "Sesión SIMIT: GET {url_inicial} → HTTP {code} (cookies del jar conservadas)"
                );
                return Ok(());
            }
            Err(e) => {
                return Err(AppError::Generic(format!(
                    "No se pudo sembrar la sesión SIMIT ({url_inicial}): {e}"
                )));
            }
        }
    }
    Ok(())
}

fn sembrar_cookies_sitio() -> Result<(), AppError> {
    sembrar_cookies_sitio_con(agente(), SITIO_URL)
}

/// Asegura la siembra una sola vez por proceso (best-effort; no aborta si falla).
fn asegurar_sesion_sitio() {
    if SESION_SITIO_LISTA.load(Ordering::SeqCst) {
        return;
    }
    match sembrar_cookies_sitio() {
        Ok(()) => SESION_SITIO_LISTA.store(true, Ordering::SeqCst),
        Err(e) => log::warn!("Agente SIMIT: {e}"),
    }
}

/// Re-siembra forzada tras un 401 del gateway: la cookie ADC puede haber
/// expirado; una navegación nueva la renueva en el jar compartido.
fn resembrar_cookies_sitio() {
    let _ = sembrar_cookies_sitio();
    SESION_SITIO_LISTA.store(true, Ordering::SeqCst);
}

/// Inicializa el agente HTTP con timeout configurado
pub fn init_http_agent(timeout_secs: u64) {
    // Como es Lazy, se inicializa en el primer acceso
    let _ = &*AGENTE;
    log::info!("Agente HTTP SIMIT: timeout={}s", timeout_secs);
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

/// Resuelve el captcha Proof-of-Work con reintentos inteligentes
/// Devuelve el token (array JSON de objetos de verificación) listo para
/// enviar al microservicio de consulta.
pub fn resolver_captcha_con_reintentos(
    max_retries: u32,
    base_delay_ms: u64,
) -> Result<(String, u64), AppError> {
    let mut ultimo_error = None;
    
    for intento in 0..=max_retries {
        // Verificar circuit breaker
        if !CIRCUIT_BREAKER.allow_request() {
            return Err(AppError::Generic(
                "Circuit Breaker abierto: el portal SIMIT no está disponible. Intenta más tarde.".into(),
            ));
        }
        
        match resolver_captcha() {
            Ok((token, duracion)) => {
                CIRCUIT_BREAKER.record_success();
                log::debug!(
                    "Captcha SIMIT resuelto en {}ms (intento {}/{})",
                    duracion,
                    intento + 1,
                    max_retries + 1
                );
                return Ok((token, duracion));
            }
            Err(e) => {
                CIRCUIT_BREAKER.record_failure();
                ultimo_error = Some(e);
                
                if intento < max_retries {
                    // Backoff exponencial: base * 2^intento + jitter
                    let delay = base_delay_ms * (1u64 << intento) + (rand::random::<u64>() % 500);
                    log::warn!(
                        "Captcha SIMIT falló (intento {}/{}), reintento en {}ms",
                        intento + 1,
                        max_retries + 1,
                        delay
                    );
                    std::thread::sleep(Duration::from_millis(delay));
                }
            }
        }
    }
    
    Err(ultimo_error.unwrap_or_else(|| AppError::Generic(
        "No se pudo resolver el captcha SIMIT después de múltiples intentos.".into(),
    )))
}

/// Resuelve el captcha Proof-of-Work y devuelve el token (array JSON con la
/// PRIMERA solución de verificación) listo para enviar al microservicio.
///
/// Fase 1: antes de resolver, se asegura la sesión del sitio (GET a la SPA)
/// para que el gateway ADC emita sus cookies en el jar compartido.
pub fn resolver_captcha() -> Result<(String, u64), AppError> {
    let inicio = Instant::now();
    asegurar_sesion_sitio();
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
    let token = resolver_pow(&data.question, tiempo, dificultad)?;
    // Fase 1: el microservicio acepta (y el servicio Python de referencia, que
    // se verificó exitoso, envía) solo la PRIMERA solución; el resto se descarta.
    let token = solo_primera_solucion(&token)?;
    let duracion = inicio.elapsed().as_millis() as u64;
    Ok((token, duracion))
}

/// Recorta el token PoW `[a,b,c,…]` a `[a]` (una sola solución de verificación).
/// El servicio Python de referencia envía `pow_solutions[:1]` — ver
/// app/procesos/simit/service.py de API-Runt-simit (verificado exitoso).
fn solo_primera_solucion(token: &str) -> Result<String, AppError> {
    let soluciones: Vec<serde_json::Value> = serde_json::from_str(token)
        .map_err(|e| AppError::Generic(format!("Token PoW inválido: {e}")))?;
    let primera = soluciones.first().ok_or_else(|| {
        AppError::Generic("Token PoW vacío: el captcha no produjo soluciones".into())
    })?;
    Ok(format!("[{primera}]"))
}

/// Consulta los comparendos/multas de una placa en el SIMIT con reintentos.
pub fn consultar_placa_con_reintentos(
    placa: &str,
    max_retries: u32,
    base_delay_ms: u64,
) -> Result<(Vec<RegistroSimit>, MetricasPlaca), AppError> {
    let mut ultimo_error = None;
    let mut metricas = MetricasPlaca::default();
    
    for intento in 0..=max_retries {
        // Verificar circuit breaker
        if !CIRCUIT_BREAKER.allow_request() {
            return Err(AppError::Generic(
                format!("Circuit Breaker abierto: no se puede consultar la placa {placa}")
            ));
        }
        
        let inicio = Instant::now();
        match consultar_placa(placa) {
            Ok((registros, tiempo_captcha, tiempo_consulta)) => {
                CIRCUIT_BREAKER.record_success();
                metricas.tiempo_captcha_ms += tiempo_captcha;
                metricas.tiempo_consulta_ms += tiempo_consulta;
                metricas.reintentos = intento as u32;
                log::debug!(
                    "Placa {placa} consultada OK: {} registros (intento {}/{}, {}ms)",
                    registros.len(),
                    intento + 1,
                    max_retries + 1,
                    inicio.elapsed().as_millis()
                );
                return Ok((registros, metricas));
            }
            Err(e) => {
                CIRCUIT_BREAKER.record_failure();
                metricas.reintentos = intento as u32;
                ultimo_error = Some(e);
                
                if intento < max_retries {
                    // Backoff exponencial con jitter
                    let delay = base_delay_ms * (1u64 << intento) + (rand::random::<u64>() % 500);
                    log::warn!(
                        "Placa {placa} falló (intento {}/{}), reintento en {}ms",
                        intento + 1,
                        max_retries + 1,
                        delay
                    );
                    std::thread::sleep(Duration::from_millis(delay));
                }
            }
        }
    }
    
    Err(ultimo_error.unwrap_or_else(|| AppError::Generic(
        format!("No se pudo consultar la placa {placa} después de múltiples intentos")
    )))
}

/// Métricas de una consulta individual de placa
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricasPlaca {
    pub tiempo_captcha_ms: u64,
    pub tiempo_consulta_ms: u64,
    pub reintentos: u32,
}

/// Fallo de la consulta al microservicio, con el 401 del gateway distinguido
/// (permite la re-siembra + reintento con token fresco en `consultar_placa`).
enum ErrorConsulta {
    /// 401 del gateway ("Autenticación fallida…") — lleva el body del SIMIT
    Unauthorized(String),
    /// Cualquier otro fallo (red, 5xx, parseo)
    Otro(AppError),
}

/// Envía el POST de consulta al microservicio. `url` es parametrizable para
/// poder testear el 401 y el manejo de cookies contra un servidor local.
fn enviar_consulta(
    placa: &str,
    token: &str,
    url: &str,
) -> Result<RespuestaConsulta, ErrorConsulta> {
    enviar_consulta_con(agente(), placa, token, url)
}

/// Igual que `enviar_consulta` pero con el agente HTTP inyectado: los tests
/// usan un agente propio con timeout amplio para no depender del agente
/// global de producción (evita flakes por timeout bajo carga paralela).
fn enviar_consulta_con(
    agente: &ureq::Agent,
    placa: &str,
    token: &str,
    url: &str,
) -> Result<RespuestaConsulta, ErrorConsulta> {
    let body = serde_json::json!({
        "filtro": placa.trim(),
        "reCaptchaDTO": { "response": token, "consumidor": "1" }
    });
    let respuesta = match con_headers_browser(agente.post(url))
        .set("Content-Type", "application/json")
        .send_json(body)
    {
        Ok(r) => r,
        // 401 del gateway: se conserva el detalle del body para el mensaje y
        // para decidir la re-siembra. Las Set-Cookie de esta respuesta ya
        // quedaron en el jar del agente.
        Err(ureq::Error::Status(401, resp)) => {
            let detalle = resp.into_string().unwrap_or_default();
            return Err(ErrorConsulta::Unauthorized(detalle));
        }
        Err(ureq::Error::Status(code, resp)) => {
            let detalle = resp.into_string().unwrap_or_default();
            return Err(ErrorConsulta::Otro(AppError::Generic(format!(
                "SIMIT respondió HTTP {code} para la placa {placa}: {detalle}"
            ))));
        }
        Err(e) => {
            return Err(ErrorConsulta::Otro(AppError::Generic(format!(
                "SIMIT no respondió para la placa {placa}: {e}"
            ))));
        }
    };
    respuesta.into_json().map_err(|e| {
        ErrorConsulta::Otro(AppError::Generic(format!(
            "Respuesta SIMIT inválida para la placa {placa}: {e}"
        )))
    })
}

/// Consulta los comparendos/multas de una placa en el SIMIT.
///
/// Fase 1: si el gateway rechaza con 401 (cookie ADC expirada o handshake
/// incompleto), se re-siembra la sesión del sitio y se reintenta UNA vez con
/// token fresco — el token PoW parece ser de un solo uso.
pub fn consultar_placa(placa: &str) -> Result<(Vec<RegistroSimit>, u64, u64), AppError> {
    // 1er intento: captcha + consulta
    let (token, duracion_captcha) = resolver_captcha()?;
    let inicio_consulta = Instant::now();
    match enviar_consulta(placa, &token, CONSULTA_URL) {
        Ok(dto) => {
            let tiempo_consulta = inicio_consulta.elapsed().as_millis() as u64;
            let registros = mapear_registros(&dto, placa.trim());
            log::debug!(
                "Placa {placa}: captcha={}ms, consulta={}ms",
                duracion_captcha,
                tiempo_consulta
            );
            return Ok((registros, duracion_captcha, tiempo_consulta));
        }
        Err(ErrorConsulta::Unauthorized(detalle)) => {
            log::warn!(
                "Placa {placa}: 401 del gateway ({detalle}); re-sembrando sesión y reintentando"
            );
            resembrar_cookies_sitio();
        }
        Err(ErrorConsulta::Otro(e)) => return Err(e),
    }

    // 2do intento (solo tras un 401): token fresco + sesión re-sembrada
    let (token2, duracion_captcha2) = resolver_captcha()?;
    let inicio_consulta2 = Instant::now();
    match enviar_consulta(placa, &token2, CONSULTA_URL) {
        Ok(dto) => {
            let tiempo_consulta = inicio_consulta2.elapsed().as_millis() as u64;
            let registros = mapear_registros(&dto, placa.trim());
            log::debug!(
                "Placa {placa}: captcha={}ms, consulta={}ms (reintento tras 401)",
                duracion_captcha + duracion_captcha2,
                tiempo_consulta
            );
            Ok((registros, duracion_captcha + duracion_captcha2, tiempo_consulta))
        }
        Err(ErrorConsulta::Unauthorized(detalle)) => {
            Err(AppError::Generic(format!(
                "SIMIT rechazó el token de seguridad para la placa {placa}: {detalle}"
            )))
        }
        Err(ErrorConsulta::Otro(e)) => Err(e),
    }
}

// ─── Sincronización con la base de datos ──────────────────────────────────────

/// Helper para emitir eventos de progreso al frontend
fn emitir_progreso<R: tauri::Runtime>(app: &tauri::AppHandle<R>, tipo: &str, placa: &str, idx: usize, total: usize, mensaje: &str) {
    let progreso = if total > 0 { idx as f64 / total as f64 } else { 1.0 };
    let evento = EventoProgreso {
        tipo: tipo.to_string(),
        placa_actual: Some(placa.to_string()),
        progreso,
        mensaje: mensaje.to_string(),
        timestamp: Local::now().to_rfc3339(),
        indice_placa: idx,
        total_placas: total,
    };
    let _ = app.emit("simit-sync-progress", &evento);
}

/// Helper para emitir eventos de log al frontend
fn emitir_log<R: tauri::Runtime>(app: &tauri::AppHandle<R>, level: LogLevel, message: &str, placa: Option<&str>, detail: Option<&str>) {
    let evento = EventoLogSimit {
        timestamp: Local::now().format("%H:%M:%S").to_string(),
        level,
        message: message.to_string(),
        placa: placa.map(|s| s.to_string()),
        detail: detail.map(|s| s.to_string()),
    };
    let _ = app.emit("simit-sync-log", &evento);
}

/// Ejecuta una sincronización completa: consulta todas las placas de la flota,
/// inserta los comparendos nuevos y genera el reporte HTML.
/// Si se proporciona `app`, emite eventos de progreso al frontend.
pub fn sincronizar<R: tauri::Runtime>(
    conn: &mut PooledConnection,
    cfg: &Arc<AppConfig>,
    app: Option<&tauri::AppHandle<R>>,
) -> Result<ResultadoSincronizacion, AppError> {
    let inicio_total = Instant::now();
    let placas = AutoRepository::placas_activas(conn)?;
    let total_placas = placas.len();
    
    log::info!(
        "Agente SIMIT: iniciando sincronización con {} placas",
        total_placas
    );
    
    // Emitir evento de inicio y log
    if let Some(app) = app {
        emitir_progreso(app, "inicio", "", 0, total_placas, &format!("Iniciando sincronización con {} placas", total_placas));
        emitir_log(app, LogLevel::Info, &format!("Iniciando sincronización de {} placas", total_placas), None, None);
    }
    
    let mut resultado = ResultadoSincronizacion {
        sincronizado_en: Local::now().to_rfc3339(),
        ..Default::default()
    };
    
    let mut metricas = MetricasSimit::default();
    let max_retries = cfg.simit_max_retries;
    let base_delay_ms = cfg.simit_retry_base_delay_ms;

    for (idx, placa) in placas.iter().enumerate() {
        log::debug!(
            "Agente SIMIT: procesando placa {} ({}/{})",
            placa,
            idx + 1,
            total_placas
        );
        
        // Emitir evento de progreso antes de procesar cada placa
        if let Some(app) = app {
            emitir_progreso(app, "placa", placa, idx, total_placas, &format!("Consultando placa {} ({}/{})", placa, idx + 1, total_placas));
        }
        
        match consultar_placa_con_reintentos(placa, max_retries, base_delay_ms) {
            Ok((registros, metricas_placa)) => {
                resultado.placas_consultadas += 1;
                metricas.placas_exitosas += 1;
                
                // Log de éxito
                if let Some(app) = app {
                    emitir_log(app, LogLevel::Success, &format!("Placa {} — {} registro(s) encontrado(s)", placa, registros.len()), Some(placa), None);
                }
                metricas.tiempo_captcha_ms += metricas_placa.tiempo_captcha_ms;
                metricas.tiempo_consulta_ms += metricas_placa.tiempo_consulta_ms;
                metricas.total_reintentos += metricas_placa.reintentos;
                
                for mut reg in registros {
                    resultado.encontrados += 1;
                    // Fecha inválida → se omite el registro (no aborta la placa).
                    // Va ANTES del dedup: id_existente llama parse_fecha
                    // y una fecha malformada (p.ej. sin número oficial) abortaría
                    // toda la sincronización en vez de omitir el registro.
                    if NaiveDate::parse_from_str(&reg.fecha_infraccion, "%Y-%m-%d").is_err() {
                        log::warn!(
                            "Agente SIMIT: fecha inválida para {} ({}), registro omitido",
                            reg.placa,
                            reg.fecha_infraccion
                        );
                        resultado.duplicados += 1;
                        continue;
                    }
                    // ¿Ya existe? (número oficial o placa+fecha+monto). Si el
                    // SIMIT reporta pagado un comparendo ya registrado, se
                    // sincroniza el estado (la BD converge con el SIMIT). En
                    // ambos casos se toca `ultimo_visto_simit` (confirmación)
                    // y se conserva el id para marcar el registro en la UI.
                    let numero =
                        reg.numero.as_deref().map(str::trim).filter(|n| !n.is_empty());
                    if let Some(id) = ComparendoRepository::id_existente(
                        conn,
                        numero,
                        &reg.placa,
                        &reg.fecha_infraccion,
                        &reg.monto,
                    )? {
                        reg.id = Some(id);
                        ComparendoRepository::marcar_visto_simit_por_id(conn, id)?;
                        if reg.estado == "Pagado" {
                            if let Some(num) = numero {
                                ComparendoRepository::marcar_pagado_por_numero(conn, num)?;
                            }
                        }
                        resultado.duplicados += 1;
                        resultado.registros.push(reg);
                        continue;
                    }
                    let mut datos = ComparendoDatos {
                        placa: reg.placa.clone(),
                        fecha_infraccion: reg.fecha_infraccion.clone(),
                        hora_infraccion: reg.hora_infraccion.clone(),
                        monto: reg.monto.clone(),
                        numero_comparendo: reg.numero.clone(),
                        id_renta: None,
                        id_cliente: None,
                        estado: reg.estado.clone(),
                        observaciones: Some(observaciones_para(&reg)),
                        // Procedencia persistente: este comparendo vino del SIMIT.
                        origen: Some("SIMIT".into()),
                    };
                    // Atribución persistente: se resuelve qué renta cubría el
                    // vehículo el día de la infracción y se guarda el vínculo
                    // (id_renta/id_cliente) — el comparendo queda asociado al
                    // cliente que tenía el vehículo (cruce comparendos↔rentas).
                    if let Some((id_renta, id_cliente)) =
                        ComparendoRepository::renta_del_dia(conn, &reg.placa, &reg.fecha_infraccion)?
                    {
                        datos.id_renta = Some(id_renta);
                        datos.id_cliente = id_cliente;
                    }
                    let id = ComparendoRepository::insertar(conn, &datos)?;
                    // El Agente acaba de confirmar este comparendo en el portal:
                    // se toca ultimo_visto_simit (y origen converge a SIMIT).
                    ComparendoRepository::marcar_visto_simit_por_id(conn, id)?;
                    reg.id = Some(id);
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
                
                // Log de error
                if let Some(app) = app {
                    emitir_log(app, LogLevel::Error, &format!("Placa {} — error: {}", placa, e.mensaje_usuario()), Some(placa), Some(&e.mensaje_usuario()));
                }
                
                // Clasificar tipo de error
                let error_msg = e.mensaje_usuario();
                if error_msg.contains("timeout") || error_msg.contains("Timeout") {
                    metricas.placas_timeout += 1;
                } else if error_msg.contains("red") || error_msg.contains("network") {
                    metricas.placas_error_red += 1;
                }
            }
        }
        // Espera corta entre placas para no saturar el portal
        if cfg.simit_polite_delay_ms > 0 && idx < total_placas - 1 {
            std::thread::sleep(Duration::from_millis(cfg.simit_polite_delay_ms));
        }
        
        // Emitir evento de progreso después de procesar cada placa
        if let Some(app) = app {
            let estado_placa = if resultado.errores.iter().any(|e| e.placa == *placa) { "error" } else { "ok" };
            emitir_progreso(app, "placa_completada", placa, idx + 1, total_placas, &format!("Placa {} {} — {}/{}", placa, estado_placa, idx + 1, total_placas));
        }
    }

    // Calcular métricas finales
    let tiempo_total = inicio_total.elapsed().as_millis() as u64;
    metricas.tiempo_total_ms = tiempo_total;
    metricas.tiempo_promedio_placa_ms = if resultado.placas_consultadas > 0 {
        tiempo_total / resultado.placas_consultadas as u64
    } else {
        0
    };
    metricas.circuit_breaker_state = format!("{:?}", CIRCUIT_BREAKER.current_state());
    resultado.metricas = metricas;
    
    log::info!(
        "Agente SIMIT: sincronización completada en {}ms - {} placas consultadas, {} nuevos, {} errores",
        tiempo_total,
        resultado.placas_consultadas,
        resultado.insertados,
        resultado.placas_con_error
    );
    
    // Log de finalización
    if let Some(app) = app {
        let nivel = if resultado.placas_con_error > 0 { LogLevel::Warn } else { LogLevel::Success };
        emitir_log(app, nivel, &format!("Sincronización completada — {} placas, {} nuevos, {} errores, {}ms", resultado.placas_consultadas, resultado.insertados, resultado.placas_con_error, tiempo_total), None, Some(&format!("Total pendiente: ${}", resultado.total_pendiente)));
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
    
    // Emitir evento de finalización
    if let Some(app) = app {
        emitir_progreso(
            app,
            "completado",
            "",
            total_placas,
            total_placas,
            &format!(
                "Sincronización completada — {} placas, {} nuevos",
                resultado.placas_consultadas,
                resultado.insertados
            ),
        );
    }

    Ok(resultado)
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
    /// Próxima corrida programada (RFC3339 local); la mantiene el scheduler
    proxima_sincronizacion: Option<String>,
    ultimo_resultado: Option<ResultadoSincronizacion>,
    ultimo_error: Option<String>,
}

/// Info serializable del agente para la UI
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoAgenteSimit {
    pub habilitado: bool,
    pub interval_hours: u64,
    /// Minutos de retraso de la primera corrida tras el arranque (0 = inmediata)
    pub start_delay_minutes: u64,
    pub ejecutando: bool,
    pub ultima_sincronizacion: Option<String>,
    /// Próxima corrida programada (RFC3339 local) — la mantiene el scheduler
    pub proxima_sincronizacion: Option<String>,
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
            start_delay_minutes: cfg.simit_start_delay_minutes,
            ejecutando: self.esta_ejecutando(),
            ultima_sincronizacion: interno.ultima_sincronizacion.clone(),
            proxima_sincronizacion: interno.proxima_sincronizacion.clone(),
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

    /// Registra el último error (visible en el panel). Lo usan el scheduler
    /// (corrida omitida por DNS) y el comando manual (sync fallida).
    pub(crate) fn registrar_error(&self, error: &str) {
        let mut interno = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        interno.ultimo_error = Some(error.to_string());
    }

    /// Fija la próxima corrida programada (visible en el panel). Solo la
    /// mantiene el scheduler; la sincronización manual no altera el ciclo.
    fn fijar_proxima(&self, proxima: Option<String>) {
        let mut interno = self.interno.lock().unwrap_or_else(|e| e.into_inner());
        interno.proxima_sincronizacion = proxima;
    }
}

// ─── Persistencia del último resultado (filtro «Solo nuevos» tras reinicio) ─

/// Fila única de `agente_simit_ultimo_resultado` (id fijo, upsert).
const ULTIMO_RESULTADO_ID: i16 = 1;

/// Persiste el último resultado de sincronización como JSON en la BD (una
/// sola fila, upsert). Sin esto el filtro «Solo nuevos» y el panel perdían
/// la última corrida al reiniciar la app; con esto se restauran al arrancar.
pub fn persistir_ultimo_resultado(
    conn: &mut PooledConnection,
    resultado: &ResultadoSincronizacion,
) -> Result<(), AppError> {
    let json = serde_json::to_string(resultado)
        .map_err(|e| AppError::Generic(format!("serializar último resultado: {e}")))?;
    let actualizadas = conn.execute(
        "UPDATE agente_simit_ultimo_resultado \
         SET resultado_json = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        (json.clone(), ULTIMO_RESULTADO_ID),
    )?;
    if actualizadas == 0 {
        conn.execute(
            "INSERT INTO agente_simit_ultimo_resultado (id, resultado_json) VALUES (?, ?)",
            (ULTIMO_RESULTADO_ID, json),
        )?;
    }
    Ok(())
}

/// Carga el último resultado persistido (None si aún no hay corrida guardada).
pub fn cargar_ultimo_resultado(
    conn: &mut PooledConnection,
) -> Result<Option<ResultadoSincronizacion>, AppError> {
    let rows: Vec<(Option<String>,)> = conn.query(
        "SELECT resultado_json FROM agente_simit_ultimo_resultado WHERE id = ?",
        (ULTIMO_RESULTADO_ID,),
    )?;
    let Some((Some(json),)) = rows.first() else {
        return Ok(None);
    };
    let resultado = serde_json::from_str(json)
        .map_err(|e| AppError::Generic(format!("parsear último resultado persistido: {e}")))?;
    Ok(Some(resultado))
}

/// Restaura en el estado en memoria el último resultado persistido (arranque).
/// Best-effort: si la BD no tiene fila o falla la lectura, el agente arranca
/// en blanco como antes (la primera corrida programada lo llena de nuevo).
pub(crate) fn restaurar_ultimo_resultado(pool: &Pool, estado: &EstadoAgenteSimit) {
    match pool.get() {
        Ok(mut conn) => match cargar_ultimo_resultado(&mut conn) {
            Ok(Some(resultado)) => {
                log::info!(
                    "Agente SIMIT: último resultado restaurado ({} nuevas, corrida de {})",
                    resultado.insertados,
                    resultado.sincronizado_en
                );
                estado.registrar_ok(resultado);
            }
            Ok(None) => {}
            Err(e) => log::warn!(
                "Agente SIMIT: no se pudo cargar el último resultado persistido: {e}"
            ),
        },
        Err(e) => log::warn!(
            "Agente SIMIT: no se pudo conectar para restaurar el último resultado: {e}"
        ),
    }
}

/// Ejecuta una sincronización y actualiza el estado del agente.
/// Es la única entrada compartida por el scheduler y el comando manual.
/// Si se proporciona `app`, emite eventos de progreso al frontend.
///
/// Fast-fail DNS: si el portal no resuelve, devuelve error al instante en vez
/// de esperar el timeout HTTP (30 s) por placa. Cubre la sincronización manual
/// («Sincronizar ahora»); el scheduler hace además su propio pre-check para no
/// reintentar cada 60 s cuando el portal está caído.
pub fn run_sync<R: tauri::Runtime>(
    pool: &Pool,
    cfg: &Arc<AppConfig>,
    estado: &EstadoAgenteSimit,
    app: Option<&tauri::AppHandle<R>>,
) -> Result<ResultadoSincronizacion, AppError> {
    // Inicializar circuit breaker con configuración
    init_circuit_breaker(cfg.simit_circuit_breaker_threshold, cfg.simit_circuit_breaker_timeout_seconds);

    if !portal_simit_accesible() {
        let msg = "El portal SIMIT no está accesible en este momento. Verifica tu conexión a internet e inténtalo más tarde.";
        log::warn!("Agente SIMIT: {msg}");
        return Err(AppError::Generic(msg.into()));
    }

    let mut conn = pool.get()?;
    let resultado = sincronizar(&mut conn, cfg, app)?;
    // Persistir el resultado (filtro «Solo nuevos» y panel sobreviven al
    // reinicio). Best-effort: si falla, el estado en memoria sigue valiendo
    // para la sesión actual y se reintenta en la siguiente corrida.
    if let Err(e) = persistir_ultimo_resultado(&mut conn, &resultado) {
        log::warn!("Agente SIMIT: no se pudo persistir el último resultado: {e}");
    }
    estado.registrar_ok(resultado.clone());
    Ok(resultado)
}

/// Lanza el hilo de fondo del agente. La **primera** corrida espera
/// `simit.start_delay_minutes` (default 10 min; 0 = inmediata, comportamiento
/// previo) para no competir con el arranque de la app (CPU del PoW + red).
/// Después corre cada `simit.interval_hours`.
///
/// Chequeo DNS previo a cada corrida: si los subdominios del portal no
/// resuelven (SIMIT caído, como el 10-08), la corrida se omite al instante en
/// lugar de esperar el timeout HTTP (30 s) por placa, y la siguiente consulta
/// ocurre en el ciclo normal.
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
        // Referencia para el retraso inicial configurable (0 = inmediato)
        let inicio = Instant::now();
        let mut ultima_ejecucion: Option<Instant> = None;
        // Próxima corrida inicial (para el panel): tras el retraso configurable
        if cfg.simit_enabled {
            estado.fijar_proxima(Some(ahora_mas(
                cfg.simit_start_delay_minutes.saturating_mul(60),
            )));
        }
        loop {
            if cfg.simit_enabled {
                let debe_ejecutar = match ultima_ejecucion {
                    None => inicio.elapsed()
                        >= Duration::from_secs(cfg.simit_start_delay_minutes.saturating_mul(60)),
                    Some(t) => {
                        t.elapsed()
                            >= Duration::from_secs(cfg.simit_interval_hours.saturating_mul(3600))
                    }
                };
                if debe_ejecutar {
                    // Chequeo DNS previo: si el portal no resuelve, se omite la
                    // corrida al instante (cada placa tardaría ~30 s en fallar
                    // por timeout HTTP). La siguiente corrida vuelve al ciclo
                    // normal (interval_hours), no al tick de 60 s.
                    if portal_simit_accesible() {
                        if estado.claimar() {
                            match run_sync(&pool, &cfg, &estado, Some(&app)) {
                            Ok(resultado) => {
                                ultima_ejecucion = Some(Instant::now());
                                estado.fijar_proxima(Some(ahora_mas(
                                    cfg.simit_interval_hours.saturating_mul(3600),
                                )));
                                log::info!(
                                    "Agente SIMIT: sincronización OK — {} placas, {} nuevos",
                                    resultado.placas_consultadas,
                                    resultado.insertados
                                );
                                let _ = app.emit("simit-sync-complete", &resultado);
                            }
                            Err(e) => {
                                estado.registrar_error(&e.to_string());
                                // Reintento en el siguiente tick (60 s)
                                estado.fijar_proxima(Some(ahora_mas(60)));
                                log::error!("Agente SIMIT: falló la sincronización: {e}");
                                // No se marca ultima_ejecucion → reintento en el siguiente tick
                            }
                            }
                            estado.liberar();
                        }
                    } else {
                        let msg = "Portal SIMIT inalcanzable (DNS) — corrida omitida. Reintento en el siguiente ciclo.";
                        log::warn!("Agente SIMIT: {msg}");
                        emitir_log(&app, LogLevel::Warn, msg, None, None);
                        estado.registrar_error(msg);
                        ultima_ejecucion = Some(Instant::now());
                        estado.fijar_proxima(Some(ahora_mas(
                            cfg.simit_interval_hours.saturating_mul(3600),
                        )));
                    }
                }
            }
            std::thread::sleep(Duration::from_secs(60));
        }
    });
}

/// Chequeo DNS previo del portal SIMIT (espejo de `scripts/check-simit.mjs`):
/// si los subdominios no resuelven (el 10-08 desaparecieron del DNS mientras
/// el dominio raíz seguía vivo), la corrida se omite sin gastar timeouts HTTP.
/// Usa el resolvedor del sistema (bloqueante pero rápido; corre en el hilo del
/// scheduler, no en el de la UI).
/// Hora local RFC3339 dentro de `secs` segundos (próxima corrida del panel)
fn ahora_mas(secs: u64) -> String {
    (Local::now() + chrono::Duration::seconds(secs as i64)).to_rfc3339()
}

fn portal_simit_accesible() -> bool {
    ["qxcaptcha.fcm.org.co", "consultasimit.fcm.org.co"]
        .iter()
        .all(|host| match (*host, 443u16).to_socket_addrs() {
            Ok(mut addrs) => addrs.next().is_some(),
            Err(_) => false,
        })
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
                id: None,
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
/// "2026-01-15", "2026-01-15 14:30:00", ISO con 'T' o DD/MM/YYYY
/// ("27/01/2025" — el SIMIT envía fechaComparendo en formato latino).
fn parsear_fecha_hora(raw: &str) -> (String, String) {
    let raw = raw.trim();
    if raw.is_empty() {
        return ("".into(), "00:00".into());
    }
    // Separa fecha y hora por 'T' (ISO) o por espacio; la hora puede traer
    // un 'T' o espacios sobrantes que se recortan antes de tomar HH:MM.
    let mut partes = raw.splitn(2, ['T', ' ']);
    let fecha_raw = partes.next().unwrap_or("").trim();
    let hora_raw = partes.next().unwrap_or("").trim();
    let hora: String = hora_raw
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take(5)
        .collect();
    let hora = if hora.is_empty() { "00:00".into() } else { hora };
    // Normaliza DD/MM/YYYY → AAAA-MM-DD (dominio en ISO). ISO y variantes
    // con separadores '-' pasan sin cambios (bytes[2] != '/').
    let fecha = if fecha_raw.len() == 10
        && fecha_raw.as_bytes().get(2) == Some(&b'/')
        && fecha_raw.as_bytes().get(5) == Some(&b'/')
    {
        format!(
            "{}-{}-{}",
            &fecha_raw[6..10],
            &fecha_raw[3..5],
            &fecha_raw[0..2]
        )
    } else {
        fecha_raw.to_string()
    };
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
    use serial_test::serial;
    use std::io::{Read, Write};
    use std::net::TcpListener;

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
        // El SIMIT envía fechaComparendo en formato latino DD/MM/YYYY
        assert_eq!(
            parsear_fecha_hora("27/01/2025"),
            ("2025-01-27".into(), "00:00".into())
        );
        assert_eq!(
            parsear_fecha_hora("02/02/2026 14:30:00"),
            ("2026-02-02".into(), "14:30".into())
        );
    }

    #[test]
    fn mapeo_respuesta_completa() {
        // FechaComparendo en formato real del SIMIT (latino DD/MM/YYYY)
        let json = r#"{
            "multas": [
                {
                    "comparendo": true,
                    "numeroComparendo": "250010000000123",
                    "valorPagar": 580000.5,
                    "estadoComparendo": "PENDIENTE",
                    "fechaComparendo": "12/03/2026 09:15:00",
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
            id: None,
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

    // ─── Fase 1: token de una sola solución ─────────────────────────────────

    #[test]
    fn solo_primera_solucion_recorta_a_la_primera() {
        let token = resolver_pow("question-test", 1700000000, 2).expect("pow resoluble");
        let recortado = solo_primera_solucion(&token).expect("token válido");
        let parseado: Vec<serde_json::Value> =
            serde_json::from_str(&recortado).expect("array JSON");
        assert_eq!(parseado.len(), 1, "solo la primera solución");
        assert_eq!(parseado[0]["question"], "question-test");
    }

    #[test]
    fn solo_primera_solucion_token_vacio_error() {
        assert!(solo_primera_solucion("[]").is_err());
        assert!(solo_primera_solucion("no-json").is_err());
    }

    // ─── Fase 1: jar de cookies compartido entre peticiones ────────────────

    /// Timeout generoso para un round-trip HTTP en loopback (evita cuelgues
    /// del hilo del servidor si el cliente nunca conecta o aborta).
    const TIMEOUT_MOCK: std::time::Duration = std::time::Duration::from_secs(10);

    /// Mini servidor HTTP de test: responde la secuencia dada y devuelve el
    /// header `Cookie` de cada request recibido (para verificar el jar).
    ///
    /// Robusto bajo paralelismo (arreglo del flake de `consulta_401`): el
    /// accept es no bloqueante con deadline, el read tiene timeout y la
    /// escritura es best-effort — si el cliente aborta la conexión (EPIPE) o
    /// no llega, el hilo falla con un panic descriptivo en vez de colgar al
    /// test o paniquear con un mensaje engañoso.
    fn servidor_http(
        respuestas: Vec<(u16, Vec<(&'static str, &'static str)>, String)>,
    ) -> (String, std::thread::JoinHandle<Vec<Option<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("puerto de test libre");
        let addr = listener.local_addr().expect("dirección del listener");
        listener
            .set_nonblocking(true)
            .expect("listener de test no bloqueante");
        let handle = std::thread::spawn(move || {
            let mut cookies_recibidas = Vec::new();
            for (status, headers, body) in respuestas {
                // Accept con deadline: espera con backoff de 5 ms y aborta con
                // un panic claro si el cliente no conecta a tiempo.
                let inicio = std::time::Instant::now();
                let mut stream = loop {
                    match listener.accept() {
                        Ok((s, _)) => break s,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            if inicio.elapsed() > TIMEOUT_MOCK {
                                panic!(
                                    "servidor de test: el cliente no conectó en {TIMEOUT_MOCK:?}"
                                );
                            }
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(e) => panic!("servidor de test: accept falló: {e}"),
                    }
                };
                // El socket aceptado hereda el modo no bloqueante del listener
                // en Windows: el primer read() devolvía WouldBlock (os error
                // 10035) antes de llegar los bytes del request, el hilo del
                // servidor paniqueaba y el cliente recibía una conexión
                // abortada (os error 10053) — el flake de CI. Forzamos modo
                // bloqueante y dejamos que SO_RCVTIMEO haga el deadline.
                stream
                    .set_nonblocking(false)
                    .expect("socket de test en modo bloqueante");
                stream
                    .set_read_timeout(Some(TIMEOUT_MOCK))
                    .expect("read timeout del socket de test");
                let inicio_read = std::time::Instant::now();
                let mut buf = Vec::new();
                let mut tmp = [0u8; 2048];
                loop {
                    match stream.read(&mut tmp) {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&tmp[..n]);
                            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                        // Defensivo: en modo bloqueante no debería ocurrir, pero
                        // si alguna plataforma devuelve WouldBlock/Interrupted se
                        // reintenta hasta el deadline en vez de paniquear.
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::Interrupted =>
                        {
                            if inicio_read.elapsed() > TIMEOUT_MOCK {
                                panic!("servidor de test: timeout leyendo el request ({e})");
                            }
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            panic!("servidor de test: timeout leyendo el request ({e})");
                        }
                        Err(e) => panic!("servidor de test: error leyendo el request: {e}"),
                    }
                }
                let req = String::from_utf8_lossy(&buf);
                let cookie = req
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("cookie:"))
                    .map(|l| l.trim().to_string());
                cookies_recibidas.push(cookie);
                let mut resp = format!("HTTP/1.1 {status} OK\r\nConnection: close\r\n");
                for (k, v) in &headers {
                    resp.push_str(&format!("{k}: {v}\r\n"));
                }
                resp.push_str(&format!("Content-Length: {}\r\n\r\n{}", body.len(), body));
                // Best-effort: si el cliente abortó la conexión (EPIPE/RST), no
                // paniquear — el test del lado cliente ya está fallando con su
                // propio mensaje. Se registra y se sigue con la siguiente.
                if let Err(e) = stream.write_all(resp.as_bytes()) {
                    eprintln!("[servidor de test] cliente abortó al escribir la respuesta: {e}");
                }
            }
            cookies_recibidas
        });
        (format!("http://{addr}"), handle)
    }

    #[test]
    #[serial]
    fn cookie_jar_compartido_entre_peticiones() {
        let (base, server) = servidor_http(vec![
            (200, vec![("Set-Cookie", "adc_test=xyz123; Path=/")], String::new()),
            (200, vec![], "ok".into()),
        ]);
        // Agente propio con timeout amplio: no depende del agente global de
        // producción ni de su timeout de 30 s (causa del flake bajo carga
        // paralela en CI — un timeout de transporte reseteaba la conexión
        // loopback y el test paniqueaba; mismo patrón que consulta_401).
        let agente = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(60))
            .cookie_store(CookieStore::default())
            .build();
        agente
            .get(&format!("{base}/captcha"))
            .call()
            .expect("primera petición");
        agente
            .get(&format!("{base}/consulta"))
            .call()
            .expect("segunda petición");
        let cookies = server.join().expect("servidor de test");
        assert_eq!(cookies[0], None, "la primera petición no lleva cookie");
        let cookie2 = cookies[1]
            .as_deref()
            .expect("la segunda petición lleva la cookie");
        assert!(
            cookie2.contains("adc_test=xyz123"),
            "jar compartido: la cookie de la 1ª petición viaja en la 2ª"
        );
    }

    #[test]
    #[serial]
    fn sembrar_cookies_sitio_siembra_el_jar() {
        let (base, server) = servidor_http(vec![
            (200, vec![("Set-Cookie", "ADC_CONN=abc; Path=/")], "<html>".into()),
            (200, vec![], "ok".into()),
        ]);
        let agente = ureq::AgentBuilder::new()
            .cookie_store(CookieStore::default())
            .build();
        sembrar_cookies_sitio_con(&agente, &base).expect("siembra ok");
        agente
            .get(&format!("{base}/x"))
            .call()
            .expect("petición de verificación");
        let cookies = server.join().expect("servidor de test");
        assert!(cookies[0].is_none());
        let cookie2 = cookies[1]
            .as_deref()
            .expect("la cookie del sitio viaja después");
        assert!(cookie2.contains("ADC_CONN=abc"));
    }

    #[test]
    #[serial]
    fn consulta_401_clasifica_como_unauthorized() {
        let (base, server) = servidor_http(vec![(
            401,
            vec![("Content-Type", "application/json")],
            r#"{"codigo":5,"descripcion":"Autenticación fallida: Acceso denegado"}"#.into(),
        )]);
        // Agente propio del test con timeout amplio: no depende del agente
        // global de producción ni de su timeout de 30 s (causa del flake bajo
        // carga paralela — un timeout de transporte rompía la clasificación
        // del 401 y el test paniqueaba con "se esperaba Unauthorized").
        let agente = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(60))
            .cookie_store(CookieStore::default())
            .build();
        let err = enviar_consulta_con(&agente, "ABC123", "TOKEN", &base)
            .expect_err("401 → Unauthorized");
        match err {
            ErrorConsulta::Unauthorized(detalle) => {
                assert!(
                    detalle.contains("Autenticación fallida"),
                    "el body del SIMIT se conserva para el mensaje"
                );
            }
            ErrorConsulta::Otro(e) => panic!("se esperaba Unauthorized, no {e}"),
        }
        server.join().expect("servidor de test");
    }

    /// Integración contra el portal REAL (no corre en la suite normal):
    /// ejecuta la siembra del sitio + captcha con el agente global y falla si
    /// el jar no captura las cookies del gateway ADC (`aiovg_rand_seed` del
    /// GET del sitio y `ADC_CONN_*`/`ADC_REQ_*` de la respuesta del captcha),
    /// replicando la verificación manual del 11-08.
    ///
    /// Ejecutar con:
    ///   cargo test --lib jar_portal_real_captura_cookies_adc -- --ignored --nocapture
    #[test]
    #[ignore = "requiere el portal SIMIT real (internet); correr con -- --ignored"]
    fn jar_portal_real_captura_cookies_adc() {
        // Siembra explícita (GET www.fcm.org.co → aiovg_rand_seed) + captcha
        // PoW (POST qxcaptcha → ADC_CONN_*/ADC_REQ_*), ambos con el agente
        // global real de la sincronización.
        sembrar_cookies_sitio_con(agente(), SITIO_URL).expect("siembra del sitio real");
        let (token, _duracion) = resolver_captcha().expect("captcha contra el portal real");
        assert!(!token.is_empty(), "token PoW generado");

        let guard = AGENTE.cookie_store();
        let nombres: Vec<String> = guard.iter_any().map(|c| c.name().to_string()).collect();
        let resumen = nombres.join(", ");
        println!("[jar:portal-real] cookies capturadas → {resumen}");

        assert!(
            nombres.iter().any(|n| n == "aiovg_rand_seed"),
            "falta aiovg_rand_seed (GET del sitio) en el jar — cookies: {resumen}"
        );
        assert!(
            nombres.iter().any(|n| n.starts_with("ADC_CONN_")),
            "falta ADC_CONN_* (respuesta del captcha) en el jar — cookies: {resumen}"
        );
        assert!(
            nombres.iter().any(|n| n.starts_with("ADC_REQ_")),
            "falta ADC_REQ_* (respuesta del captcha) en el jar — cookies: {resumen}"
        );
    }
}
