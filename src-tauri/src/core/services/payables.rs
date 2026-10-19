//! Servicio de cuentas por pagar.
//!
//! Registrar un pago genera el movimiento de gasto correspondiente. No se
//! duplican gastos: si la obligación ya está vinculada a una compra o cuota,
//! el pago de esa cuota se registra desde su propio módulo.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{Payable, PayableInput, PayablePayment, PayablePaymentInput, TransactionInput};
use crate::core::repos;
use crate::core::services::{self, new_id, transactions as tx_service};

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Payable>> {
    repos::payables::list(conn, status)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Payable> {
    repos::payables::get(conn, id)
}

pub fn list_payments(conn: &Connection, payable_id: &str) -> AppResult<Vec<PayablePayment>> {
    repos::payables::list_payments(conn, payable_id)
}

pub fn create(conn: &Connection, input: &PayableInput, default_currency: &str) -> AppResult<Payable> {
    let title = services::text(Some(&input.title), "la descripción")?;
    let total = services::positive(input.total_amount)?;
    let currency = if input.currency_code.trim().is_empty() {
        default_currency.to_string()
    } else {
        services::ensure_currency(conn, &input.currency_code)?
    };
    let contact_id = services::resolve_contact(
        conn,
        input.contact_id.as_deref(),
        input.contact_name.as_deref(),
        "person",
    )?
    .ok_or_else(|| AppError::validation("Indica a quién debes pagar."))?;
    let created_date = dates::format_date(dates::parse_date(&input.created_date)?);
    let due_date = match input.due_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => None,
    };
    let account_id = services::ensure_account(conn, input.account_id.as_deref())?;

    let id = new_id();
    repos::payables::insert(
        conn,
        &id,
        &PayableInput {
            contact_id: Some(contact_id),
            contact_name: None,
            title,
            total_amount: total,
            currency_code: currency,
            created_date,
            due_date,
            account_id,
            notes: services::optional_text(input.notes.clone()),
        },
        &dates::now_str(),
    )?;
    repos::payables::get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &PayableInput) -> AppResult<Payable> {
    let current = repos::payables::get(conn, id)?;
    let title = services::text(Some(&input.title), "la descripción")?;
    let total = services::positive(input.total_amount)?;
    if total < current.paid_amount {
        return Err(AppError::validation(
            "El total no puede ser menor que lo ya pagado.",
        ));
    }
    let currency = services::ensure_currency(conn, &input.currency_code)?;
    let contact_id = services::resolve_contact(
        conn,
        input.contact_id.as_deref(),
        input.contact_name.as_deref(),
        "person",
    )?
    .ok_or_else(|| AppError::validation("Indica a quién debes pagar."))?;
    let created_date = dates::format_date(dates::parse_date(&input.created_date)?);
    let due_date = match input.due_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => None,
    };
    let account_id = services::ensure_account(conn, input.account_id.as_deref())?;

    repos::payables::update(
        conn,
        id,
        &PayableInput {
            contact_id: Some(contact_id),
            contact_name: None,
            title,
            total_amount: total,
            currency_code: currency,
            created_date,
            due_date,
            account_id,
            notes: services::optional_text(input.notes.clone()),
        },
    )?;
    repos::payables::get(conn, id)
}

/// Registra el pago de la obligación y genera el gasto correspondiente.
pub fn pay(
    conn: &mut Connection,
    payable_id: &str,
    input: &PayablePaymentInput,
) -> AppResult<Payable> {
    let amount = services::positive(input.amount)?;
    let paid_at = dates::format_date(dates::parse_date(&input.paid_at)?);
    let tx = conn.transaction().map_err(AppError::from_sqlite)?;

    let payable = repos::payables::get(&tx, payable_id)?;
    if payable.status == "cancelled" {
        return Err(AppError::conflict("La cuenta por pagar está cancelada."));
    }
    let pending = payable.total_amount - payable.paid_amount;
    if pending <= 0 {
        return Err(AppError::conflict("Esta cuenta por pagar ya está saldada."));
    }
    if amount > pending {
        return Err(AppError::validation("El pago supera el saldo pendiente."));
    }

    let account_id = match input.account_id.as_deref().filter(|a| !a.trim().is_empty()) {
        Some(a) => Some(a.to_string()),
        None => payable.account_id.clone(),
    };
    let account_id = services::ensure_account(&tx, account_id.as_deref())?;

    let transaction = tx_service::create_linked(
        &tx,
        &TransactionInput {
            transaction_type: "expense".to_string(),
            description: payable.title.clone(),
            amount,
            currency_code: payable.currency_code.clone(),
            date: paid_at.clone(),
            category_id: None,
            account_id: account_id.clone(),
            transfer_account_id: None,
            contact_id: Some(payable.contact_id.clone()),
            payment_method: None,
            notes: None,
            tags: None,
            related_transaction_id: None,
        },
        "payable",
        payable_id,
    )?;

    repos::payables::insert_payment(
        &tx,
        &new_id(),
        payable_id,
        amount,
        &paid_at,
        account_id.as_deref(),
        Some(&transaction.id),
        &dates::now_str(),
    )?;

    let new_paid = payable.paid_amount + amount;
    let status = if new_paid >= payable.total_amount {
        "paid"
    } else if payable
        .due_date
        .as_deref()
        .map(|d| d < dates::today_str().as_str())
        .unwrap_or(false)
    {
        // Un pago parcial no saca la obligación de la categoría «vencida».
        "overdue"
    } else {
        "partial"
    };
    repos::payables::apply_payment(&tx, payable_id, new_paid, status)?;

    tx.commit().map_err(AppError::from_sqlite)?;
    repos::payables::get(conn, payable_id)
}

pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<Payable> {
    let valid = ["pending", "partial", "paid", "cancelled", "overdue"];
    if !valid.contains(&status) {
        return Err(AppError::validation("El estado indicado no es válido."));
    }
    let current = repos::payables::get(conn, id)?;
    if status == "paid" && current.total_amount - current.paid_amount > 0 {
        // Marcar «pagado» sin registrar el pago dejaría la fila incoherente.
        return Err(AppError::validation(
            "Para marcar como pagado registra el pago: así el saldo pendiente queda en cero.",
        ));
    }
    repos::payables::set_status(conn, id, status)?;
    repos::payables::get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repos::payables::delete(conn, id)
}
