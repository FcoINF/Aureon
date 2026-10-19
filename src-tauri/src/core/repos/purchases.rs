//! Consultas de compras, cuotas y sus pagos.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Installment, Purchase, PurchaseInput};

const SELECT_PURCHASE: &str = "
    SELECT p.id, p.name, p.contact_id, ct.name, p.url, p.purchase_date,
           p.subtotal_amount, p.interest_amount, p.fees_amount, p.total_amount,
           p.currency_code, p.installments_count, p.installment_amount, p.first_due_date,
           p.frequency, p.frequency_days, p.payment_method, p.account_id, a.name,
           p.status, p.notes, p.created_at,
           p.category_id, c.name,
           COALESCE((SELECT SUM(i.paid_amount) FROM purchase_installments i
                      WHERE i.purchase_id = p.id AND i.status <> 'cancelled'), 0) AS paid_amount,
           COALESCE((SELECT SUM(i.amount - i.paid_amount) FROM purchase_installments i
                      WHERE i.purchase_id = p.id AND i.status <> 'cancelled' AND i.paid_amount < i.amount), 0) AS pending_amount,
           COALESCE((SELECT COUNT(*) FROM purchase_installments i
                      WHERE i.purchase_id = p.id AND i.status = 'paid'), 0) AS installments_paid,
           COALESCE((SELECT COUNT(*) FROM purchase_installments i
                      WHERE i.purchase_id = p.id AND i.status IN ('pending', 'overdue')), 0) AS installments_pending,
           COALESCE((SELECT COUNT(*) FROM purchase_installments i
                      WHERE i.purchase_id = p.id AND i.status = 'overdue'), 0) AS installments_overdue,
           (SELECT MIN(i.due_date) FROM purchase_installments i
              WHERE i.purchase_id = p.id AND i.status IN ('pending', 'overdue')) AS next_due_date,
           (SELECT MAX(i.due_date) FROM purchase_installments i
              WHERE i.purchase_id = p.id) AS last_due_date
    FROM purchases p
    LEFT JOIN contacts ct ON ct.id = p.contact_id
    LEFT JOIN accounts a  ON a.id = p.account_id
    LEFT JOIN categories c ON c.id = p.category_id";

