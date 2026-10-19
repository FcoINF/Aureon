//! Capa de acceso a datos.
//!
//! Este módulo contiene únicamente consultas SQL parametrizadas. Toda la
//! lógica de negocio vive en `core::services`.

pub mod accounts;
pub mod budgets;
pub mod calendar;
pub mod categories;
pub mod contacts;
pub mod deposits;
pub mod maintenance;
pub mod payables;
pub mod purchases;
pub mod receivables;
pub mod reports;
pub mod settings;
pub mod subscriptions;
pub mod transactions;
pub mod wishlist;

use rusqlite::{Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::Currency;

/// Lista las monedas registradas.
pub fn list_currencies(conn: &Connection) -> AppResult<Vec<Currency>> {
    let mut stmt = conn.prepare("SELECT code, name, symbol, exponent FROM currencies ORDER BY code")?;
    let rows = stmt.query_map([], |row| {
        Ok(Currency {
            code: row.get(0)?,
            name: row.get(1)?,
            symbol: row.get(2)?,
            exponent: row.get::<_, i64>(3)? as u8,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

/// Devuelve el exponente de una moneda, o 0 si no está registrada.
pub fn currency_exponent(conn: &Connection, code: &str) -> AppResult<u8> {
    let exponent: Option<i64> = conn
        .query_row(
            "SELECT exponent FROM currencies WHERE code = ?1",
            [code],
            |row| row.get(0),
        )
        .ok();
    Ok(exponent.unwrap_or(0) as u8)
}

/// Comprueba que la moneda exista.
pub fn ensure_currency(conn: &Connection, code: &str) -> AppResult<()> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM currencies WHERE code = ?1",
        [code],
        |row| row.get(0),
    )?;
    if exists == 0 {
        return Err(AppError::validation("La moneda indicada no está disponible."));
    }
    Ok(())
}

/// Convierte una columna nullable de texto en `Option<String>`.
pub fn opt_text(row: &Row<'_>, idx: usize) -> rusqlite::Result<Option<String>> {
    row.get::<_, Option<String>>(idx)
}

/// Convierte un entero de SQLite en `bool`.
pub fn to_bool(value: i64) -> bool {
    value != 0
}
