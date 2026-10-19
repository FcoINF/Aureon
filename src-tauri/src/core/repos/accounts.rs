//! Consultas de cuentas financieras.

use rusqlite::{params, Connection};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Account, AccountInput};
use crate::core::repos::to_bool;

const SELECT_ACCOUNT: &str = "
    SELECT
        a.id, a.name, a.type, a.currency_code, a.initial_balance, a.initial_balance_date,
        a.archived, a.description, a.created_at,
        a.initial_balance + COALESCE(SUM(
            CASE
                WHEN t.voided = 1 THEN 0
                WHEN t.type = 'income'  AND t.account_id = a.id THEN t.amount
                WHEN t.type = 'expense' AND t.account_id = a.id THEN -t.amount
                WHEN t.type = 'refund'  AND t.account_id = a.id THEN t.amount
                WHEN t.type = 'transfer' AND t.account_id = a.id THEN -t.amount
                WHEN t.type = 'transfer' AND t.transfer_account_id = a.id THEN t.amount
                ELSE 0
            END
        ), 0) AS balance
    FROM accounts a
    LEFT JOIN transactions t
        ON t.voided = 0 AND (t.account_id = a.id OR t.transfer_account_id = a.id)";

fn map_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        name: row.get(1)?,
        account_type: row.get(2)?,
        currency_code: row.get(3)?,
        initial_balance: row.get(4)?,
        initial_balance_date: row.get(5)?,
        archived: to_bool(row.get(6)?),
        description: row.get(7)?,
        created_at: row.get(8)?,
        balance: row.get(9)?,
    })
}

pub fn list(conn: &Connection, include_archived: bool) -> AppResult<Vec<Account>> {
    let sql = if include_archived {
        format!("{SELECT_ACCOUNT} GROUP BY a.id ORDER BY a.archived ASC, a.name COLLATE NOCASE ASC")
    } else {
        format!("{SELECT_ACCOUNT} WHERE a.archived = 0 GROUP BY a.id ORDER BY a.name COLLATE NOCASE ASC")
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_account)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Account> {
    let sql = format!("{SELECT_ACCOUNT} WHERE a.id = ?1 GROUP BY a.id");
    conn.query_row(&sql, [id], map_account)
        .map_err(|_| AppError::not_found("La cuenta no existe."))
}

pub fn insert(conn: &Connection, id: &str, input: &AccountInput, created_at: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO accounts (id, name, type, currency_code, initial_balance, initial_balance_date, archived, description, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            input.name,
            input.account_type,
            input.currency_code,
            input.initial_balance,
            input.initial_balance_date,
            if input.archived.unwrap_or(false) { 1 } else { 0 },
            input.description,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &AccountInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE accounts SET name = ?2, type = ?3, currency_code = ?4, initial_balance = ?5,
                initial_balance_date = ?6, archived = ?7, description = ?8 WHERE id = ?1",
            params![
                id,
                input.name,
                input.account_type,
                input.currency_code,
                input.initial_balance,
                input.initial_balance_date,
                if input.archived.unwrap_or(false) { 1 } else { 0 },
                input.description
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta no existe."));
    }
    Ok(())
}

pub fn set_archived(conn: &Connection, id: &str, archived: bool) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE accounts SET archived = ?2 WHERE id = ?1",
            params![id, if archived { 1 } else { 0 }],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta no existe."));
    }
    Ok(())
}

/// Elimina la cuenta sólo si no tiene movimientos asociados.
pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let used: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE account_id = ?1 OR transfer_account_id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    if used > 0 {
        return Err(AppError::conflict(
            "La cuenta tiene movimientos asociados. Archívala en lugar de eliminarla.",
        ));
    }
    conn.execute("DELETE FROM accounts WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn exists(conn: &Connection, id: &str) -> AppResult<bool> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(AppError::from_sqlite)?;
    Ok(count > 0)
}
