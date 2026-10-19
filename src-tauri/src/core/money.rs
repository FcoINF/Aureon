//! Manejo de dinero.
//!
//! Reglas:
//! * Los importes se almacenan como enteros en la unidad mínima de la moneda
//!   (CLP: pesos, exponente 0; USD/EUR: centavos, exponente 2).
//! * Nunca se usan `f32` ni `f64` para importes.
//! * El formato de entrada y salida sigue la convención chilena:
//!   punto para los miles y coma para los decimales.

use crate::core::error::{AppError, AppResult};

/// Convierte el texto escrito por el usuario a unidades mínimas de la moneda.
pub fn parse_to_minor(raw: &str, exponent: u8) -> AppResult<i64> {
    let mut cleaned = String::new();
    let mut negative = false;
    for ch in raw.chars() {
        if ch == '-' {
            negative = true;
        } else if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            cleaned.push(ch);
        }
    }
    if cleaned.is_empty() {
        return Err(AppError::validation("Ingresa un monto válido."));
    }
    if cleaned.matches('.').count() > 1 && cleaned.matches(',').count() > 1 {
        return Err(AppError::validation("El monto tiene demasiados separadores."));
    }
    if cleaned.matches('.').count() > 1 || cleaned.matches(',').count() > 1 {
        return Err(AppError::validation("El monto no tiene un formato válido."));
    }

    let (head, frac): (String, Option<String>) = if let Some(pos) = cleaned.rfind(',') {
        // La coma es siempre el separador decimal (convención chilena).
        let head = cleaned[..pos].replace('.', "");
        let frac = cleaned[pos + 1..].to_string();
        (head, Some(frac))
    } else if let Some(pos) = cleaned.rfind('.') {
        let tail = &cleaned[pos + 1..];
        let is_decimal = if exponent == 0 {
            // En pesos el punto sólo separa miles, salvo que el usuario haya
            // escrito centavos con dos dígitos.
            tail.len() == 2 && tail.chars().any(|c| c != '0')
        } else {
            tail.len() == exponent as usize
        };
        if is_decimal {
            (cleaned[..pos].replace(',', ""), Some(tail.to_string()))
        } else {
            // El punto separa miles: se elimina por completo.
            (cleaned.replace('.', ""), None)
        }
    } else {
        (cleaned.clone(), None)
    };

    if head.is_empty() && frac.is_none() {
        return Err(AppError::validation("Ingresa un monto válido."));
    }
    if !head.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::validation("El monto no tiene un formato válido."));
    }
    if let Some(f) = &frac {
        if !f.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::validation("El monto no tiene un formato válido."));
        }
    }

    let base: i128 = if head.is_empty() {
        0
    } else {
        head.parse::<i128>()
            .map_err(|_| AppError::validation("El monto es demasiado grande."))?
    };

    let mut minor: i128 = base * 10i128.pow(exponent as u32);
    if let Some(f) = &frac {
        let digits: String = f.chars().take(exponent as usize).collect();
        let value: i128 = if digits.is_empty() {
            0
        } else {
            digits.parse::<i128>().unwrap_or(0)
        };
        minor += value;
        if f.len() > exponent as usize {
            // Redondeo al entero más cercano (importes con exceso de decimales).
            let next = f.as_bytes()[exponent as usize] - b'0';
            if next >= 5 {
                minor += 1;
            }
        }
    }

    if negative {
        minor = -minor;
    }
    if minor > i64::MAX as i128 || minor < i64::MIN as i128 {
        return Err(AppError::validation("El monto es demasiado grande."));
    }
    Ok(minor as i64)
}

/// Formatea unidades mínimas como texto legible en convención chilena.
pub fn format_minor(amount: i64, exponent: u8) -> String {
    let negative = amount < 0;
    let abs = amount.unsigned_abs() as i128;
    let factor = 10i128.pow(exponent as u32);
    let integer = abs / factor;
    let fraction = abs % factor;

    let digits = integer.to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    out.push_str(&grouped);
    if exponent > 0 {
        out.push(',');
        let frac = format!("{:0width$}", fraction, width = exponent as usize);
        out.push_str(&frac);
    }
    out
}

/// Formatea con el símbolo de la moneda: `$1.234` / `US$1.234,56`.
pub fn format_currency(amount: i64, exponent: u8, symbol: &str) -> String {
    format!("{}{}", symbol, format_minor(amount, exponent))
}

/// Devuelve el exponente (decimales) de una moneda registrada en la base.
pub fn exponent_of(currencies: &[(String, u8)], code: &str) -> u8 {
    currencies
        .iter()
        .find(|(c, _)| c == code)
        .map(|(_, e)| *e)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsea_pesos_chilenos() {
        assert_eq!(parse_to_minor("1234", 0).unwrap(), 1234);
        assert_eq!(parse_to_minor("1.234", 0).unwrap(), 1234);
        assert_eq!(parse_to_minor("$19.990", 0).unwrap(), 19_990);
        assert_eq!(parse_to_minor("45.000", 0).unwrap(), 45_000);
    }

    #[test]
    fn parsea_decimales_y_miles() {
        assert_eq!(parse_to_minor("19,99", 2).unwrap(), 1999);
        assert_eq!(parse_to_minor("1.234,56", 2).unwrap(), 123_456);
        assert_eq!(parse_to_minor("19.99", 2).unwrap(), 1999);
        assert_eq!(parse_to_minor("1.234", 2).unwrap(), 123_400);
    }

    #[test]
    fn rechaza_entradas_invalidas() {
        assert!(parse_to_minor("", 0).is_err());
        assert!(parse_to_minor("abc", 0).is_err());
        assert!(parse_to_minor("1.2.3", 0).is_err());
    }

    #[test]
    fn formatea_convencion_chilena() {
        assert_eq!(format_minor(1_234_567, 0), "1.234.567");
        assert_eq!(format_minor(-1_234, 0), "-1.234");
        assert_eq!(format_minor(123_456, 2), "1.234,56");
        assert_eq!(format_minor(0, 0), "0");
    }
}
