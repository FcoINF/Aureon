//! Servicio de compras y cuotas.
//!
//! Reglas principales:
//! * Crear una compra no genera gastos: sólo genera el calendario de cuotas.
//! * Una cuota pagada genera un único movimiento de gasto, enlazado a la cuota.
//! * El pago de una cuota y su movimiento se escriben en la misma transacción.

use rusqlite::Connection;

use crate::core::dates::{self, Frequency};
use crate::core::error::{AppError, AppResult};
use crate::core::models::{
    Installment, InstallmentPaymentInput, Purchase, PurchaseInput, TransactionInput,
};
use crate::core::repos;
use crate::core::services::{self, new_id, transactions as tx_service};

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Purchase>> {
    repos::purchases::list(conn, status)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Purchase> {
    repos::purchases::get(conn, id)
}

pub fn list_installments(conn: &Connection, purchase_id: &str) -> AppResult<Vec<Installment>> {
    repos::purchases::list_installments(conn, purchase_id)
}

pub fn get_installment(conn: &Connection, id: &str) -> AppResult<Installment> {
    repos::purchases::get_installment(conn, id)
}

/// Reparte el total en cuotas. Los importes que no se dividen de forma exacta
/// se reparten de a uno entre las primeras cuotas, de modo que la suma siempre
/// coincide exactamente con el total de la compra.
pub fn split_amount(total: i64, count: i64) -> Vec<i64> {
    if count <= 0 {
        return vec![total.max(0)];
    }
    let count = count as usize;
    let base = total / count as i64;
    let remainder = total % count as i64;
    (0..count)
        .map(|i| if (i as i64) < remainder { base + 1 } else { base })
        .collect()
}

/// Calcula las fechas de vencimiento del calendario de cuotas.
pub fn schedule(first_due: chrono::NaiveDate, count: i64, frequency: &Frequency) -> Vec<String> {
    let mut dates = Vec::new();
    let mut current = first_due;
    for _ in 0..count.max(0) {
        dates.push(dates::format_date(current));
        current = frequency.advance(current);
    }
    dates
}

struct Plan {
    name: String,
    purchase_date: String,
    total: i64,
    currency: String,
    count: i64,
    amounts: Vec<i64>,
    due_dates: Vec<String>,
    frequency: String,
    frequency_days: Option<i64>,
    first_due_date: Option<String>,
    account_id: Option<String>,
    category_id: Option<String>,
    contact_id: Option<String>,
    url: Option<String>,
    payment_method: Option<String>,
    notes: Option<String>,
    paid_immediately: bool,
}

fn prepare(
    conn: &Connection,
    input: &PurchaseInput,
    default_currency: &str,
) -> AppResult<Plan> {
    let name = services::text(Some(&input.name), "el nombre de la compra")?;
    let total = services::positive(input.total_amount)?;
    let currency = if input.currency_code.trim().is_empty() {
        default_currency.to_string()
    } else {
        services::ensure_currency(conn, &input.currency_code)?
    };
    let purchase_date = dates::format_date(dates::parse_date(&input.purchase_date)?);
    let count = input.installments_count.max(1);
    let frequency_name = input
        .frequency
        .clone()
        .unwrap_or_else(|| "monthly".to_string());
    let frequency = Frequency::from_str(&frequency_name, input.frequency_days)?;

    let amounts = match input.installment_amount {
        Some(a) if a > 0 => {
            // Se respeta el importe de cuota indicado, ajustando la última.
            let mut amounts = vec![a; count as usize];
            let sum: i64 = amounts.iter().sum();
            let diff = total - sum;
            if diff != 0 {
                let last = amounts.len() - 1;
                let new_last = amounts[last] + diff;
                if new_last < 0 {
                    return Err(AppError::validation(
                        "El importe de cada cuota no puede ser mayor que el total de la compra.",
                    ));
                }
                amounts[last] = new_last;
            }
            amounts
        }
        _ => split_amount(total, count),
    };

    let first_due = match input.first_due_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => dates::format_date(dates::parse_date(d)?),
        None => {
            if count == 1 {
                purchase_date.clone()
            } else {
                return Err(AppError::validation(
                    "Indica la fecha de vencimiento de la primera cuota.",
                ));
            }
        }
    };

    let first_due_date = chrono::NaiveDate::parse_from_str(&first_due, "%Y-%m-%d")
        .map_err(|_| AppError::validation("La fecha de la primera cuota no es válida."))?;

    let paid_immediately = input.paid_immediately.unwrap_or(count == 1);
    if paid_immediately && count > 1 {
        return Err(AppError::validation(
            "Una compra en cuotas genera sus vencimientos. Registra cada pago cuando ocurra.",
        ));
    }

    let account_id = services::ensure_account(conn, input.account_id.as_deref())?;
    let category_id = services::ensure_category(conn, input.category_id.as_deref(), Some("expense"))?;
    let contact_id = services::resolve_contact(
        conn,
        input.contact_id.as_deref(),
        input.contact_name.as_deref(),
        "merchant",
    )?;

    Ok(Plan {
        name,
        purchase_date,
        total,
        currency,
        count,
        amounts,
        due_dates: schedule(first_due_date, count, &frequency),
        frequency: frequency_name,
        frequency_days: input.frequency_days,
        first_due_date: Some(first_due),
        account_id,
        category_id,
        contact_id,
        url: services::optional_text(input.url.clone()),
        payment_method: services::optional_text(input.payment_method.clone()),
        notes: services::optional_text(input.notes.clone()),
        paid_immediately,
    })
}

