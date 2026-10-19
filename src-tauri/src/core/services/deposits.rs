//! Servicio de depósitos y transferencias pendientes.
//!
//! Crear un depósito NO altera ningún saldo. Sólo al completarlo se puede
//! registrar el movimiento financiero correspondiente.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{
    Deposit, DepositCompleteInput, DepositInput, ReceivableInput, TransactionInput,
};
use crate::core::repos;
use crate::core::services::{self, new_id, receivables as receivable_service, transactions as tx_service};

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Deposit>> {
    repos::deposits::list(conn, status)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Deposit> {
    repos::deposits::get(conn, id)
}

pub fn create(conn: &Connection, input: &DepositInput, default_currency: &str) -> AppResult<Deposit> {
    let title = services::text(Some(&input.title), "el motivo")?;
    let amount = services::positive(input.amount)?;
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
    )?;
    let created_date = dates::format_date(dates::parse_date(&input.created_date)?);
    let due_date = match input.due_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => None,
    };
    let account_id = services::ensure_account(conn, input.account_id.as_deref())?;

    let id = new_id();
    repos::deposits::insert(
        conn,
        &id,
        &DepositInput {
            contact_id,
            contact_name: None,
            title,
            amount,
            currency_code: currency,
            created_date,
            due_date,
            account_id,
            destination_institution: services::optional_text(input.destination_institution.clone()),
            destination_details: services::optional_text(input.destination_details.clone()),
            payment_method: services::optional_text(input.payment_method.clone()),
            notes: services::optional_text(input.notes.clone()),
        },
        &dates::now_str(),
    )?;
    repos::deposits::get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &DepositInput) -> AppResult<Deposit> {
    let current = repos::deposits::get(conn, id)?;
    if current.status == "completed" {
        return Err(AppError::conflict(
            "El depósito ya está completado y no se puede editar.",
        ));
    }
    let title = services::text(Some(&input.title), "el motivo")?;
    let amount = services::positive(input.amount)?;
    let currency = services::ensure_currency(conn, &input.currency_code)?;
    let contact_id = services::resolve_contact(
        conn,
        input.contact_id.as_deref(),
        input.contact_name.as_deref(),
        "person",
    )?;
    let created_date = dates::format_date(dates::parse_date(&input.created_date)?);
    let due_date = match input.due_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => None,
    };
    let account_id = services::ensure_account(conn, input.account_id.as_deref())?;

    repos::deposits::update(
        conn,
        id,
        &DepositInput {
            contact_id,
            contact_name: None,
            title,
            amount,
            currency_code: currency,
            created_date,
            due_date,
            account_id,
            destination_institution: services::optional_text(input.destination_institution.clone()),
            destination_details: services::optional_text(input.destination_details.clone()),
            payment_method: services::optional_text(input.payment_method.clone()),
            notes: services::optional_text(input.notes.clone()),
        },
    )?;
    repos::deposits::get(conn, id)
}

