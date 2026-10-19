//! Consultas de cuentas por pagar.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Payable, PayableInput, PayablePayment};

const SELECT_PAYABLE: &str = "
    SELECT p.id, p.contact_id, ct.name, p.title, p.total_amount, p.paid_amount,
           (p.total_amount - p.paid_amount) AS pending_amount, p.currency_code,
           p.created_date, p.due_date, p.account_id, a.name, p.status, p.notes, p.created_at,
           (SELECT COUNT(*) FROM payable_payments pp WHERE pp.payable_id = p.id) AS payment_count
    FROM payables p
    JOIN contacts ct ON ct.id = p.contact_id
    LEFT JOIN accounts a ON a.id = p.account_id";

fn map_payable(row: &Row<'_>) -> rusqlite::Result<Payable> {
    Ok(Payable {
        id: row.get(0)?,
        contact_id: row.get(1)?,
        contact_name: row.get(2)?,
        title: row.get(3)?,
        total_amount: row.get(4)?,
        paid_amount: row.get(5)?,
        pending_amount: row.get(6)?,
        currency_code: row.get(7)?,
        created_date: row.get(8)?,
        due_date: row.get(9)?,
        account_id: row.get(10)?,
        account_name: row.get(11)?,
        status: row.get(12)?,
        notes: row.get(13)?,
        created_at: row.get(14)?,
        payment_count: row.get(15)?,
    })
}

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Payable>> {
    let mut sql = format!("{SELECT_PAYABLE} WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(s) = status.filter(|s| !s.is_empty()) {
        sql.push_str(" AND p.status = ?1");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY CASE p.status WHEN 'partial' THEN 0 WHEN 'overdue' THEN 1 WHEN 'pending' THEN 2 ELSE 3 END, p.due_date IS NULL, p.due_date ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_payable)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Payable> {
    let sql = format!("{SELECT_PAYABLE} WHERE p.id = ?1");
    conn.query_row(&sql, [id], map_payable)
        .map_err(|_| AppError::not_found("La cuenta por pagar no existe."))
}

pub fn insert(conn: &Connection, id: &str, input: &PayableInput, created_at: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO payables (id, contact_id, title, total_amount, paid_amount, currency_code,
            created_date, due_date, account_id, status, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, 'pending', ?9, ?10)",
        params![
            id,
            input.contact_id,
            input.title,
            input.total_amount,
            input.currency_code,
            input.created_date,
            input.due_date,
            input.account_id,
            input.notes,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &PayableInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE payables SET contact_id = ?2, title = ?3, total_amount = ?4, currency_code = ?5,
                created_date = ?6, due_date = ?7, account_id = ?8, notes = ?9 WHERE id = ?1",
            params![
                id,
                input.contact_id,
                input.title,
                input.total_amount,
                input.currency_code,
                input.created_date,
                input.due_date,
                input.account_id,
                input.notes
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta por pagar no existe."));
    }
    Ok(())
}

pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<()> {
    let changed = conn
        .execute("UPDATE payables SET status = ?2 WHERE id = ?1", params![id, status])
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta por pagar no existe."));
    }
    Ok(())
}

/// Actualiza el total pagado y el estado tras registrar un pago.
pub fn apply_payment(
    conn: &Connection,
    id: &str,
    paid_amount: i64,
    status: &str,
) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE payables SET paid_amount = ?2, status = ?3 WHERE id = ?1",
            params![id, paid_amount, status],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuenta por pagar no existe."));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM payables WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}

const SELECT_PAYABLE_PAYMENT: &str = "
    SELECT pp.id, pp.payable_id, pp.amount, pp.paid_at, pp.account_id, a.name,
           pp.transaction_id, pp.created_at
    FROM payable_payments pp
    LEFT JOIN accounts a ON a.id = pp.account_id";

fn map_payable_payment(row: &Row<'_>) -> rusqlite::Result<PayablePayment> {
    Ok(PayablePayment {
        id: row.get(0)?,
        payable_id: row.get(1)?,
        amount: row.get(2)?,
        paid_at: row.get(3)?,
        account_id: row.get(4)?,
        account_name: row.get(5)?,
        transaction_id: row.get(6)?,
        created_at: row.get(7)?,
    })
}

pub fn list_payments(conn: &Connection, payable_id: &str) -> AppResult<Vec<PayablePayment>> {
    let sql = format!(
        "{SELECT_PAYABLE_PAYMENT} WHERE pp.payable_id = ?1 ORDER BY pp.paid_at DESC, pp.created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([payable_id], map_payable_payment)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

#[allow(clippy::too_many_arguments)]
pub fn insert_payment(
    conn: &Connection,
    id: &str,
    payable_id: &str,
    amount: i64,
    paid_at: &str,
    account_id: Option<&str>,
    transaction_id: Option<&str>,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO payable_payments (id, payable_id, amount, paid_at, account_id, transaction_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, payable_id, amount, paid_at, account_id, transaction_id, created_at],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}