/// Crea la compra y su calendario de cuotas. No genera ningún gasto, salvo
/// que la compra sea de una sola cuota y el usuario la marque como pagada.
pub fn create(conn: &mut Connection, input: &PurchaseInput, default_currency: &str) -> AppResult<Purchase> {
    let plan = prepare(conn, input, default_currency)?;
    let tx = conn.transaction().map_err(AppError::from_sqlite)?;

    let purchase_id = new_id();
    repos::purchases::insert(
        &tx,
        &purchase_id,
        &PurchaseInput {
            name: plan.name.clone(),
            contact_id: plan.contact_id.clone(),
            contact_name: None,
            url: plan.url.clone(),
            purchase_date: plan.purchase_date.clone(),
            total_amount: plan.total,
            currency_code: plan.currency.clone(),
            installments_count: plan.count,
            installment_amount: plan.amounts.first().copied(),
            first_due_date: plan.first_due_date.clone(),
            frequency: Some(plan.frequency.clone()),
            frequency_days: plan.frequency_days,
            payment_method: plan.payment_method.clone(),
            account_id: plan.account_id.clone(),
            category_id: plan.category_id.clone(),
            status: Some("active".to_string()),
            paid_immediately: None,
            notes: plan.notes.clone(),
        },
        &dates::now_str(),
    )?;

    let created_at = dates::now_str();
    for index in 0..plan.count as usize {
        let installment_id = new_id();
        let amount = plan.amounts[index];
        let due_date = plan.due_dates[index].clone();
        if plan.paid_immediately {
            repos::purchases::insert_installment(
                &tx,
                &installment_id,
                &purchase_id,
                index as i64 + 1,
                amount,
                &due_date,
                "paid",
            )?;
            let transaction = tx_service::create_linked(
                &tx,
                &TransactionInput {
                    transaction_type: "expense".to_string(),
                    description: plan.name.clone(),
                    amount,
                    currency_code: plan.currency.clone(),
                    date: plan.purchase_date.clone(),
                    category_id: plan.category_id.clone(),
                    account_id: plan.account_id.clone(),
                    transfer_account_id: None,
                    contact_id: plan.contact_id.clone(),
                    payment_method: plan.payment_method.clone(),
                    notes: plan.notes.clone(),
                    tags: None,
                    related_transaction_id: None,
                },
                "purchase",
                &purchase_id,
            )?;
            repos::purchases::apply_installment_payment(
                &tx,
                &installment_id,
                amount,
                "paid",
                Some(&plan.purchase_date),
                plan.account_id.as_deref(),
                Some(&transaction.id),
            )?;
            repos::purchases::insert_installment_payment(
                &tx,
                &new_id(),
                &installment_id,
                amount,
                &plan.purchase_date,
                plan.account_id.as_deref(),
                Some(&transaction.id),
                &created_at,
            )?;
        } else {
            repos::purchases::insert_installment(
                &tx,
                &installment_id,
                &purchase_id,
                index as i64 + 1,
                amount,
                &due_date,
                "pending",
            )?;
        }
    }

    if plan.paid_immediately {
        repos::purchases::set_status(&tx, &purchase_id, "completed")?;
    }

    tx.commit().map_err(AppError::from_sqlite)?;
    repos::purchases::get(conn, &purchase_id)
}

