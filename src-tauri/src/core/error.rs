//! error.rs — Errores de aplicación (puerto de core/exceptions.py)
//!
//! AppError se serializa como `{ "kind": ..., "message": ..., "detail": ... }`
//! para que el frontend muestre `mensaje_usuario` de forma uniforme.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AppError {
    /// Error genérico de base de datos
    Database(String),
    /// Registro no encontrado
    NotFound(String),
    /// Registro duplicado
    Duplicate(String),
    /// Validación de datos de entrada
    Validation(String),
    /// Regla de negocio violada
    Business(String),
    /// Credenciales inválidas
    InvalidCredentials,
    /// Sesión expirada / no existe
    SessionExpired,
    /// Permisos insuficientes (RBAC)
    Permission,
    /// Cuenta bloqueada por intentos fallidos
    AccountLocked { remaining_seconds: u64 },
    /// Rate limit excedido
    RateLimited,
    /// Entrada maliciosa (XSS/SQLi)
    Sanitization(String),
    /// Error de criptografía
    Crypto(String),
    /// Error genérico
    Generic(String),
}

impl AppError {
    /// Mensaje mostrado al usuario final (espejo de `mensaje_usuario`)
    pub fn mensaje_usuario(&self) -> String {
        match self {
            AppError::Database(_) => "Error al acceder a la base de datos.".into(),
            AppError::NotFound(_) => "El registro solicitado no existe.".into(),
            AppError::Duplicate(_) => "Ya existe un registro con esos datos.".into(),
            AppError::Validation(m) => m.clone(),
            AppError::Business(m) => m.clone(),
            AppError::InvalidCredentials => "Usuario o contraseña incorrectos.".into(),
            AppError::SessionExpired => {
                "Tu sesión ha expirado. Por favor inicia sesión nuevamente.".into()
            }
            AppError::Permission => "No tienes permisos para realizar esta acción.".into(),
            AppError::AccountLocked { .. } => {
                "Tu cuenta ha sido bloqueada por múltiples intentos fallidos. Contacta al administrador."
                    .into()
            }
            AppError::RateLimited => {
                "Demasiados intentos de inicio de sesión. Por favor espera antes de intentar nuevamente."
                    .into()
            }
            AppError::Sanitization(_) => "El texto contiene caracteres no permitidos.".into(),
            AppError::Crypto(_) => "Error de seguridad al procesar los datos.".into(),
            AppError::Generic(m) => m.clone(),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Database(_) => "database",
            AppError::NotFound(_) => "not_found",
            AppError::Duplicate(_) => "duplicate",
            AppError::Validation(_) => "validation",
            AppError::Business(_) => "business",
            AppError::InvalidCredentials => "invalid_credentials",
            AppError::SessionExpired => "session_expired",
            AppError::Permission => "permission",
            AppError::AccountLocked { .. } => "account_locked",
            AppError::RateLimited => "rate_limited",
            AppError::Sanitization(_) => "sanitization",
            AppError::Crypto(_) => "crypto",
            AppError::Generic(_) => "generic",
        }
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            AppError::Database(d)
            | AppError::NotFound(d)
            | AppError::Duplicate(d)
            | AppError::Validation(d)
            | AppError::Business(d)
            | AppError::Sanitization(d)
            | AppError::Crypto(d)
            | AppError::Generic(d) => Some(d.clone()),
            AppError::AccountLocked { remaining_seconds } => Some(format!(
                "Cuenta bloqueada. Restan {remaining_seconds} segundos."
            )),
            _ => None,
        }
    }

    /// Serializa el error al formato esperado por el frontend
    pub fn to_payload(&self) -> ErrorPayload {
        // Registro en el log (en producción: data_dir/logs/app.log, ver lib.rs).
        // TODOS los errores que llegan a la UI pasan por aquí; sin este registro
        // el detalle real de Firebird (SQLCODE, columna, lock conflict...) quedaba
        // perdido y solo se veía el mensaje genérico "Error al acceder a la base
        // de datos.". Los errores de BD van a log::error!; el resto a warn.
        match self {
            AppError::Database(d) => log::error!("BD: {d}"),
            _ => match self.detail() {
                Some(d) => log::warn!("[{}] {} — {d}", self.kind(), self.mensaje_usuario()),
                None => log::warn!("[{}] {}", self.kind(), self.mensaje_usuario()),
            },
        }
        ErrorPayload {
            kind: self.kind().into(),
            message: self.mensaje_usuario(),
            detail: self.detail(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for AppError {}

impl From<rsfbclient::FbError> for AppError {
    fn from(e: rsfbclient::FbError) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<r2d2::Error> for AppError {
    fn from(e: r2d2::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Generic(e.to_string())
    }
}

/// Resultado de comandos Tauri: Ok(Value) | Err(ErrorPayload)
pub type CmdResult<T> = Result<T, ErrorPayload>;

/// Convierte AppError en un Err serializable para `#[tauri::command]`
pub fn to_cmd_err(e: AppError) -> ErrorPayload {
    e.to_payload()
}
