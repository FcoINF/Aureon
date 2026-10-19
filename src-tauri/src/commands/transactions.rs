//! Comandos de movimientos financieros.

use tauri::State;

use crate::core::models::{Transaction, TransactionFilter, TransactionInput};
use crate::core::services::transactions;
use crate::state::AppState;

#[tauri::command]
pub fn list_transactions(
    state: State<'_, AppState>,
    filter: Option<TransactionFilter>,
) -> Result<Vec<Transaction>, String> {
    let filter = filter.unwrap_or_default();
    state
        .read_or(|conn| transactions::list(conn, &filter))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn count_transactions(
    state: State<'_, AppState>,
    filter: Option<TransactionFilter>,
) -> Result<i64, String> {
    let filter = filter.unwrap_or_default();
    state
        .read_or(|conn| transactions::count(conn, &filter))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_transaction(
    state: State<'_, AppState>,
    input: TransactionInput,
) -> Result<Transaction, String> {
    state
        .write(|conn| transactions::create(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_transaction(
    state: State<'_, AppState>,
    id: String,
    input: TransactionInput,
) -> Result<Transaction, String> {
    state
        .write(|conn| transactions::update(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn void_transaction(state: State<'_, AppState>, id: String) -> Result<Transaction, String> {
    state
        .write(|conn| transactions::void(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unvoid_transaction(state: State<'_, AppState>, id: String) -> Result<Transaction, String> {
    state
        .write(|conn| transactions::unvoid(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_transaction(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| transactions::delete(conn, &id))
        .map_err(|e| e.to_string())
}
