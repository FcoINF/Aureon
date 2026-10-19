//! Comandos de cuentas por cobrar, cuentas por pagar y depósitos pendientes.

use tauri::State;

use crate::core::models::{
    Deposit, DepositCompleteInput, DepositInput, Payable, PayableInput, PayablePayment,
    PayablePaymentInput, Receivable, ReceivableInput, ReceivablePayment, ReceivablePaymentInput,
};
use crate::core::services::{deposits, payables, receivables, settings};
use crate::state::AppState;

#[tauri::command]
pub fn list_receivables(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<Receivable>, String> {
    state
        .read_or(|conn| receivables::list(conn, status.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_receivable(state: State<'_, AppState>, id: String) -> Result<Receivable, String> {
    state
        .read(|conn| receivables::get(conn, &id))?
        .ok_or_else(|| "La cuenta por cobrar no existe.".to_string())
}

#[tauri::command]
pub fn list_receivable_payments(
    state: State<'_, AppState>,
    receivable_id: String,
) -> Result<Vec<ReceivablePayment>, String> {
    state
        .read_or(|conn| receivables::list_payments(conn, &receivable_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_receivable(
    state: State<'_, AppState>,
    input: ReceivableInput,
) -> Result<Receivable, String> {
    state
        .write(|conn| {
            let currency = settings::default_currency(conn);
            receivables::create(conn, &input, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_receivable(
    state: State<'_, AppState>,
    id: String,
    input: ReceivableInput,
) -> Result<Receivable, String> {
    state
        .write(|conn| receivables::update(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn receive_money(
    state: State<'_, AppState>,
    id: String,
    input: ReceivablePaymentInput,
) -> Result<Receivable, String> {
    state
        .write(|conn| receivables::receive(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_receivable_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<Receivable, String> {
    state
        .write(|conn| receivables::set_status(conn, &id, &status))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_receivable(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| receivables::delete(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_payables(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<Payable>, String> {
    state
        .read_or(|conn| payables::list(conn, status.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_payable(state: State<'_, AppState>, id: String) -> Result<Payable, String> {
    state
        .read(|conn| payables::get(conn, &id))?
        .ok_or_else(|| "La cuenta por pagar no existe.".to_string())
}

#[tauri::command]
pub fn list_payable_payments(
    state: State<'_, AppState>,
    payable_id: String,
) -> Result<Vec<PayablePayment>, String> {
    state
        .read_or(|conn| payables::list_payments(conn, &payable_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_payable(
    state: State<'_, AppState>,
    input: PayableInput,
) -> Result<Payable, String> {
    state
        .write(|conn| {
            let currency = settings::default_currency(conn);
            payables::create(conn, &input, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_payable(
    state: State<'_, AppState>,
    id: String,
    input: PayableInput,
) -> Result<Payable, String> {
    state
        .write(|conn| payables::update(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pay_obligation(
    state: State<'_, AppState>,
    id: String,
    input: PayablePaymentInput,
) -> Result<Payable, String> {
    state
        .write(|conn| payables::pay(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_payable_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<Payable, String> {
    state
        .write(|conn| payables::set_status(conn, &id, &status))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_payable(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| payables::delete(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_deposits(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<Deposit>, String> {
    state
        .read_or(|conn| deposits::list(conn, status.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_deposit(state: State<'_, AppState>, id: String) -> Result<Deposit, String> {
    state
        .read(|conn| deposits::get(conn, &id))?
        .ok_or_else(|| "El depósito no existe.".to_string())
}

#[tauri::command]
pub fn create_deposit(
    state: State<'_, AppState>,
    input: DepositInput,
) -> Result<Deposit, String> {
    state
        .write(|conn| {
            let currency = settings::default_currency(conn);
            deposits::create(conn, &input, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_deposit(
    state: State<'_, AppState>,
    id: String,
    input: DepositInput,
) -> Result<Deposit, String> {
    state
        .write(|conn| deposits::update(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_deposit(
    state: State<'_, AppState>,
    id: String,
    input: DepositCompleteInput,
) -> Result<Deposit, String> {
    state
        .write(|conn| deposits::complete(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_deposit_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<Deposit, String> {
    state
        .write(|conn| deposits::set_status(conn, &id, &status))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_deposit(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| deposits::delete(conn, &id))
        .map_err(|e| e.to_string())
}
