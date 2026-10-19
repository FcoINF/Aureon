//! Servicio de reportes y panel principal.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{
    CategoryBreakdown, HistorySummary, SeriesPoint, SummaryByCurrency, Transaction,
    TransactionFilter, UpcomingCommitment,
};
use crate::core::repos;
use crate::core::services::{subscriptions as subscription_service, transactions as tx_service};

/// Resumen del panel principal para una moneda.
pub fn dashboard(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
) -> AppResult<SummaryByCurrency> {
    let summaries = repos::reports::summary_by_currency(conn, start, end)?;
    let mut summary = summaries
        .into_iter()
        .find(|s| s.currency_code == currency)
        .ok_or_else(|| AppError::other("No hay datos para la moneda seleccionada."))?;

    // Dinero por cobrar pendiente.
    summary.receivable_pending = scalar(
        conn,
        "SELECT COALESCE(SUM(total_amount - received_amount), 0) FROM receivables
          WHERE currency_code = ?1 AND status IN ('pending', 'partial', 'overdue')",
        currency,
    )?;

    // Dinero por pagar pendiente.
    summary.payable_pending = scalar(
        conn,
        "SELECT COALESCE(SUM(total_amount - paid_amount), 0) FROM payables
          WHERE currency_code = ?1 AND status IN ('pending', 'partial', 'overdue')",
        currency,
    )?;

    // Cuotas por pagar.
    summary.installments_pending = scalar(
        conn,
        "SELECT COUNT(*) FROM purchase_installments i
           JOIN purchases p ON p.id = i.purchase_id
          WHERE p.currency_code = ?1 AND i.status IN ('pending', 'overdue')",
        currency,
    )?;
    summary.installments_pending_amount = scalar(
        conn,
        "SELECT COALESCE(SUM(i.amount - i.paid_amount), 0) FROM purchase_installments i
           JOIN purchases p ON p.id = i.purchase_id
          WHERE p.currency_code = ?1 AND i.status IN ('pending', 'overdue')",
        currency,
    )?;

    // Suscripciones activas y su costo mensual estimado.
    let subscriptions = subscription_service::list(conn, Some("active"))?;
    let mut monthly = 0i64;
    for subscription in subscriptions.iter() {
        if subscription.currency_code == currency {
            summary.subscriptions_active += 1;
            monthly += subscription.monthly_estimate;
        }
    }
    summary.subscriptions_monthly = monthly;

    // Depósitos pendientes.
    summary.deposits_pending = scalar(
        conn,
        "SELECT COUNT(*) FROM deposits
          WHERE currency_code = ?1 AND status IN ('pending', 'scheduled', 'overdue')",
        currency,
    )?;
    summary.deposits_pending_amount = scalar(
        conn,
        "SELECT COALESCE(SUM(amount), 0) FROM deposits
          WHERE currency_code = ?1 AND status IN ('pending', 'scheduled', 'overdue')",
        currency,
    )?;

    // Compromisos vencidos.
    let today = dates::today_str();
    summary.overdue_count = scalar(
        conn,
        "SELECT (SELECT COUNT(*) FROM purchase_installments i JOIN purchases p ON p.id = i.purchase_id
                   WHERE p.currency_code = ?1 AND i.status = 'overdue')
              + (SELECT COUNT(*) FROM payables WHERE currency_code = ?1 AND status = 'overdue')
              + (SELECT COUNT(*) FROM deposits WHERE currency_code = ?1 AND status = 'overdue')
              + (SELECT COUNT(*) FROM receivables WHERE currency_code = ?1 AND status = 'overdue')",
        currency,
    )?;
    summary.overdue_amount = scalar(
        conn,
        "SELECT (SELECT COALESCE(SUM(i.amount - i.paid_amount), 0) FROM purchase_installments i JOIN purchases p ON p.id = i.purchase_id
                   WHERE p.currency_code = ?1 AND i.status = 'overdue')
              + (SELECT COALESCE(SUM(total_amount - paid_amount), 0) FROM payables
                   WHERE currency_code = ?1 AND status = 'overdue')
              + (SELECT COALESCE(SUM(amount), 0) FROM deposits
                   WHERE currency_code = ?1 AND status = 'overdue')",
        currency,
    )?;

    // Próximos compromisos (30 días).
    let horizon = (dates::today() + chrono::Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let events = repos::calendar::upcoming(conn, &today, 100)?;
    for event in events.iter() {
        if event.currency_code != currency || event.date > horizon {
            continue;
        }
        if event.status == "overdue" {
            continue;
        }
        summary.upcoming_amount += event.amount;
        summary.upcoming_count += 1;
    }

    Ok(summary)
}

fn scalar(conn: &Connection, sql: &str, currency: &str) -> AppResult<i64> {
    conn.query_row(sql, [currency], |row| row.get(0))
        .map_err(AppError::from_sqlite)
}

/// Resumen del período por moneda.
pub fn summary_by_currency(conn: &Connection, start: &str, end: &str) -> AppResult<Vec<SummaryByCurrency>> {
    repos::reports::summary_by_currency(conn, start, end)
}

pub fn income_expense_series(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
    granularity: &str,
) -> AppResult<Vec<SeriesPoint>> {
    repos::reports::income_expense_series(conn, start, end, currency, granularity)
}

pub fn expenses_by_category(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
) -> AppResult<Vec<CategoryBreakdown>> {
    repos::reports::expenses_by_category(conn, start, end, currency)
}

pub fn expenses_by_contact(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
) -> AppResult<Vec<CategoryBreakdown>> {
    repos::reports::expenses_by_contact(conn, start, end, currency)
}

pub fn balance_evolution(
    conn: &Connection,
    start: &str,
    end: &str,
    currency: &str,
) -> AppResult<Vec<SeriesPoint>> {
    repos::reports::balance_evolution(conn, start, end, currency)
}

pub fn history_summary(
    conn: &Connection,
    start: &str,
    end: &str,
) -> AppResult<Vec<HistorySummary>> {
    repos::reports::history_summary(conn, start, end)
}

pub fn recent_transactions(conn: &Connection, limit: i64) -> AppResult<Vec<Transaction>> {
    let filter = TransactionFilter {
        limit: Some(limit),
        ..Default::default()
    };
    tx_service::list(conn, &filter)
}

/// Compromisos futuros mostrados en el panel principal.
pub fn upcoming_commitments(
    conn: &Connection,
    today: &str,
    limit: i64,
) -> AppResult<Vec<UpcomingCommitment>> {
    let events = repos::calendar::upcoming(conn, today, limit)?;
    Ok(events
        .into_iter()
        .map(|event| UpcomingCommitment {
            id: event.id,
            source: event.source,
            title: event.title,
            amount: event.amount,
            currency_code: event.currency_code,
            due_date: event.date,
            status: event.status,
            detail: event.detail,
        })
        .collect())
}
