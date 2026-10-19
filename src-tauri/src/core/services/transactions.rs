//! Servicio de movimientos financieros.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{Transaction, TransactionFilter, TransactionInput};
use crate::core::repos;
use crate::core::services::{self, new_id};

const TYPES: [&str; 4] = ["income", "expense", "transfer", "refund"];

/// Si el filtro trae un período predefinido y no un rango explícito, lo
/// convierte a fechas de inicio y fin.
pub fn resolve_period(filter: &TransactionFilter) -> AppResult<TransactionFilter> {
    let mut resolved = filter.clone();
    let has_range = resolved
        .start_date
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        || resolved
            .end_date
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
    if !has_range {
        if let Some(period) = resolved.period.as_ref().filter(|p| !p.is_empty()) {
            let (start, end) = dates::period_range(period, 0)?;
            resolved.start_date = Some(start);
            resolved.end_date = Some(end);
        }
    }
    Ok(resolved)
}

pub fn list(conn: &Connection, filter: &TransactionFilter) -> AppResult<Vec<Transaction>> {
    let filter = &resolve_period(filter)?;
    repos::transactions::list(conn, filter)
}

pub fn count(conn: &Connection, filter: &TransactionFilter) -> AppResult<i64> {
    let filter = &resolve_period(filter)?;
    repos::transactions::count(conn, filter)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Transaction> {
    repos::transactions::get(conn, id)
}

struct Normalized {
    input: TransactionInput,
}

fn normalize(conn: &Connection, input: &TransactionInput) -> AppResult<Normalized> {
    if !TYPES.contains(&input.transaction_type.as_str()) {
        return Err(AppError::validation("El tipo de movimiento no es válido."));
    }
    let description = services::text(Some(&input.description), "la descripción")?;
    let amount = services::positive(input.amount)?;
    let currency = services::ensure_currency(conn, &input.currency_code)?;
    let date = dates::format_date(dates::parse_date(&input.date)?);

    let account_id = services::ensure_account_in_currency(conn, input.account_id.as_deref(), &currency)?;
    let transfer_account_id =
        services::ensure_account_in_currency(conn, input.transfer_account_id.as_deref(), &currency)?;
    let contact_id = services::ensure_contact(conn, input.contact_id.as_deref())?;

    let mut category_id = services::ensure_category(conn, input.category_id.as_deref(), None)?;

    match input.transaction_type.as_str() {
        "income" => {
            if account_id.is_none() {
                return Err(AppError::validation("Indica la cuenta donde se recibe el dinero."));
            }
            if let Some(cat) = category_id.as_deref() {
                let category = repos::categories::get(conn, cat, ("0000-00-00", "9999-99-99"), None)?;
                if category.kind != "income" {
                    return Err(AppError::validation(
                        "La categoría seleccionada no es de ingresos.",
                    ));
                }
            }
        }
        "expense" => {
            if account_id.is_none() {
                return Err(AppError::validation("Indica la cuenta desde donde se paga."));
            }
            if let Some(cat) = category_id.as_deref() {
                let category = repos::categories::get(conn, cat, ("0000-00-00", "9999-99-99"), None)?;
                if category.kind != "expense" {
                    return Err(AppError::validation(
                        "La categoría seleccionada no es de gastos.",
                    ));
                }
            }
        }
        "transfer" => {
            let origin = account_id.clone();
            let destination = transfer_account_id.clone();
            match (origin, destination) {
                (Some(o), Some(d)) if o != d => {}
                (Some(_), Some(_)) => {
                    return Err(AppError::validation(
                        "Una transferencia debe ir entre dos cuentas distintas.",
                    ))
                }
                _ => {
                    return Err(AppError::validation(
                        "Indica la cuenta de origen y la cuenta de destino.",
                    ))
                }
            }
            // Una transferencia no lleva categoría: no es un gasto ni un ingreso.
            category_id = None;
        }
        "refund" => {
            if account_id.is_none() {
                return Err(AppError::validation("Indica la cuenta donde vuelve el dinero."));
            }
            if let Some(cat) = category_id.as_deref() {
                let category = repos::categories::get(conn, cat, ("0000-00-00", "9999-99-99"), None)?;
                if category.kind != "income" {
                    return Err(AppError::validation(
                        "La categoría seleccionada no es de ingresos.",
                    ));
                }
            }
        }
        _ => {}
    }

    Ok(Normalized {
        input: TransactionInput {
            transaction_type: input.transaction_type.clone(),
            description,
            amount,
            currency_code: currency.clone(),
            date: date.clone(),
            category_id,
            account_id,
            transfer_account_id,
            contact_id,
            payment_method: services::optional_text(input.payment_method.clone()),
            notes: services::optional_text(input.notes.clone()),
            tags: services::optional_text(input.tags.clone()),
            related_transaction_id: input
                .related_transaction_id
                .as_ref()
                .filter(|v| !v.trim().is_empty())
                .cloned(),
        },
    })
}

/// Crea un movimiento validado.
pub fn create(conn: &Connection, input: &TransactionInput) -> AppResult<Transaction> {
    let normalized = normalize(conn, input)?;
    if normalized.input.transaction_type == "refund" {
        validate_refund(conn, &normalized.input)?;
    }
    let id = new_id();
    repos::transactions::insert(
        conn,
        &id,
        &normalized.input,
        None,
        None,
        &dates::now_str(),
    )?;
    repos::transactions::get(conn, &id)
}

/// Crea un movimiento vinculado a otro registro (cuota, suscripción, etc.).
pub fn create_linked(
    conn: &Connection,
    input: &TransactionInput,
    linked_type: &str,
    linked_id: &str,
) -> AppResult<Transaction> {
    let normalized = normalize(conn, input)?;
    let id = new_id();
    repos::transactions::insert(
        conn,
        &id,
        &normalized.input,
        Some(linked_type),
        Some(linked_id),
        &dates::now_str(),
    )?;
    repos::transactions::get(conn, &id)
}

/// Una devolución no puede superar lo que se.original gastó.
fn validate_refund(conn: &Connection, input: &TransactionInput) -> AppResult<()> {
    let original_id = input
        .related_transaction_id
        .as_ref()
        .ok_or_else(|| AppError::validation("Indica el movimiento original al que corresponde la devolución."))?;
    let original = repos::transactions::get(conn, original_id)?;
    if original.transaction_type != "expense" {
        return Err(AppError::validation(
            "La devolución debe relacionarse con un gasto.",
        ));
    }
    if original.voided {
        return Err(AppError::validation(
            "No se puede devolver un movimiento que está anulado.",
        ));
    }
    let refunded: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions
              WHERE type = 'refund' AND voided = 0 AND related_transaction_id = ?1",
            [original_id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    if refunded + input.amount > original.amount {
        return Err(AppError::validation(
            "El monto de la devolución supera el gasto original.",
        ));
    }
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &TransactionInput) -> AppResult<Transaction> {
    let current = repos::transactions::get(conn, id)?;
    if current.voided {
        return Err(AppError::conflict("No se puede editar un movimiento anulado."));
    }
    if current.linked_type.is_some() {
        return Err(AppError::conflict(
            "Este movimiento pertenece a otro módulo. Edítalo desde allí para mantener la coherencia.",
        ));
    }
    let normalized = normalize(conn, input)?;
    if normalized.input.transaction_type == "refund" {
        validate_refund(conn, &normalized.input)?;
    }
    repos::transactions::update(conn, id, &normalized.input)?;
    repos::transactions::get(conn, id)
}

/// Anula un movimiento (borrado lógico, conserva la auditoría).
pub fn void(conn: &Connection, id: &str) -> AppResult<Transaction> {
    let current = repos::transactions::get(conn, id)?;
    if current.linked_type.is_some() {
        return Err(AppError::conflict(
            "Este movimiento pertenece a otro módulo. Revierte la operación en ese módulo.",
        ));
    }
    repos::transactions::void(conn, id, &dates::now_str())?;
    repos::transactions::get(conn, id)
}

pub fn unvoid(conn: &Connection, id: &str) -> AppResult<Transaction> {
    repos::transactions::unvoid(conn, id)?;
    repos::transactions::get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repos::transactions::delete(conn, id)
}
