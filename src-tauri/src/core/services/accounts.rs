//! Servicio de cuentas.

use rusqlite::Connection;

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Account, AccountInput};
use crate::core::repos;
use crate::core::services::{self, new_id};
use crate::core::dates;

const ACCOUNT_TYPES: [&str; 6] = [
    "cash",
    "checking",
    "savings",
    "rut",
    "credit_card",
    "other",
];

pub fn list(conn: &Connection, include_archived: bool) -> AppResult<Vec<Account>> {
    repos::accounts::list(conn, include_archived)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Account> {
    repos::accounts::get(conn, id)
}

fn validate(conn: &Connection, input: &AccountInput) -> AppResult<(String, String, String, String)> {
    let name = services::text(Some(&input.name), "el nombre de la cuenta")?;
    if !ACCOUNT_TYPES.contains(&input.account_type.as_str()) {
        return Err(AppError::validation("El tipo de cuenta no es válido."));
    }
    let currency = services::ensure_currency(conn, &input.currency_code)?;
    let date = dates::parse_date(&input.initial_balance_date)?;
    services::non_negative(input.initial_balance).map_err(|_| {
        AppError::validation("El saldo inicial no puede ser negativo.")
    })?;
    Ok((name, input.account_type.clone(), currency, dates::format_date(date)))
}

pub fn create(conn: &Connection, input: &AccountInput) -> AppResult<Account> {
    let (name, account_type, currency, date) = validate(conn, input)?;
    let id = new_id();
    let normalized = AccountInput {
        name,
        account_type,
        currency_code: currency,
        initial_balance: input.initial_balance,
        initial_balance_date: date,
        archived: input.archived,
        description: services::optional_text(input.description.clone()),
    };
    repos::accounts::insert(conn, &id, &normalized, &dates::now_str())?;
    repos::accounts::get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &AccountInput) -> AppResult<Account> {
    let (name, account_type, currency, date) = validate(conn, input)?;
    let normalized = AccountInput {
        name,
        account_type,
        currency_code: currency,
        initial_balance: input.initial_balance,
        initial_balance_date: date,
        archived: input.archived,
        description: services::optional_text(input.description.clone()),
    };
    repos::accounts::update(conn, id, &normalized)?;
    repos::accounts::get(conn, id)
}

pub fn set_archived(conn: &Connection, id: &str, archived: bool) -> AppResult<Account> {
    repos::accounts::set_archived(conn, id, archived)?;
    repos::accounts::get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repos::accounts::delete(conn, id)
}
