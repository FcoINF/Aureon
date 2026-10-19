//! Comandos de reportes, panel principal y calendario.

use tauri::State;

use crate::core::dates;
use crate::core::models::{
    CalendarEvent, CategoryBreakdown, HistorySummary, SeriesPoint, SummaryByCurrency,
    Transaction, TransactionFilter, UpcomingCommitment,
};
use crate::core::repos;
use crate::core::services::{reports, settings, transactions as tx_service};
use crate::state::AppState;

fn period_range(period: &str) -> Result<(String, String), String> {
    dates::period_range(period, 0).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dashboard_summary(
    state: State<'_, AppState>,
    period: Option<String>,
    currency: Option<String>,
) -> Result<Option<SummaryByCurrency>, String> {
    let period = period.unwrap_or_else(|| "month".to_string());
    let (start, end) = period_range(&period)?;
    state
        .read(|conn| {
            let currency = currency
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| settings::default_currency(conn));
            reports::dashboard(conn, &start, &end, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn summary_by_currency(
    state: State<'_, AppState>,
    period: Option<String>,
) -> Result<Vec<SummaryByCurrency>, String> {
    let period = period.unwrap_or_else(|| "month".to_string());
    let (start, end) = period_range(&period)?;
    state
        .read_or(|conn| reports::summary_by_currency(conn, &start, &end))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn income_expense_series(
    state: State<'_, AppState>,
    period: Option<String>,
    currency: Option<String>,
    granularity: Option<String>,
) -> Result<Vec<SeriesPoint>, String> {
    let period = period.unwrap_or_else(|| "month".to_string());
    let (start, end) = period_range(&period)?;
    state
        .read_or(|conn| {
            let currency = currency
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| settings::default_currency(conn));
            reports::income_expense_series(
                conn,
                &start,
                &end,
                &currency,
                granularity.as_deref().unwrap_or("day"),
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn expenses_by_category(
    state: State<'_, AppState>,
    period: Option<String>,
    currency: Option<String>,
) -> Result<Vec<CategoryBreakdown>, String> {
    let period = period.unwrap_or_else(|| "month".to_string());
    let (start, end) = period_range(&period)?;
    state
        .read_or(|conn| {
            let currency = currency
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| settings::default_currency(conn));
            reports::expenses_by_category(conn, &start, &end, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn expenses_by_contact(
    state: State<'_, AppState>,
    period: Option<String>,
    currency: Option<String>,
) -> Result<Vec<CategoryBreakdown>, String> {
    let period = period.unwrap_or_else(|| "month".to_string());
    let (start, end) = period_range(&period)?;
    state
        .read_or(|conn| {
            let currency = currency
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| settings::default_currency(conn));
            reports::expenses_by_contact(conn, &start, &end, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn balance_evolution(
    state: State<'_, AppState>,
    period: Option<String>,
    currency: Option<String>,
) -> Result<Vec<SeriesPoint>, String> {
    let period = period.unwrap_or_else(|| "month".to_string());
    let (start, end) = period_range(&period)?;
    state
        .read_or(|conn| {
            let currency = currency
                .filter(|c| !c.trim().is_empty())
                .unwrap_or_else(|| settings::default_currency(conn));
            reports::balance_evolution(conn, &start, &end, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_summary(
    state: State<'_, AppState>,
    period: Option<String>,
) -> Result<Vec<HistorySummary>, String> {
    let period = period.unwrap_or_else(|| "month".to_string());
    let (start, end) = period_range(&period)?;
    state
        .read_or(|conn| reports::history_summary(conn, &start, &end))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn recent_transactions(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<Transaction>, String> {
    state
        .read_or(|conn| reports::recent_transactions(conn, limit.unwrap_or(8)))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn upcoming_commitments(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<UpcomingCommitment>, String> {
    let today = dates::today_str();
    state
        .read_or(|conn| reports::upcoming_commitments(conn, &today, limit.unwrap_or(8)))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn calendar_events(
    state: State<'_, AppState>,
    start: String,
    end: String,
) -> Result<Vec<CalendarEvent>, String> {
    state
        .read_or(|conn| repos::calendar::events(conn, &start, &end))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_calendar_events(
    state: State<'_, AppState>,
    start: String,
    end: String,
) -> Result<Vec<CalendarEvent>, String> {
    state
        .read_or(|conn| repos::calendar::events(conn, &start, &end))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_transactions(
    state: State<'_, AppState>,
    filter: TransactionFilter,
) -> Result<Vec<Transaction>, String> {
    state
        .read_or(|conn| tx_service::list(conn, &filter))
        .map_err(|e| e.to_string())
}
