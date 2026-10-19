//! Fechas y períodos.
//!
//! Todas las fechas se guardan como texto `AAAA-MM-DD` (sin hora) y las marcas
//! de tiempo como `AAAA-MM-DDTHH:MM:SSZ`. La semana chilena empieza el lunes.

use crate::core::error::{AppError, AppResult};
use chrono::{Datelike, Duration, Local, NaiveDate};

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn today_str() -> String {
    today().format("%Y-%m-%d").to_string()
}

pub fn now_str() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn parse_date(raw: &str) -> AppResult<NaiveDate> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Ingresa una fecha."));
    }
    let normalized = match trimmed.split('/').count() {
        3 => {
            let parts: Vec<&str> = trimmed.split('/').collect();
            format!("{}-{}-{}", parts[2], parts[1], parts[0])
        }
        _ => trimmed.to_string(),
    };
    NaiveDate::parse_from_str(&normalized, "%Y-%m-%d")
        .map_err(|_| AppError::validation("La fecha no es válida."))
}

pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Frecuencia de pago soportada por compras y suscripciones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    Weekly,
    Biweekly,
    Monthly,
    Bimonthly,
    Quarterly,
    Semiannual,
    Yearly,
    Custom(u32),
}

impl Frequency {
    pub fn from_str(value: &str, custom_days: Option<i64>) -> AppResult<Self> {
        Ok(match value {
            "weekly" => Frequency::Weekly,
            "biweekly" => Frequency::Biweekly,
            "monthly" => Frequency::Monthly,
            "bimonthly" => Frequency::Bimonthly,
            "quarterly" => Frequency::Quarterly,
            "semiannual" => Frequency::Semiannual,
            "yearly" => Frequency::Yearly,
            "custom" => {
                let days = custom_days.unwrap_or(0);
                if days <= 0 {
                    return Err(AppError::validation("Indica cada cuántos días se repite el pago."));
                }
                Frequency::Custom(days as u32)
            }
            _ => return Err(AppError::validation("La frecuencia indicada no es válida.")),
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Frequency::Weekly => "weekly",
            Frequency::Biweekly => "biweekly",
            Frequency::Monthly => "monthly",
            Frequency::Bimonthly => "bimonthly",
            Frequency::Quarterly => "quarterly",
            Frequency::Semiannual => "semiannual",
            Frequency::Yearly => "yearly",
            Frequency::Custom(_) => "custom",
        }
    }

    /// Avanza una fecha según la frecuencia. Los meses sin el día exacto
    /// caen al último día del mes (31 de enero + 1 mes = 28/29 de febrero).
    pub fn advance(&self, date: NaiveDate) -> NaiveDate {
        match self {
            Frequency::Weekly => date + Duration::weeks(1),
            Frequency::Biweekly => date + Duration::weeks(2),
            Frequency::Monthly => add_months(date, 1),
            Frequency::Bimonthly => add_months(date, 2),
            Frequency::Quarterly => add_months(date, 3),
            Frequency::Semiannual => add_months(date, 6),
            Frequency::Yearly => add_months(date, 12),
            Frequency::Custom(days) => date + Duration::days(*days as i64),
        }
    }

    /// Veces por año, para estimar costos mensuales y anuales.
    pub fn occurrences_per_year(&self) -> f64 {
        let (num, den) = self.occurrences_ratio();
        num as f64 / den as f64
    }

    /// Cobros por año como fracción exacta (numerador, denominador).
    /// Se usa aritmética entera para que las estimaciones no acumulen
    /// errores de redondeo.
    pub fn occurrences_ratio(&self) -> (i64, i64) {
        match self {
            Frequency::Weekly => (52, 1),
            Frequency::Biweekly => (26, 1),
            Frequency::Monthly => (12, 1),
            Frequency::Bimonthly => (6, 1),
            Frequency::Quarterly => (4, 1),
            Frequency::Semiannual => (2, 1),
            Frequency::Yearly => (1, 1),
            Frequency::Custom(days) => (365, *days as i64),
        }
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let total = date.year() * 12 + date.month0() as i32 + months;
    let year = total.div_euclid(12);
    let month = (total.rem_euclid(12) + 1) as u32;
    let day = date.day().min(days_in_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(date)
}

/// Rango inclusivo de fechas de un período predefinido.
pub fn period_range(period: &str, offset: i32) -> AppResult<(String, String)> {
    let now = today();
    let (start, end) = match period {
        "today" => (now, now),
        "yesterday" => (now - Duration::days(1), now - Duration::days(1)),
        "week" => {
            let start = now - Duration::days(now.weekday().num_days_from_monday() as i64);
            (start, start + Duration::days(6))
        }
        "last_week" => {
            let this_monday =
                now - Duration::days(now.weekday().num_days_from_monday() as i64);
            let start = this_monday - Duration::weeks(1);
            (start, start + Duration::days(6))
        }
        "month" => {
            let start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
            let end = add_months(start, 1) - Duration::days(1);
            (start, end)
        }
        "last_month" => {
            let this_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
            let start = add_months(this_month, -1);
            let end = this_month - Duration::days(1);
            (start, end)
        }
        "year" => {
            let start = NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap_or(now);
            let end = NaiveDate::from_ymd_opt(now.year(), 12, 31).unwrap_or(now);
            (start, end)
        }
        "last_year" => {
            let start = NaiveDate::from_ymd_opt(now.year() - 1, 1, 1).unwrap_or(now);
            let end = NaiveDate::from_ymd_opt(now.year() - 1, 12, 31).unwrap_or(now);
            (start, end)
        }
        "last_30" => (now - Duration::days(29), now),
        "next_30" => (now, now + Duration::days(29)),
        "next_90" => (now, now + Duration::days(89)),
        other => return Err(AppError::validation(format!("Período desconocido: {other}"))),
    };
    let _ = offset;
    Ok((format_date(start), format_date(end)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avanza_meses_manejando_fin_de_mes() {
        let d = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        assert_eq!(Frequency::Monthly.advance(d).to_string(), "2024-02-29");
        assert_eq!(Frequency::Quarterly.advance(d).to_string(), "2024-04-30");
    }

    #[test]
    fn rango_de_mes_actual() {
        let (start, end) = period_range("month", 0).unwrap();
        assert!(start.ends_with("-01"));
        assert!(end.len() == 10);
    }

    #[test]
    fn rango_de_semana_empieza_en_lunes() {
        let (start, _) = period_range("week", 0).unwrap();
        let date = NaiveDate::parse_from_str(&start, "%Y-%m-%d").unwrap();
        assert_eq!(date.weekday().num_days_from_monday(), 0);
    }
}
