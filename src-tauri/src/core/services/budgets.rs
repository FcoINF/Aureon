//! Servicio de presupuestos.
//!
//! Un presupuesto es sólo un límite mensual por categoría. Nunca genera
//! movimientos financieros.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::AppResult;
use crate::core::models::{Budget, BudgetInput};
use crate::core::repos;
use crate::core::services::{self, new_id};

pub fn list(conn: &Connection, period: (&str, &str)) -> AppResult<Vec<Budget>> {
    let mut budgets = repos::budgets::list(conn, period)?;
    for budget in budgets.iter_mut() {
        budget.spent = budget.period_spent;
    }
    Ok(budgets)
}

pub fn upsert(conn: &Connection, input: &BudgetInput) -> AppResult<Budget> {
    let category_id = services::ensure_category(conn, Some(&input.category_id), Some("expense"))?
        .ok_or_else(|| crate::core::error::AppError::validation("Elige una categoría."))?;
    let currency = services::ensure_currency(conn, &input.currency_code)?;
    let amount = services::non_negative(input.amount)?;
    let start_date = match input.start_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => dates::format_date(dates::parse_date(d)?),
        None => dates::today_str(),
    };
    let end_date = match input.end_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => None,
    };
    let normalized = BudgetInput {
        category_id,
        currency_code: currency,
        amount,
        start_date: Some(start_date),
        end_date,
    };
    let id = new_id();
    let saved_id = repos::budgets::upsert(conn, &id, &normalized, &dates::now_str())?;
    let (start, end) = dates::period_range("month", 0)?;
    repos::budgets::get(conn, &saved_id, (&start, &end))
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repos::budgets::delete(conn, id)
}
