//! Consultas de depósitos y transferencias pendientes.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Deposit, DepositInput};

const SELECT_DEPOSIT: &str = "
    SELECT d.id, d.contact_id, ct.name, d.title, d.amount, d.currency_code, d.created_date,
           d.due_date, d.account_id, a.name, d.destination_institution, d.destination_details,
           d.payment_method, d.status, d.completed_at, d.transaction_id, d.receivable_id,
           d.notes, d.created_at
    FROM deposits d
    LEFT JOIN contacts ct ON ct.id = d.contact_id
    LEFT JOIN accounts a  ON a.id = d.account_id";

fn map_deposit(row: &Row<'_>) -> rusqlite::Result<Deposit> {
    Ok(Deposit {
        id: row.get(0)?,
        contact_id: row.get(1)?,
        contact_name: row.get(2)?,
        title: row.get(3)?,
        amount: row.get(4)?,
        currency_code: row.get(5)?,
        created_date: row.get(6)?,
        due_date: row.get(7)?,
        account_id: row.get(8)?,
        account_name: row.get(9)?,
        destination_institution: row.get(10)?,
        destination_details: row.get(11)?,
        payment_method: row.get(12)?,
        status: row.get(13)?,
        completed_at: row.get(14)?,
        transaction_id: row.get(15)?,
        receivable_id: row.get(16)?,
        notes: row.get(17)?,
        created_at: row.get(18)?,
    })
}

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Deposit>> {
    let mut sql = format!("{SELECT_DEPOSIT} WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(s) = status.filter(|s| !s.is_empty()) {
        sql.push_str(" AND d.status = ?1");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY CASE d.status WHEN 'overdue' THEN 0 WHEN 'pending' THEN 1 WHEN 'scheduled' THEN 2 WHEN 'completed' THEN 3 ELSE 4 END, d.due_date IS NULL, d.due_date ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_deposit)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Deposit> {
    let sql = format!("{SELECT_DEPOSIT} WHERE d.id = ?1");
    conn.query_row(&sql, [id], map_deposit)
        .map_err(|_| AppError::not_found("El depósito no existe."))
}

pub fn insert(conn: &Connection, id: &str, input: &DepositInput, created_at: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO deposits (id, contact_id, title, amount, currency_code, created_date, due_date,
            account_id, destination_institution, destination_details, payment_method, status,
            completed_at, transaction_id, receivable_id, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'pending', NULL, NULL, NULL, ?12, ?13)",
        params![
            id,
            input.contact_id,
            input.title,
            input.amount,
            input.currency_code,
            input.created_date,
            input.due_date,
            input.account_id,
            input.destination_institution,
            input.destination_details,
            input.payment_method,
            input.notes,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &DepositInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE deposits SET contact_id = ?2, title = ?3, amount = ?4, currency_code = ?5,
                created_date = ?6, due_date = ?7, account_id = ?8, destination_institution = ?9,
                destination_details = ?10, payment_method = ?11, notes = ?12
             WHERE id = ?1",
            params![
                id,
                input.contact_id,
                input.title,
                input.amount,
                input.currency_code,
                input.created_date,
                input.due_date,
                input.account_id,
                input.destination_institution,
                input.destination_details,
                input.payment_method,
                input.notes
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El depósito no existe."));
    }
    Ok(())
}

pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE deposits SET status = ?2 WHERE id = ?1",
            params![id, status],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El depósito no existe."));
    }
    Ok(())
}

pub fn complete(
    conn: &Connection,
    id: &str,
    status: &str,
    completed_at: Option<&str>,
    transaction_id: Option<&str>,
    receivable_id: Option<&str>,
) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE deposits SET status = ?2, completed_at = ?3, transaction_id = ?4, receivable_id = ?5
             WHERE id = ?1",
            params![id, status, completed_at, transaction_id, receivable_id],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El depósito no existe."));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM deposits WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}
