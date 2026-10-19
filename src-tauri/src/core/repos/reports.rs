//! Consultas de reportes y agregados.

use rusqlite::{params, Connection};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{CategoryBreakdown, HistorySummary, SeriesPoint, SummaryByCurrency};

/// Efecto de un movimiento sobre el saldo, en unidades de la cuenta.
fn signed_effect(kind: &str, amount: i64, is_account: bool, is_transfer_target: bool) -> i64 {
    if !is_account && !is_transfer_target {
        return 0;
    }
    match kind {
        "income" | "refund" => {
            if is_account {
                amount
            } else {
                0
            }
        }
        "expense" => {
            if is_account {
                -amount
            } else {
                0
            }
        }
        "transfer" => {
            if is_account {
                -amount
            } else {
                amount
            }
        }
        _ => 0,
    }
}

pub fn balance_evolution(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
) -> AppResult<Vec<SeriesPoint>> {
    // Saldo acumulado hasta el inicio del período.
    let mut running: i64 = 0;
    let mut stmt = conn.prepare(
        "SELECT t.type, t.amount, t.account_id, t.transfer_account_id
           FROM transactions t
          WHERE t.voided = 0 AND t.date < ?1
            AND (t.account_id IN (SELECT id FROM accounts WHERE currency_code = ?2)
              OR t.transfer_account_id IN (SELECT id FROM accounts WHERE currency_code = ?2))
          ORDER BY t.date ASC, t.created_at ASC",
    )?;
    let rows = stmt.query_map(params![start, currency], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    for row in rows {
        let (kind, amount, account_id, transfer_id) = row.map_err(AppError::from_sqlite)?;
        let is_account = account_id.is_some();
        let is_transfer = transfer_id.is_some();
        running += signed_effect(&kind, amount, is_account, is_transfer);
    }

    // Evolución diaria dentro del período.
    let mut stmt = conn.prepare(
        "SELECT t.date, t.type, t.amount, t.account_id, t.transfer_account_id
           FROM transactions t
          WHERE t.voided = 0 AND t.date >= ?1 AND t.date <= ?2
            AND (t.account_id IN (SELECT id FROM accounts WHERE currency_code = ?3)
              OR t.transfer_account_id IN (SELECT id FROM accounts WHERE currency_code = ?3))
          ORDER BY t.date ASC, t.created_at ASC",
    )?;
    let rows = stmt.query_map(params![start, end, currency], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;
    let mut series: Vec<SeriesPoint> = Vec::new();
    for row in rows {
        let (date, kind, amount, account_id, transfer_id) = row.map_err(AppError::from_sqlite)?;
        running += signed_effect(&kind, amount, account_id.is_some(), transfer_id.is_some());
        let label = format!("{}-{}", &date[5..], &date[8..]);
        match series.last_mut() {
            Some(point) if point.label == label => point.value = running,
            _ => series.push(SeriesPoint {
                label,
                value: running,
                secondary: None,
            }),
        }
    }
    Ok(series)
}

/// Ingresos y gastos por día/semana/mes dentro del período.
pub fn income_expense_series(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
    granularity: &str,
) -> AppResult<Vec<SeriesPoint>> {
    let expr = match granularity {
        "month" => "substr(t.date, 1, 7)",
        "year" => "substr(t.date, 1, 4)",
        _ => "t.date",
    };
    let sql = format!(
        "SELECT {expr} AS bucket,
                COALESCE(SUM(CASE WHEN t.type = 'income' THEN t.amount ELSE 0 END), 0) AS income,
                COALESCE(SUM(CASE WHEN t.type = 'expense' THEN t.amount ELSE 0 END), 0) AS expenses,
                COALESCE(SUM(CASE WHEN t.type = 'refund' THEN t.amount ELSE 0 END), 0) AS refunds
           FROM transactions t
          WHERE t.voided = 0 AND t.date >= ?1 AND t.date <= ?2
            AND t.currency_code = ?3
            AND t.type IN ('income', 'expense', 'refund')
          GROUP BY bucket
          ORDER BY bucket ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![start, end, currency], |row| {
        Ok(SeriesPoint {
            label: row.get(0)?,
            value: row.get(1)?,
            secondary: Some(row.get(2)?),
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn expenses_by_category(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
) -> AppResult<Vec<CategoryBreakdown>> {
    let sql = "SELECT t.category_id, COALESCE(c.name, 'Sin categoría') AS label,
                      COALESCE(c.color, '#9A8F7A') AS color, SUM(t.amount) AS amount
                 FROM transactions t
                 LEFT JOIN categories c ON c.id = t.category_id
                WHERE t.voided = 0 AND t.type = 'expense' AND t.date >= ?1 AND t.date <= ?2
                  AND t.currency_code = ?3
                GROUP BY t.category_id
                ORDER BY amount DESC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![start, end, currency], |row| {
        Ok(CategoryBreakdown {
            id: row.get(0)?,
            label: row.get(1)?,
            color: Some(row.get(2)?),
            amount: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn expenses_by_contact(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
) -> AppResult<Vec<CategoryBreakdown>> {
    let sql = "SELECT t.contact_id, COALESCE(ct.name, 'Sin comercio o persona') AS label,
                      '#B08B4A' AS color, SUM(t.amount) AS amount
                 FROM transactions t
                 LEFT JOIN contacts ct ON ct.id = t.contact_id
                WHERE t.voided = 0 AND t.type = 'expense' AND t.date >= ?1 AND t.date <= ?2
                  AND t.currency_code = ?3
                GROUP BY t.contact_id
                ORDER BY amount DESC
                LIMIT 12";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![start, end, currency], |row| {
        Ok(CategoryBreakdown {
            id: row.get(0)?,
            label: row.get(1)?,
            color: Some(row.get(2)?),
            amount: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

/// Resumen del período por moneda.
pub fn summary_by_currency(
    conn: &Connection,
    start: &str,
    end: &str,
) -> AppResult<Vec<SummaryByCurrency>> {
    let sql = "SELECT c.code,
                COALESCE((SELECT SUM(a.initial_balance) FROM accounts a
                           WHERE a.currency_code = c.code AND a.archived = 0), 0)
                + COALESCE((SELECT SUM(CASE
                        WHEN t.type IN ('income', 'refund') THEN t.amount
                        WHEN t.type = 'expense' THEN -t.amount
                        WHEN t.type = 'transfer' AND t.account_id IN (SELECT id FROM accounts WHERE currency_code = c.code) THEN -t.amount
                        WHEN t.type = 'transfer' AND t.transfer_account_id IN (SELECT id FROM accounts WHERE currency_code = c.code) THEN t.amount
                        ELSE 0 END)
                    FROM transactions t
                   WHERE t.voided = 0 AND t.date < ?1
                     AND (t.account_id IN (SELECT id FROM accounts WHERE currency_code = c.code)
                       OR t.transfer_account_id IN (SELECT id FROM accounts WHERE currency_code = c.code))), 0) AS opening,
                COALESCE((SELECT SUM(CASE WHEN t.type = 'income' THEN t.amount ELSE 0 END)
                    FROM transactions t WHERE t.voided = 0 AND t.type = 'income'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0) AS income,
                COALESCE((SELECT SUM(CASE WHEN t.type = 'expense' THEN t.amount ELSE 0 END)
                    FROM transactions t WHERE t.voided = 0 AND t.type = 'expense'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0) AS expenses,
                COALESCE((SELECT SUM(CASE WHEN t.type = 'refund' THEN t.amount ELSE 0 END)
                    FROM transactions t WHERE t.voided = 0 AND t.type = 'refund'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0) AS refunds,
                COALESCE((SELECT SUM(t.amount)
                    FROM transactions t WHERE t.voided = 0 AND t.type = 'transfer'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0) AS transfers
               FROM currencies c
              ORDER BY c.code ASC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![start, end], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let mut result = Vec::new();
    for row in rows {
        let (code, opening, income, expenses, refunds, transfers) =
            row.map_err(AppError::from_sqlite)?;
        result.push((
            code,
            opening,
            income,
            expenses,
            refunds,
            transfers,
        ));
    }
    Ok(result
        .into_iter()
        .map(|(code, opening, income, expenses, refunds, transfers)| SummaryByCurrency {
            currency_code: code,
            balance: opening + income - expenses + refunds,
            income,
            expenses,
            net: income - expenses + refunds,
            receivable_pending: 0,
            payable_pending: 0,
            installments_pending: 0,
            installments_pending_amount: 0,
            subscriptions_active: 0,
            subscriptions_monthly: 0,
            deposits_pending: 0,
            deposits_pending_amount: 0,
            overdue_amount: 0,
            overdue_count: 0,
            upcoming_amount: 0,
            upcoming_count: 0,
            refunds,
            transfers,
        })
        .collect())
}

/// Resumen del historial por moneda para el módulo de reportes.
pub fn history_summary(
    conn: &Connection,
    start: &str,
    end: &str,
) -> AppResult<Vec<HistorySummary>> {
    let sql = "SELECT c.code,
                COALESCE((SELECT SUM(CASE WHEN t.type = 'income' THEN t.amount ELSE 0 END)
                    FROM transactions t WHERE t.voided = 0 AND t.type = 'income'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(CASE WHEN t.type = 'expense' THEN t.amount ELSE 0 END)
                    FROM transactions t WHERE t.voided = 0 AND t.type = 'expense'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(CASE WHEN t.type = 'refund' THEN t.amount ELSE 0 END)
                    FROM transactions t WHERE t.voided = 0 AND t.type = 'refund'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(t.amount) FROM transactions t WHERE t.voided = 0
                      AND t.type = 'transfer' AND t.date >= ?1 AND t.date <= ?2
                      AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(t.amount) FROM transactions t WHERE t.voided = 0
                      AND t.type = 'expense' AND t.linked_type = 'purchase'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(t.amount) FROM transactions t WHERE t.voided = 0
                      AND t.type = 'expense' AND t.linked_type = 'installment'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(t.amount) FROM transactions t WHERE t.voided = 0
                      AND t.type = 'expense' AND t.linked_type = 'subscription'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(t.amount) FROM transactions t WHERE t.voided = 0
                      AND t.type = 'income' AND t.linked_type = 'receivable'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                COALESCE((SELECT SUM(t.amount) FROM transactions t WHERE t.voided = 0
                      AND t.type = 'expense' AND t.linked_type = 'payable'
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code), 0),
                (SELECT COUNT(*) FROM transactions t WHERE t.voided = 0
                      AND t.date >= ?1 AND t.date <= ?2 AND t.currency_code = c.code)
               FROM currencies c
              ORDER BY c.code ASC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![start, end], |row| {
        let income: i64 = row.get(1)?;
        let expenses: i64 = row.get(2)?;
        let refunds: i64 = row.get(3)?;
        Ok(HistorySummary {
            currency_code: row.get(0)?,
            income,
            expenses,
            net: income - expenses + refunds,
            refunds,
            transfers: row.get(4)?,
            purchases_total: row.get(5)?,
            installments_paid: row.get(6)?,
            subscriptions_paid: row.get(7)?,
            money_received: row.get(8)?,
            money_paid: row.get(9)?,
            pending_commitments: 0,
            overdue_commitments: 0,
            transaction_count: row.get(10)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}
