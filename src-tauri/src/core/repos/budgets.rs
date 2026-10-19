//! Consultas de presupuestos y ajustes.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Budget, BudgetInput};

const SELECT_BUDGET: &str = "
    SELECT b.id, b.category_id, c.name, b.currency_code, b.amount, b.start_date, b.end_date,
           COALESCE((SELECT SUM(t.amount) FROM transactions t
                      WHERE t.category_id = b.category_id AND t.voided = 0
                        AND t.type = 'expense' AND t.date >= ?1 AND t.date <= ?2), 0) AS period_spent
    FROM budgets b
    JOIN categories c ON c.id = b.category_id";

fn map_budget(row: &Row<'_>) -> rusqlite::Result<Budget> {
    Ok(Budget {
        id: row.get(0)?,
        category_id: row.get(1)?,
        category_name: row.get(2)?,
        currency_code: row.get(3)?,
        amount: row.get(4)?,
        start_date: row.get(5)?,
        end_date: row.get(6)?,
        spent: 0,
        period_spent: row.get(7)?,
    })
}

pub fn list(conn: &Connection, period: (&str, &str)) -> AppResult<Vec<Budget>> {
    let sql = format!("{SELECT_BUDGET} ORDER BY c.name COLLATE NOCASE ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![period.0, period.1], map_budget)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str, period: (&str, &str)) -> AppResult<Budget> {
    let sql = format!("{SELECT_BUDGET} WHERE b.id = ?3");
    conn.query_row(&sql, params![period.0, period.1, id], map_budget)
        .map_err(|_| AppError::not_found("El presupuesto no existe."))
}

pub fn upsert(conn: &Connection, id: &str, input: &BudgetInput, created_at: &str) -> AppResult<String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM budgets WHERE category_id = ?1 AND currency_code = ?2",
            params![input.category_id, input.currency_code],
            |row| row.get(0),
        )
        .ok();
    match existing {
        Some(existing_id) => {
            conn.execute(
                "UPDATE budgets SET amount = ?2, start_date = ?3, end_date = ?4 WHERE id = ?1",
                params![
                    existing_id,
                    input.amount,
                    input.start_date.clone().unwrap_or_else(|| created_at.chars().take(10).collect()),
                    input.end_date
                ],
            )
            .map_err(AppError::from_sqlite)?;
            Ok(existing_id)
        }
        None => {
            conn.execute(
                "INSERT INTO budgets (id, category_id, currency_code, amount, start_date, end_date, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id,
                    input.category_id,
                    input.currency_code,
                    input.amount,
                    input.start_date.clone().unwrap_or_else(|| created_at.chars().take(10).collect()),
                    input.end_date,
                    created_at
                ],
            )
            .map_err(AppError::from_sqlite)?;
            Ok(id.to_string())
        }
    }
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM budgets WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}