/// Marca el depósito como completado. Opcionalmente registra el movimiento
/// financiero real y, si corresponde, la cuenta por cobrar del dinero enviado.
pub fn complete(
    conn: &mut Connection,
    deposit_id: &str,
    input: &DepositCompleteInput,
) -> AppResult<Deposit> {
    let completed_at = match input.completed_at.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => dates::format_date(dates::parse_date(d)?),
        None => dates::today_str(),
    };
    let tx = conn.transaction().map_err(AppError::from_sqlite)?;

    let deposit = repos::deposits::get(&tx, deposit_id)?;
    if deposit.status == "completed" {
        return Err(AppError::conflict("El depósito ya está completado."));
    }

    let account_id = match input.account_id.as_deref().filter(|a| !a.trim().is_empty()) {
        Some(a) => Some(a.to_string()),
        None => deposit.account_id.clone(),
    };
    let account_id = services::ensure_account(&tx, account_id.as_deref())?;

    let mut transaction_id: Option<String> = None;
    if input.register_transaction.unwrap_or(false) {
        let transaction_type = input
            .transaction_type
            .as_deref()
            .unwrap_or("expense")
            .to_string();
        let contact_id = match input.contact_id.as_deref().filter(|c| !c.trim().is_empty()) {
            Some(c) => services::ensure_contact(&tx, Some(c))?,
            None => deposit.contact_id.clone(),
        };
        let category_id = services::ensure_category(&tx, input.category_id.as_deref(), Some("expense"))?;

        let (kind, transfer_account_id) = match transaction_type.as_str() {
            "transfer" => {
                let destination = services::ensure_account(&tx, input.transfer_account_id.as_deref())?
                    .ok_or_else(|| {
                        AppError::validation(
                            "Indica la cuenta de destino de la transferencia.",
                        )
                    })?;
                if let Some(origin) = &account_id {
                    if origin == &destination {
                        return Err(AppError::validation(
                            "El origen y el destino de la transferencia deben ser distintas.",
                        ));
                    }
                }
                ("transfer", Some(destination))
            }
            _ => ("expense", None),
        };

        let transaction = tx_service::create_linked(
            &tx,
            &TransactionInput {
                transaction_type: kind.to_string(),
                description: deposit.title.clone(),
                amount: deposit.amount,
                currency_code: deposit.currency_code.clone(),
                date: completed_at.clone(),
                category_id: if kind == "expense" { category_id } else { None },
                account_id: account_id.clone(),
                transfer_account_id,
                contact_id: if kind == "expense" { contact_id } else { None },
                payment_method: input
                    .payment_method
                    .clone()
                    .or_else(|| deposit.payment_method.clone()),
                notes: input.notes.clone().or_else(|| deposit.notes.clone()),
                tags: None,
                related_transaction_id: None,
            },
            "deposit",
            deposit_id,
        )?;
        transaction_id = Some(transaction.id);
    }

    // Si el dinero enviado debe ser devuelto, se deja registrado como cuenta
    // por cobrar para no perder el seguimiento.
    let mut receivable_id = input
        .receivable_id
        .as_ref()
        .filter(|v| !v.trim().is_empty())
        .cloned();
    if receivable_id.is_none() {
        if let Some(contact_id) = deposit.contact_id.as_ref() {
            let created = receivable_service::create(
                &tx,
                &ReceivableInput {
                    contact_id: Some(contact_id.clone()),
                    contact_name: None,
                    title: format!("Devolución de {}", deposit.title),
                    kind: Some("debt".to_string()),
                    total_amount: deposit.amount,
                    currency_code: deposit.currency_code.clone(),
                    origin_date: completed_at.clone(),
                    due_date: None,
                    notes: Some(format!("Dinero enviado el {}", completed_at)),
                },
                &deposit.currency_code,
            )?;
            receivable_id = Some(created.id);
        }
    }

    repos::deposits::complete(
        &tx,
        deposit_id,
        "completed",
        Some(&completed_at),
        transaction_id.as_deref(),
        receivable_id.as_deref(),
    )?;

    tx.commit().map_err(AppError::from_sqlite)?;
    repos::deposits::get(conn, deposit_id)
}

/// Cambia el estado sin registrar movimientos (programado, cancelado, etc.).
pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<Deposit> {
    let valid = ["pending", "scheduled", "completed", "overdue", "cancelled"];
    if !valid.contains(&status) {
        return Err(AppError::validation("El estado indicado no es válido."));
    }
    if status == "completed" {
        return Err(AppError::validation(
            "Usa la acción de completar para registrar el movimiento correspondiente.",
        ));
    }
    repos::deposits::set_status(conn, id, status)?;
    repos::deposits::get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let deposit = repos::deposits::get(conn, id)?;
    if deposit.status == "completed" {
        return Err(AppError::conflict(
            "El depósito ya está completado y no se puede eliminar.",
        ));
    }
    repos::deposits::delete(conn, id)
}
