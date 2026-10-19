//! Servicio de cuentas por cobrar.
//!
//! El dinero por cobrar no se considera disponible hasta que se recibe.
//! Registrar un cobro actualiza el saldo pendiente y crea el ingreso en la
//! misma transacción.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{Receivable, ReceivableInput, ReceivablePayment, ReceivablePaymentInput, TransactionInput};
use crate::core::repos;
use crate::core::services::{self, new_id, transactions as tx_service};

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Receivable>> {
    repos::receivables::list(conn, status)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Receivable> {
    repos::receivables::get(conn, id)
}

pub fn list_payments(conn: &Connection, receivable_id: &str) -> AppResult<Vec<ReceivablePayment>> {
    repos::receivables::list_payments(conn, receivable_id)
}

pub fn create(conn: &Connection, input: &ReceivableInput, default_currency: &str) -> AppResult<Receivable> {
    let title = services::text(Some(&input.title), "el motivo")?;
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
    .ok_or_else(|| AppError::validation("Indica a quién le debes cobrar."))?;
    let origin_date = dates::format_date(dates::parse_date(&input.origin_date)?);
    let due_date = match input.due_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => None,
    };
    let kind = match input.kind.as_deref() {
        Some("advance") => "advance".to_string(),
        Some("refund") => "refund".to_string(),
        _ => "debt".to_string(),
    };

    let id = new_id();
    repos::receivables::insert(
        conn,
        &id,
        &ReceivableInput {
            contact_id: Some(contact_id),
            contact_name: None,
            title,
            kind: Some(kind),
            total_amount: total,
            currency_code: currency,
            origin_date,
            due_date,
            notes: services::optional_text(input.notes.clone()),
        },
        &dates::now_str(),
    )?;
    repos::receivables::get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &ReceivableInput) -> AppResult<Receivable> {
    let current = repos::receivables::get(conn, id)?;
    let title = services::text(Some(&input.title), "el motivo")?;
    let total = services::positive(input.total_amount)?;
    if total < current.received_amount {
        return Err(AppError::validation(
            "El total no puede ser menor que lo ya recibido.",
        ));
    }
    let currency = services::ensure_currency(conn, &input.currency_code)?;
    let contact_id = services::resolve_contact(
        conn,
        input.contact_id.as_deref(),
        input.contact_name.as_deref(),
        "person",
    )?
    .ok_or_else(|| AppError::validation("Indica a quién le debes cobrar."))?;
    let origin_date = dates::format_date(dates::parse_date(&input.origin_date)?);
    let due_date = match input.due_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => None,
    };

    repos::receivables::update(
        conn,
        id,
        &ReceivableInput {
            contact_id: Some(contact_id),
            contact_name: None,
            title,
            kind: input.kind.clone(),
            total_amount: total,
            currency_code: currency,
            origin_date,
            due_date,
            notes: services::optional_text(input.notes.clone()),
        },
    )?;
    repos::receivables::get(conn, id)
}

/// Registra el cobro: actualiza el saldo pendiente y crea el ingreso.
pub fn receive(
    conn: &mut Connection,
    receivable_id: &str,
    input: &ReceivablePaymentInput,
) -> AppResult<Receivable> {
    let amount = services::positive(input.amount)?;
    let received_at = dates::format_date(dates::parse_date(&input.received_at)?);
    let tx = conn.transaction().map_err(AppError::from_sqlite)?;

    let receivable = repos::receivables::get(&tx, receivable_id)?;
    if receivable.status == "cancelled" {
        return Err(AppError::conflict("La cuenta por cobrar está cancelada."));
    }
    let pending = receivable.total_amount - receivable.received_amount;
    if pending <= 0 {
        return Err(AppError::conflict("Esta cuenta por cobrar ya está saldada."));
    }
    if amount > pending {
        return Err(AppError::validation("El cobro supera el saldo pendiente."));
    }

    let account_id = services::ensure_account(&tx, input.account_id.as_deref())?;
    let transaction = tx_service::create_linked(
        &tx,
        &TransactionInput {
            transaction_type: "income".to_string(),
            description: format!("Cobro · {}", receivable.title),
            amount,
            currency_code: receivable.currency_code.clone(),
            date: received_at.clone(),
            category_id: None,
            account_id: account_id.clone(),
            transfer_account_id: None,
            contact_id: Some(receivable.contact_id.clone()),
            payment_method: None,
            notes: None,
            tags: None,
            related_transaction_id: None,
        },
        "receivable",
        receivable_id,
    )?;

    repos::receivables::insert_payment(
        &tx,
        &new_id(),
        receivable_id,
        amount,
        &received_at,
        account_id.as_deref(),
        Some(&transaction.id),
        &dates::now_str(),
    )?;

    let new_received = receivable.received_amount + amount;
    let status = if new_received >= receivable.total_amount {
        "paid"
    } else {
        "partial"
    };
    repos::receivables::apply_payment(&tx, receivable_id, new_received, status)?;

    tx.commit().map_err(AppError::from_sqlite)?;
    repos::receivables::get(conn, receivable_id)
}

pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<Receivable> {
    let valid = ["pending", "partial", "paid", "cancelled", "overdue"];
    if !valid.contains(&status) {
        return Err(AppError::validation("El estado indicado no es válido."));
    }
    repos::receivables::set_status(conn, id, status)?;
    repos::receivables::get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repos::receivables::delete(conn, id)
}
