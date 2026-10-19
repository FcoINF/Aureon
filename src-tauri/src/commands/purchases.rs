//! Comandos de compras, cuotas y presupuestos.

use tauri::State;

use crate::core::models::{
    Budget, BudgetInput, Installment, InstallmentPaymentInput, Purchase, PurchaseInput,
};
use crate::core::services::{budgets, purchases, settings};
use crate::state::AppState;

fn default_currency(conn: &rusqlite::Connection) -> String {
    settings::default_currency(conn)
}

#[tauri::command]
pub fn list_purchases(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<Purchase>, String> {
    state
        .read_or(|conn| purchases::list(conn, status.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_purchase(state: State<'_, AppState>, id: String) -> Result<Purchase, String> {
    state
        .read(|conn| purchases::get(conn, &id))?
        .ok_or_else(|| "La compra no existe.".to_string())
}

#[tauri::command]
pub fn list_installments(
    state: State<'_, AppState>,
    purchase_id: String,
) -> Result<Vec<Installment>, String> {
    state
        .read_or(|conn| purchases::list_installments(conn, &purchase_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_purchase(
    state: State<'_, AppState>,
    input: PurchaseInput,
) -> Result<Purchase, String> {
    state
        .write(|conn| {
            let currency = default_currency(conn);
            purchases::create(conn, &input, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pay_installment(
    state: State<'_, AppState>,
    installment_id: String,
    input: InstallmentPaymentInput,
) -> Result<Installment, String> {
    state
        .write(|conn| purchases::pay_installment(conn, &installment_id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_installment(
    state: State<'_, AppState>,
    installment_id: String,
    amount: i64,
    due_date: String,
) -> Result<Installment, String> {
    state
        .write(|conn| purchases::update_installment(conn, &installment_id, amount, &due_date))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel_purchase(state: State<'_, AppState>, id: String) -> Result<Purchase, String> {
    state
        .write(|conn| purchases::cancel(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn refund_purchase(
    state: State<'_, AppState>,
    id: String,
    amount: i64,
    date: String,
    account_id: Option<String>,
) -> Result<Purchase, String> {
    state
        .write(|conn| purchases::refund(conn, &id, amount, &date, account_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_budgets(state: State<'_, AppState>) -> Result<Vec<Budget>, String> {
    state
        .read_or(|conn| {
            let (start, end) = crate::core::dates::period_range("month", 0)?;
            budgets::list(conn, (&start, &end))
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_budget(state: State<'_, AppState>, input: BudgetInput) -> Result<Budget, String> {
    state
        .write(|conn| budgets::upsert(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_budget(
    state: State<'_, AppState>,
    id: String,
    input: BudgetInput,
) -> Result<Budget, String> {
    state
        .write(|conn| budgets::update(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_budget(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| budgets::delete(conn, &id))
        .map_err(|e| e.to_string())
}