fn map_purchase(row: &Row<'_>) -> rusqlite::Result<Purchase> {
    Ok(Purchase {
        id: row.get(0)?,
        name: row.get(1)?,
        contact_id: row.get(2)?,
        contact_name: row.get(3)?,
        url: row.get(4)?,
        purchase_date: row.get(5)?,
        subtotal_amount: row.get(6)?,
        interest_amount: row.get(7)?,
        fees_amount: row.get(8)?,
        total_amount: row.get(9)?,
        currency_code: row.get(10)?,
        installments_count: row.get(11)?,
        installment_amount: row.get(12)?,
        first_due_date: row.get(13)?,
        frequency: row.get(14)?,
        frequency_days: row.get(15)?,
        payment_method: row.get(16)?,
        account_id: row.get(17)?,
        account_name: row.get(18)?,
        status: row.get(19)?,
        notes: row.get(20)?,
        created_at: row.get(21)?,
        category_id: row.get(22)?,
        category_name: row.get(23)?,
        paid_amount: row.get(24)?,
        pending_amount: row.get(25)?,
        installments_paid: row.get(26)?,
        installments_pending: row.get(27)?,
        installments_overdue: row.get(28)?,
        next_due_date: row.get(29)?,
        last_due_date: row.get(30)?,
    })
}

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Purchase>> {
    let mut sql = format!("{SELECT_PURCHASE} WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(s) = status.filter(|s| !s.is_empty()) {
        sql.push_str(" AND p.status = ?1");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY p.purchase_date DESC, p.created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_purchase)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Purchase> {
    let sql = format!("{SELECT_PURCHASE} WHERE p.id = ?1");
    conn.query_row(&sql, [id], map_purchase)
        .map_err(|_| AppError::not_found("La compra no existe."))
}

pub fn insert(conn: &Connection, id: &str, input: &PurchaseInput, created_at: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO purchases (id, name, contact_id, url, purchase_date, subtotal_amount,
            interest_amount, fees_amount, total_amount, currency_code, installments_count,
            installment_amount, first_due_date, frequency, frequency_days, payment_method,
            account_id, category_id, status, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        params![
            id,
            input.name,
            input.contact_id,
            input.url,
            input.purchase_date,
            input.total_amount,
            0,
            0,
            input.total_amount,
            input.currency_code,
            input.installments_count,
            input.installment_amount.unwrap_or(input.total_amount),
            input.first_due_date,
            input.frequency.clone().unwrap_or_else(|| "monthly".to_string()),
            input.frequency_days,
            input.payment_method,
            input.account_id,
            input.category_id,
            input.status.clone().unwrap_or_else(|| "active".to_string()),
            input.notes,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &PurchaseInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE purchases SET name = ?2, contact_id = ?3, url = ?4, purchase_date = ?5,
                total_amount = ?6, currency_code = ?7, installments_count = ?8,
                installment_amount = ?9, first_due_date = ?10, frequency = ?11, frequency_days = ?12,
                payment_method = ?13, account_id = ?14, category_id = ?15, status = ?16, notes = ?17
             WHERE id = ?1",
            params![
                id,
                input.name,
                input.contact_id,
                input.url,
                input.purchase_date,
                input.total_amount,
                input.currency_code,
                input.installments_count,
                input.installment_amount.unwrap_or(input.total_amount),
                input.first_due_date,
                input.frequency.clone().unwrap_or_else(|| "monthly".to_string()),
                input.frequency_days,
                input.payment_method,
                input.account_id,
                input.category_id,
                input.status.clone().unwrap_or_else(|| "active".to_string()),
                input.notes
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La compra no existe."));
    }
    Ok(())
}

pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE purchases SET status = ?2 WHERE id = ?1",
            params![id, status],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La compra no existe."));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Cuotas
// ---------------------------------------------------------------------------

const SELECT_INSTALLMENT: &str = "
    SELECT i.id, i.purchase_id, p.name, i.number, i.amount, i.due_date, i.status,
           i.paid_at, i.paid_amount, i.account_id, a.name, i.transaction_id
    FROM purchase_installments i
    JOIN purchases p ON p.id = i.purchase_id
    LEFT JOIN accounts a ON a.id = i.account_id";

fn map_installment(row: &Row<'_>) -> rusqlite::Result<Installment> {
    Ok(Installment {
        id: row.get(0)?,
        purchase_id: row.get(1)?,
        purchase_name: row.get(2)?,
        number: row.get(3)?,
        amount: row.get(4)?,
        due_date: row.get(5)?,
        status: row.get(6)?,
        paid_at: row.get(7)?,
        paid_amount: row.get(8)?,
        account_id: row.get(9)?,
        account_name: row.get(10)?,
        transaction_id: row.get(11)?,
    })
}

pub fn list_installments(conn: &Connection, purchase_id: &str) -> AppResult<Vec<Installment>> {
    let sql = format!("{SELECT_INSTALLMENT} WHERE i.purchase_id = ?1 ORDER BY i.number ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([purchase_id], map_installment)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get_installment(conn: &Connection, id: &str) -> AppResult<Installment> {
    let sql = format!("{SELECT_INSTALLMENT} WHERE i.id = ?1");
    conn.query_row(&sql, [id], map_installment)
        .map_err(|_| AppError::not_found("La cuota no existe."))
}

pub fn insert_installment(
    conn: &Connection,
    id: &str,
    purchase_id: &str,
    number: i64,
    amount: i64,
    due_date: &str,
    status: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO purchase_installments (id, purchase_id, number, amount, due_date, status, paid_amount)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
        params![id, purchase_id, number, amount, due_date, status],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update_installment(
    conn: &Connection,
    id: &str,
    amount: i64,
    due_date: &str,
    status: &str,
) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE purchase_installments SET amount = ?2, due_date = ?3, status = ?4 WHERE id = ?1",
            params![id, amount, due_date, status],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuota no existe."));
    }
    Ok(())
}

/// Actualiza el saldo pagado y el estado de la cuota tras registrar un pago.
pub fn apply_installment_payment(
    conn: &Connection,
    id: &str,
    paid_amount: i64,
    status: &str,
    paid_at: Option<&str>,
    account_id: Option<&str>,
    transaction_id: Option<&str>,
) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE purchase_installments
                SET paid_amount = ?2, status = ?3, paid_at = ?4, account_id = ?5, transaction_id = ?6
              WHERE id = ?1",
            params![id, paid_amount, status, paid_at, account_id, transaction_id],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La cuota no existe."));
    }
    Ok(())
}

pub fn insert_installment_payment(
    conn: &Connection,
    id: &str,
    installment_id: &str,
    amount: i64,
    paid_at: &str,
    account_id: Option<&str>,
    transaction_id: Option<&str>,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO installment_payments (id, installment_id, amount, paid_at, account_id, transaction_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, installment_id, amount, paid_at, account_id, transaction_id, created_at],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn list_installment_payments(
    conn: &Connection,
    installment_id: &str,
) -> AppResult<Vec<(String, i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, amount, paid_at FROM installment_payments WHERE installment_id = ?1 ORDER BY paid_at ASC, created_at ASC",
    )?;
    let rows = stmt.query_map([installment_id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM purchases WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}

/// Elimina las cuotas que todavía no tienen pagos registrados.
pub fn delete_unpaid_installments(conn: &Connection, purchase_id: &str) -> AppResult<usize> {
    let changed = conn
        .execute(
            "DELETE FROM purchase_installments
              WHERE purchase_id = ?1
                AND id NOT IN (SELECT installment_id FROM installment_payments)",
            [purchase_id],
        )
        .map_err(AppError::from_sqlite)?;
    Ok(changed)
}

/// Número de pagos ya registrados en una compra.
pub fn count_payments(conn: &Connection, purchase_id: &str) -> AppResult<i64> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM installment_payments ip
               JOIN purchase_installments i ON i.id = ip.installment_id
              WHERE i.purchase_id = ?1",
            [purchase_id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    Ok(count)
}

/// Cancela las cuotas pendientes de una compra.
pub fn cancel_pending_installments(conn: &Connection, purchase_id: &str) -> AppResult<usize> {
    let changed = conn
        .execute(
            "UPDATE purchase_installments SET status = 'cancelled'
              WHERE purchase_id = ?1 AND status IN ('pending', 'overdue')",
            [purchase_id],
        )
        .map_err(AppError::from_sqlite)?;
    Ok(changed)
}