/// Registra el pago de una cuota (total o parcial) y el gasto correspondiente.
pub fn pay_installment(
    conn: &mut Connection,
    installment_id: &str,
    input: &InstallmentPaymentInput,
) -> AppResult<Installment> {
    let amount = services::positive(input.amount)?;
    let paid_at = dates::format_date(dates::parse_date(&input.paid_at)?);

    let tx = conn.transaction().map_err(AppError::from_sqlite)?;

    let installment = repos::purchases::get_installment(&tx, installment_id)?;
    if installment.status == "cancelled" {
        return Err(AppError::conflict("Esta cuota está cancelada."));
    }
    let purchase = repos::purchases::get(&tx, &installment.purchase_id)?;
    if purchase.status == "cancelled" {
        return Err(AppError::conflict("La compra está cancelada."));
    }
    let remaining = installment.amount - installment.paid_amount;
    if remaining <= 0 {
        return Err(AppError::conflict("Esta cuota ya está pagada."));
    }
    if amount > remaining {
        return Err(AppError::validation(
            "El pago supera el saldo pendiente de la cuota.",
        ));
    }

    let account_id = match input.account_id.as_deref().filter(|a| !a.trim().is_empty()) {
        Some(a) => Some(a.to_string()),
        None => purchase.account_id.clone(),
    };
    let account_id = services::ensure_account(&tx, account_id.as_deref())?;

    let transaction = tx_service::create_linked(
        &tx,
        &TransactionInput {
            transaction_type: "expense".to_string(),
            description: format!("{} · cuota {}", purchase.name, installment.number),
            amount,
            currency_code: purchase.currency_code.clone(),
            date: paid_at.clone(),
            category_id: None,
            account_id: account_id.clone(),
            transfer_account_id: None,
            contact_id: purchase.contact_id.clone(),
            payment_method: purchase.payment_method.clone(),
            notes: None,
            tags: None,
            related_transaction_id: None,
        },
        "installment",
        installment_id,
    )?;

    let new_paid = installment.paid_amount + amount;
    let status = if new_paid >= installment.amount {
        "paid"
    } else if installment.due_date.as_str() < paid_at.as_str() {
        "overdue"
    } else {
        "pending"
    };
    let paid_at_stored = if status == "paid" {
        Some(paid_at.as_str())
    } else {
        installment.paid_at.as_deref()
    };

    repos::purchases::apply_installment_payment(
        &tx,
        installment_id,
        new_paid,
        status,
        paid_at_stored,
        account_id.as_deref(),
        Some(&transaction.id),
    )?;
    repos::purchases::insert_installment_payment(
        &tx,
        &new_id(),
        installment_id,
        amount,
        &paid_at,
        account_id.as_deref(),
        Some(&transaction.id),
        &dates::now_str(),
    )?;

    // Si ya no quedan cuotas pendientes, la compra queda completada.
    let pending: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM purchase_installments
              WHERE purchase_id = ?1 AND status IN ('pending', 'overdue')",
            [&purchase.id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    if pending == 0 {
        repos::purchases::set_status(&tx, &purchase.id, "completed")?;
    }

    tx.commit().map_err(AppError::from_sqlite)?;
    repos::purchases::get_installment(conn, installment_id)
}

/// Modifica una cuota que todavía no tiene pagos.
pub fn update_installment(
    conn: &Connection,
    installment_id: &str,
    amount: i64,
    due_date: &str,
) -> AppResult<Installment> {
    let installment = repos::purchases::get_installment(conn, installment_id)?;
    if installment.paid_amount > 0 {
        return Err(AppError::conflict(
            "No se puede modificar una cuota que ya tiene pagos registrados.",
        ));
    }
    let amount = services::positive(amount)?;
    let due_date = dates::format_date(dates::parse_date(due_date)?);
    let status = if installment.due_date.as_str() < due_date.as_str() {
        "pending"
    } else {
        installment.status.as_str()
    };
    repos::purchases::update_installment(conn, installment_id, amount, &due_date, status)?;
    repos::purchases::get_installment(conn, installment_id)
}

/// Cancela la compra y sus cuotas pendientes. El historial se conserva.
pub fn cancel(conn: &Connection, purchase_id: &str) -> AppResult<Purchase> {
    let purchase = repos::purchases::get(conn, purchase_id)?;
    if purchase.status == "cancelled" {
        return Ok(purchase);
    }
    repos::purchases::cancel_pending_installments(conn, purchase_id)?;
    repos::purchases::set_status(conn, purchase_id, "cancelled")?;
    repos::purchases::get(conn, purchase_id)
}

/// Registra la devolución de una compra: genera un ingreso enlazado a ella.
pub fn refund(
    conn: &mut Connection,
    purchase_id: &str,
    amount: i64,
    date: &str,
    account_id: Option<String>,
) -> AppResult<Purchase> {
    let amount = services::positive(amount)?;
    let date = dates::format_date(dates::parse_date(date)?);
    let tx = conn.transaction().map_err(AppError::from_sqlite)?;

    let purchase = repos::purchases::get(&tx, purchase_id)?;
    if purchase.status == "cancelled" {
        return Err(AppError::conflict("La compra está cancelada."));
    }
    let refunded: i64 = tx
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions
              WHERE type = 'refund' AND voided = 0 AND linked_type = 'purchase' AND linked_id = ?1",
            [purchase_id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    if refunded + amount > purchase.paid_amount {
        return Err(AppError::validation(
            "La devolución supera el monto ya pagado de la compra.",
        ));
    }
    let account_id = services::ensure_account(&tx, account_id.as_deref())?;
    let account_id = match account_id {
        Some(a) => Some(a),
        None => purchase.account_id.clone(),
    };

    tx_service::create_linked(
        &tx,
        &TransactionInput {
            transaction_type: "refund".to_string(),
            description: format!("Devolución · {}", purchase.name),
            amount,
            currency_code: purchase.currency_code.clone(),
            date: date.clone(),
            category_id: None,
            account_id,
            transfer_account_id: None,
            contact_id: purchase.contact_id.clone(),
            payment_method: None,
            notes: None,
            tags: None,
            related_transaction_id: None,
        },
        "purchase",
        purchase_id,
    )?;

    repos::purchases::set_status(&tx, purchase_id, "refunded")?;
    tx.commit().map_err(AppError::from_sqlite)?;
    repos::purchases::get(conn, purchase_id)
}
