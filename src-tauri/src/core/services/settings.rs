//! Servicio de ajustes de la aplicación.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use crate::core::repos;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_currency: String,
    pub date_format: String,
    pub theme: String,
    pub export_separator: String,
    pub backup_folder: Option<String>,
    pub week_starts_on_monday: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            default_currency: "CLP".to_string(),
            date_format: "DD/MM/AAAA".to_string(),
            theme: "light".to_string(),
            export_separator: ";".to_string(),
            backup_folder: None,
            week_starts_on_monday: true,
        }
    }
}

const KEY_CURRENCY: &str = "default_currency";
const KEY_DATE_FORMAT: &str = "date_format";
const KEY_THEME: &str = "theme";
const KEY_EXPORT_SEPARATOR: &str = "export_separator";
const KEY_BACKUP_FOLDER: &str = "backup_folder";
const KEY_WEEK_START: &str = "week_starts_on_monday";

pub fn load(conn: &Connection) -> AppResult<AppSettings> {
    let stored = repos::settings::all_settings(conn)?;
    let mut settings = AppSettings::default();
    for (key, value) in stored {
        match key.as_str() {
            KEY_CURRENCY => settings.default_currency = value,
            KEY_DATE_FORMAT => settings.date_format = value,
            KEY_THEME => settings.theme = value,
            KEY_EXPORT_SEPARATOR => settings.export_separator = value,
            KEY_BACKUP_FOLDER => {
                settings.backup_folder = if value.trim().is_empty() { None } else { Some(value) }
            }
            KEY_WEEK_START => settings.week_starts_on_monday = value != "false",
            _ => {}
        }
    }
    Ok(settings)
}

pub fn save(conn: &Connection, settings: &AppSettings) -> AppResult<AppSettings> {
    repos::ensure_currency(conn, &settings.default_currency)?;
    if !["light", "dark"].contains(&settings.theme.as_str()) {
        return Err(AppError::validation("El tema indicado no es válido."));
    }
    if ![";", ",", "\t"].contains(&settings.export_separator.as_str()) {
        return Err(AppError::validation("El separador de exportación no es válido."));
    }
    repos::settings::set_setting(conn, KEY_CURRENCY, &settings.default_currency)?;
    repos::settings::set_setting(conn, KEY_DATE_FORMAT, &settings.date_format)?;
    repos::settings::set_setting(conn, KEY_THEME, &settings.theme)?;
    repos::settings::set_setting(conn, KEY_EXPORT_SEPARATOR, &settings.export_separator)?;
    repos::settings::set_setting(
        conn,
        KEY_BACKUP_FOLDER,
        settings.backup_folder.as_deref().unwrap_or(""),
    )?;
    repos::settings::set_setting(
        conn,
        KEY_WEEK_START,
        if settings.week_starts_on_monday { "true" } else { "false" },
    )?;
    load(conn)
}

/// Moneda por defecto, usada por los formularios.
pub fn default_currency(conn: &Connection) -> String {
    load(conn)
        .map(|s| s.default_currency)
        .unwrap_or_else(|_| "CLP".to_string())
}
