//! validators.rs — Validadores de datos (puerto de core/validators.py)

use super::error::AppError;

/// Patrones XSS/SQLi (espejo de `_XSS_PATTERNS` en validators.py)
const XSS_PATTERNS: [&str; 13] = [
    "<script",
    "javascript:",
    "on\\w+\\s*=", // event handlers
    "<iframe",
    "<object",
    "<embed",
    "<form",
    "eval\\s*\\(",
    "document\\.",
    "window\\.",
    "union\\s+select",
    "drop\\s+table",
    ";\\s*--", // SQL comment injection
];

/// Valida que el texto no contenga patrones peligrosos (XSS/SQLi)
pub fn validate_no_xss(value: &str, max_length: usize) -> Result<String, AppError> {
    let mut v = value.to_string();
    if v.is_empty() {
        return Ok(v);
    }
    if v.len() > max_length {
        v.truncate(max_length);
    }
    let lower = v.to_lowercase();
    for pattern in XSS_PATTERNS {
        // Regex básico: buscamos el patrón en minúsculas
        let found = match pattern {
            "on\\w+\\s*=" | "eval\\s*\\(" | "union\\s+select" | ";\\s*--" => {
                regex_light(pattern, &lower)
            }
            _ => lower.contains(pattern),
        };
        if found {
            return Err(AppError::Sanitization(format!(
                "Patrón peligroso detectado: {pattern}"
            )));
        }
    }
    Ok(v.trim().to_string())
}

/// Mini-motor de regex para los pocos patrones que lo requieren
fn regex_light(pattern: &str, text: &str) -> bool {
    match pattern {
        "on\\w+\\s*=" => {
            // detecta "onclick=", "on error =", "onload=" etc. (los espacios son opcionales)
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;
            while i + 1 < chars.len() {
                if chars[i] == 'o' && chars[i + 1] == 'n' {
                    let mut j = i + 2;
                    // \w+ = letras/dígitos/underscore
                    while j < chars.len()
                        && (chars[j].is_alphanumeric() || chars[j] == '_')
                    {
                        j += 1;
                    }
                    // \s* = espacios opcionales
                    while j < chars.len() && chars[j].is_whitespace() {
                        j += 1;
                    }
                    if j < chars.len() && chars[j] == '=' {
                        return true;
                    }
                    i += 1;
                } else {
                    i += 1;
                }
            }
            false
        }
        "eval\\s*\\(" => {
            let s = text.replace(" ", "").replace("\t", "");
            s.contains("eval(")
        }
        "union\\s+select" => {
            let s = text.replace(" ", "").replace("\t", "");
            s.contains("unionselect")
        }
        ";\\s*--" => {
            // detecta ";--" o "; --" (comentario SQL)
            let s = text.replace(" ", "").replace("\t", "");
            s.contains(";--")
        }
        _ => false,
    }
}

/// Valida la fortaleza de una contraseña (espejo de SecurityManager.validate_password_strength)
pub fn validate_password_strength(password: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if password.len() < 8 {
        errors.push("La contraseña debe tener al menos 8 caracteres".into());
    }
    if password.len() > 128 {
        errors.push("La contraseña no puede tener más de 128 caracteres".into());
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Debe contener al menos una letra mayúscula".into());
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        errors.push("Debe contener al menos una letra minúscula".into());
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push("Debe contener al menos un número".into());
    }
    if !password.chars().any(|c| "!@#$%^&*(),.?\":{}|<>".contains(c)) {
        errors.push("Debe contener al menos un carácter especial (!@#$%^&*(),.?\":{}|<>)".into());
    }
    errors
}

/// Limita y limpia texto de entrada (espejo de `sanitizar`)
pub fn sanitizar(texto: &str, max_len: usize) -> String {
    texto.trim().chars().take(max_len).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xss_detection() {
        assert!(validate_no_xss("<script>alert(1)</script>", 500).is_err());
        assert!(validate_no_xss("javascript:alert(1)", 500).is_err());
        assert!(validate_no_xss("onclick=alert(1)", 500).is_err());
        assert!(validate_no_xss("SELECT * FROM usuarios; --", 500).is_err());
        assert!(validate_no_xss("texto normal", 500).is_ok());
    }

    #[test]
    fn password_strength() {
        assert!(!validate_password_strength("short1A").is_empty()); // 7 chars
        assert_eq!(validate_password_strength("Passw0rd!").len(), 0); // válida
        assert!(!validate_password_strength("alllower1!").is_empty()); // sin mayúscula
    }
}
