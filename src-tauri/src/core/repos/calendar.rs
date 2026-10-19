//! Consultas del calendario financiero.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::CalendarEvent;

fn map_event(row: &Row<'_>) -> rusqlite::Result<CalendarEvent> {
    Ok(CalendarEvent {
        id: row.get(0)?,
        source: row.get(1)?,
        title: row.get(2)?,
        amount: row.get(3)?,
        currency_code: row.get(4)?,
        date: row.get(5)?,
        status: row.get(6)?,
        detail: row.get(7)?,
    })
}

/// Eventos de vencimiento en un rango de fechas.
pub fn events(conn: &Connection, start: &str, end: &str) -> AppResult<Vec<CalendarEvent>> {
    let sql = "
        SELECT i.id AS id, 'installment' AS source, p.name AS title,
               (i.amount - i.paid_amount) AS amount, p.currency_code AS currency_code,
               i.due_date AS date, i.status AS status,
               'Cuota ' || i.number || ' de ' || p.installments_count AS detail
          FROM purchase_installments i
          JOIN purchases p ON p.id = i.purchase_id
         WHERE i.due_date >= ?1 AND i.due_date <= ?2 AND i.status <> 'cancelled'
        UNION ALL
        SELECT s.id, 'subscription', s.name, s.amount, s.currency_code,
               s.next_charge_date, s.status, 'Cobro de suscripción'
          FROM subscriptions s
         WHERE s.next_charge_date >= ?1 AND s.next_charge_date <= ?2
           AND s.status IN ('active', 'expired')
        UNION ALL
        SELECT p.id, 'payable', p.title, (p.total_amount - p.paid_amount), p.currency_code,
               p.due_date, p.status, 'Cuenta por pagar'
          FROM payables p
         WHERE p.due_date >= ?1 AND p.due_date <= ?2 AND p.status <> 'cancelled'
        UNION ALL
        SELECT r.id, 'receivable', r.title, (r.total_amount - r.received_amount), r.currency_code,
               r.due_date, r.status, 'Cuenta por cobrar'
          FROM receivables r
         WHERE r.due_date >= ?1 AND r.due_date <= ?2 AND r.status <> 'cancelled'
        UNION ALL
        SELECT d.id, 'deposit', d.title, d.amount, d.currency_code,
               d.due_date, d.status, 'Depósito pendiente'
          FROM deposits d
         WHERE d.due_date >= ?1 AND d.due_date <= ?2 AND d.status <> 'cancelled'
        ORDER BY date ASC, amount DESC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![start, end], map_event)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

/// Próximos vencimientos (cuotas, suscripciones, cuentas por pagar y depósitos).
pub fn upcoming(conn: &Connection, today: &str, limit: i64) -> AppResult<Vec<CalendarEvent>> {
    let sql = "
        SELECT i.id AS id, 'installment' AS source, p.name AS title,
               (i.amount - i.paid_amount) AS amount, p.currency_code AS currency_code,
               i.due_date AS date, i.status AS status,
               'Cuota ' || i.number || ' de ' || p.installments_count AS detail
          FROM purchase_installments i
          JOIN purchases p ON p.id = i.purchase_id
         WHERE i.due_date >= ?1 AND i.status IN ('pending', 'overdue')
        UNION ALL
        SELECT s.id, 'subscription', s.name, s.amount, s.currency_code,
               s.next_charge_date, s.status, 'Cobro de suscripción'
          FROM subscriptions s
         WHERE s.next_charge_date >= ?1 AND s.status IN ('active', 'expired')
        UNION ALL
        SELECT p.id, 'payable', p.title, (p.total_amount - p.paid_amount), p.currency_code,
               p.due_date, p.status, 'Cuenta por pagar'
          FROM payables p
         WHERE p.due_date >= ?1 AND p.status IN ('pending', 'partial', 'overdue')
        UNION ALL
        SELECT r.id, 'receivable', r.title, (r.total_amount - r.received_amount), r.currency_code,
               r.due_date, r.status, 'Cuenta por cobrar'
          FROM receivables r
         WHERE r.due_date >= ?1 AND r.status IN ('pending', 'partial', 'overdue')
        UNION ALL
        SELECT d.id, 'deposit', d.title, d.amount, d.currency_code,
               d.due_date, d.status, 'Depósito pendiente'
          FROM deposits d
         WHERE d.due_date >= ?1 AND d.status IN ('pending', 'scheduled', 'overdue')
        ORDER BY date ASC
        LIMIT ?2";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![today, limit], map_event)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}
