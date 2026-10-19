//! Consultas de suscripciones, cobros y participación.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Subscription, SubscriptionInput, SubscriptionPayment, SubscriptionShare};

const SELECT_SUBSCRIPTION: &str = "
    SELECT s.id, s.name, s.category_id, c.name, s.contact_id, ct.name, s.url,
           s.amount, s.currency_code, s.frequency, s.frequency_days, s.start_date,
           s.next_charge_date, s.account_id, a.name, s.payment_method,
           s.responsible_contact_id, rc.name, s.status, s.cancelled_at, s.notes, s.created_at,
           COALESCE((SELECT SUM(sp.amount) FROM subscription_payments sp
                      WHERE sp.subscription_id = s.id), 0) AS paid_total,
           (SELECT COUNT(*) FROM subscription_payments sp
              WHERE sp.subscription_id = s.id) AS payment_count
    FROM subscriptions s
    LEFT JOIN categories c  ON c.id = s.category_id
    LEFT JOIN contacts ct   ON ct.id = s.contact_id
    LEFT JOIN contacts rc   ON rc.id = s.responsible_contact_id
    LEFT JOIN accounts a    ON a.id = s.account_id";

fn map_subscription(row: &Row<'_>) -> rusqlite::Result<Subscription> {
    Ok(Subscription {
        id: row.get(0)?,
        name: row.get(1)?,
        category_id: row.get(2)?,
        category_name: row.get(3)?,
        contact_id: row.get(4)?,
        contact_name: row.get(5)?,
        url: row.get(6)?,
        amount: row.get(7)?,
        currency_code: row.get(8)?,
        frequency: row.get(9)?,
        frequency_days: row.get(10)?,
        start_date: row.get(11)?,
        next_charge_date: row.get(12)?,
        account_id: row.get(13)?,
        account_name: row.get(14)?,
        payment_method: row.get(15)?,
        responsible_contact_id: row.get(16)?,
        responsible_name: row.get(17)?,
        status: row.get(18)?,
        cancelled_at: row.get(19)?,
        notes: row.get(20)?,
        created_at: row.get(21)?,
        paid_total: row.get(22)?,
        monthly_estimate: 0,
        yearly_estimate: 0,
        payment_count: row.get(23)?,
    })
}

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Subscription>> {
    let mut sql = format!("{SELECT_SUBSCRIPTION} WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(s) = status.filter(|s| !s.is_empty()) {
        sql.push_str(" AND s.status = ?1");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY s.status ASC, s.name COLLATE NOCASE ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_subscription)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Subscription> {
    let sql = format!("{SELECT_SUBSCRIPTION} WHERE s.id = ?1");
    conn.query_row(&sql, [id], map_subscription)
        .map_err(|_| AppError::not_found("La suscripción no existe."))
}

pub fn insert(
    conn: &Connection,
    id: &str,
    input: &SubscriptionInput,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO subscriptions (id, name, category_id, contact_id, url, amount, currency_code,
            frequency, frequency_days, start_date, next_charge_date, account_id, payment_method,
            responsible_contact_id, status, cancelled_at, notes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, NULL, ?16, ?17)",
        params![
            id,
            input.name,
            input.category_id,
            input.contact_id,
            input.url,
            input.amount,
            input.currency_code,
            input.frequency,
            input.frequency_days,
            input.start_date,
            input.next_charge_date,
            input.account_id,
            input.payment_method,
            input.responsible_contact_id,
            input.status.clone().unwrap_or_else(|| "active".to_string()),
            input.notes,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &SubscriptionInput) -> AppResult<()> {
    let status = input.status.clone().unwrap_or_else(|| "active".to_string());
    let changed = conn
        .execute(
            "UPDATE subscriptions SET name = ?2, category_id = ?3, contact_id = ?4, url = ?5,
                amount = ?6, currency_code = ?7, frequency = ?8, frequency_days = ?9,
                start_date = ?10, next_charge_date = ?11, account_id = ?12, payment_method = ?13,
                responsible_contact_id = ?14, status = ?15, notes = ?16
             WHERE id = ?1",
            params![
                id,
                input.name,
                input.category_id,
                input.contact_id,
                input.url,
                input.amount,
                input.currency_code,
                input.frequency,
                input.frequency_days,
                input.start_date,
                input.next_charge_date,
                input.account_id,
                input.payment_method,
                input.responsible_contact_id,
                status,
                input.notes
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La suscripción no existe."));
    }
    Ok(())
}

pub fn set_status(conn: &Connection, id: &str, status: &str, cancelled_at: Option<&str>) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE subscriptions SET status = ?2, cancelled_at = ?3 WHERE id = ?1",
            params![id, status, cancelled_at],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La suscripción no existe."));
    }
    Ok(())
}

pub fn set_next_charge(conn: &Connection, id: &str, next_charge: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE subscriptions SET next_charge_date = ?2 WHERE id = ?1",
        params![id, next_charge],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let used: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE linked_type = 'subscription' AND linked_id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    if used > 0 {
        return Err(AppError::conflict(
            "La suscripción tiene cobros registrados. Cancélala en lugar de eliminarla.",
        ));
    }
    conn.execute("DELETE FROM subscriptions WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Cobros
// ---------------------------------------------------------------------------

const SELECT_SUBSCRIPTION_PAYMENT: &str = "
    SELECT sp.id, sp.subscription_id, s.name, sp.amount, sp.charged_at, sp.next_charge_date,
           sp.account_id, a.name, sp.transaction_id, sp.created_at
    FROM subscription_payments sp
    JOIN subscriptions s ON s.id = sp.subscription_id
    LEFT JOIN accounts a ON a.id = sp.account_id";

fn map_subscription_payment(row: &Row<'_>) -> rusqlite::Result<SubscriptionPayment> {
    Ok(SubscriptionPayment {
        id: row.get(0)?,
        subscription_id: row.get(1)?,
        subscription_name: row.get(2)?,
        amount: row.get(3)?,
        charged_at: row.get(4)?,
        next_charge_date: row.get(5)?,
        account_id: row.get(6)?,
        account_name: row.get(7)?,
        transaction_id: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn list_payments(conn: &Connection, subscription_id: &str) -> AppResult<Vec<SubscriptionPayment>> {
    let sql = format!(
        "{SELECT_SUBSCRIPTION_PAYMENT} WHERE sp.subscription_id = ?1 ORDER BY sp.charged_at DESC, sp.created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([subscription_id], map_subscription_payment)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn list_all_payments(conn: &Connection) -> AppResult<Vec<SubscriptionPayment>> {
    let sql = format!("{SELECT_SUBSCRIPTION_PAYMENT} ORDER BY sp.charged_at DESC, sp.created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_subscription_payment)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn insert_payment(
    conn: &Connection,
    id: &str,
    subscription_id: &str,
    amount: i64,
    charged_at: &str,
    next_charge_date: Option<&str>,
    account_id: Option<&str>,
    transaction_id: Option<&str>,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO subscription_payments (id, subscription_id, amount, charged_at, next_charge_date, account_id, transaction_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, subscription_id, amount, charged_at, next_charge_date, account_id, transaction_id, created_at],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Participación
// ---------------------------------------------------------------------------

pub fn list_shares(conn: &Connection, subscription_id: &str) -> AppResult<Vec<SubscriptionShare>> {
    let mut stmt = conn.prepare(
        "SELECT sh.id, sh.subscription_id, sh.contact_id, ct.name, sh.share_type, sh.share_value, sh.receivable_id
           FROM subscription_shares sh
           JOIN contacts ct ON ct.id = sh.contact_id
          WHERE sh.subscription_id = ?1
          ORDER BY ct.name COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map([subscription_id], |row| {
        Ok(SubscriptionShare {
            id: row.get(0)?,
            subscription_id: row.get(1)?,
            contact_id: row.get(2)?,
            contact_name: row.get(3)?,
            share_type: row.get(4)?,
            share_value: row.get(5)?,
            receivable_id: row.get(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn insert_share(
    conn: &Connection,
    id: &str,
    subscription_id: &str,
    contact_id: &str,
    share_type: &str,
    share_value: i64,
    receivable_id: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO subscription_shares (id, subscription_id, contact_id, share_type, share_value, receivable_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, subscription_id, contact_id, share_type, share_value, receivable_id],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn delete_share(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM subscription_shares WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn set_share_receivable(conn: &Connection, id: &str, receivable_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE subscription_shares SET receivable_id = ?2 WHERE id = ?1",
        params![id, receivable_id],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

/// Marca como vencidas las cuotas cuya fecha ya pasó, sin cambiarlas a pagadas.
pub fn refresh_overdue(conn: &Connection, today: &str) -> AppResult<usize> {
    let changed = conn
        .execute(
            "UPDATE purchase_installments SET status = 'overdue'
              WHERE status = 'pending' AND due_date < ?1",
            [today],
        )
        .map_err(AppError::from_sqlite)?;
    Ok(changed)
}
