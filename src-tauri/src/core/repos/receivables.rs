//! Consultas de cuentas por cobrar.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Receivable, ReceivableInput, ReceivablePayment};

const SELECT_RECEIVABLE: &str = "
    SELECT r.id, r.contact_id, ct.name, r.title, r.kind, r.total_amount, r.received_amount,
           (r.total_amount - r.received_amount) AS pending_amount, r.currency_code,
           r.origin_date, r.due_date, r.status, r.notes, r.created_at,
           (SELECT COUNT(*) FROM receivable_payments rp WHERE rp.receivable_id = r.id) AS payment_count
    FROM receivables r
    JOIN contacts ct ON ct.id = r.contact_id";

fn map_receivable(row: &Row<'_>) -> rusqlite::Result<Receivable> {
    Ok(Receivable {
        id: row.get(0)?,
        contact_id: row.get(1)?,
        contact_name: row.get(2)?,
        title: row.get(3)?,
        kind: row.get(4)?,
        total_amount: row.get(5)?,
        received_amount: row.get(6)?,
        pending_amount: row.get(7)?,
        currency_code: row.get(8)?,
        origin_date: row.get(9)?,
        due_date: row.get(10)?,
        status: row.get(11)?,
        notes: row.get(12)?,
        created_at: row.get(13)?,
        payment_count: row.get(14)?,
    })
}

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Receivable>> {
    let mut sql = format!("{SELECT_RECEIVABLE} WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(s) = status.filter(|s| !s.is_empty()) {
        sql.push_str(" AND r.status = ?1");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY CASE r.status WHEN 'partial' THEN 0 WHEN 'overdue' THEN 1 WHEN 'pending' THEN 2 ELSE 3 END, r.due_date IS NULL, r.due_date ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_receivable)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Receivable> {
    let sql = format!("{SELECT_RECEIVABLE} WHERE r.id = ?1");
    conn.query_row(&sql, [id], map_receivable)
        .map_err(|_| AppError::not_found("La cuenta por cobrar no existe."))
}

pub fn insert(
    conn: &Connection,
    id: &str,
    input: &ReceivableInput,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO receivables (id, contact_id, title, kind, total_amount, received_amount,
            currency_code, origin_date, due_date, status, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, 'pending', ?9, ?10)",
        params![
            id,
            input.contact_id,
            input.title,
            input.kind.clone().unwrap_or_else(|| "debt".to_string()),
            input.total_amount,
            input.currency_code,
            input.origin_date,
            input.due_date,
            input.notes,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &ReceivableInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE receivables SET contact_id = ?2, title = ?3, kind = ?4, total_amount = ?5,
                currency_code = ?6, origin_date = ?7, due_date = ?8, notes = ?9 WHERE id = ?1",
            params![
                id,
                input.contact_id,
                input.title,
                input.kind.clone().unwrap_or_else(|| "debt".to_string()),
                input.total_amount,
                input.currency_code,
                input.origin_date,
                input.due_date,
                input.notes
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta por cobrar no existe."));
    }
    Ok(())
}

pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE receivables SET status = ?2 WHERE id = ?1",
            params![id, status],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta por cobrar no existe."));
    }
    Ok(())
}

/// Actualiza el total recibido y el estado tras registrar un cobro.
pub fn apply_payment(
    conn: &Connection,
    id: &str,
    received_amount: i64,
    status: &str,
) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE receivables SET received_amount = ?2, status = ?3 WHERE id = ?1",
            params![id, received_amount, status],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta por cobrar no existe."));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM receivables WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}

const SELECT_RECEIVABLE_PAYMENT: &str = "
    SELECT rp.id, rp.receivable_id, rp.amount, rp.received_at, rp.account_id, a.name,
           rp.transaction_id, rp.created_at
    FROM receivable_payments rp
    LEFT JOIN accounts a ON a.id = rp.account_id";

fn map_receivable_payment(row: &Row<'_>) -> rusqlite::Result<ReceivablePayment> {
    Ok(ReceivablePayment {
        id: row.get(0)?,
        receivable_id: row.get(1)?,
        amount: row.get(2)?,
        received_at: row.get(3)?,
        account_id: row.get(4)?,
        account_name: row.get(5)?,
        transaction_id: row.get(6)?,
        created_at: row.get(7)?,
    })
}

pub fn list_payments(conn: &Connection, receivable_id: &str) -> AppResult<Vec<ReceivablePayment>> {
    let sql = format!(
        "{SELECT_RECEIVABLE_PAYMENT} WHERE rp.receivable_id = ?1 ORDER BY rp.received_at DESC, rp.created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([receivable_id], map_receivable_payment)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn insert_payment(
    conn: &Connection,
    id: &str,
    receivable_id: &str,
    amount: i64,
    received_at: &str,
    account_id: Option<&str>,
    transaction_id: Option<&str>,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO receivable_payments (id, receivable_id, amount, received_at, account_id, transaction_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, receivable_id, amount, received_at, account_id, transaction_id, created_at],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}
