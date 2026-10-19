//! Servicio de suscripciones y cobros.
//!
//! Reglas:
//! * Crear una suscripción no genera ningún gasto.
//! * Registrar un cobro real genera un único movimiento de gasto.
//! * Pausar, cancelar o cambiar el precio nunca borra el historial de cobros.

use rusqlite::{params, Connection};

use crate::core::dates::{self, Frequency};
use crate::core::error::{AppError, AppResult};
use crate::core::models::{
    Subscription, SubscriptionInput, SubscriptionPayment, SubscriptionPaymentInput,
    SubscriptionShare, SubscriptionShareInput, TransactionInput,
};
use crate::core::repos;
use crate::core::services::{self, new_id, transactions as tx_service};

/// Costo mensual estimado, en aritmética entera.
pub fn monthly_estimate(amount: i64, frequency: &Frequency) -> i64 {
    let (num, den) = frequency.occurrences_ratio();
    if den == 0 {
        return 0;
    }
    let numerator = amount as i128 * num as i128;
    let denominator = 12i128 * den as i128;
    ((numerator + denominator / 2) / denominator) as i64
}

/// Costo anual estimado, en aritmética entera.
pub fn yearly_estimate(amount: i64, frequency: &Frequency) -> i64 {
    let (num, den) = frequency.occurrences_ratio();
    if den == 0 {
        return 0;
    }
    let numerator = amount as i128 * num as i128;
    ((numerator + (den as i128) / 2) / den as i128) as i64
}

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<Subscription>> {
    let mut subscriptions = repos::subscriptions::list(conn, status)?;
    for subscription in subscriptions.iter_mut() {
        let frequency = Frequency::from_str(&subscription.frequency, subscription.frequency_days)?;
        subscription.monthly_estimate = monthly_estimate(subscription.amount, &frequency);
        subscription.yearly_estimate = yearly_estimate(subscription.amount, &frequency);
    }
    Ok(subscriptions)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Subscription> {
    let mut subscription = repos::subscriptions::get(conn, id)?;
    let frequency = Frequency::from_str(&subscription.frequency, subscription.frequency_days)?;
    subscription.monthly_estimate = monthly_estimate(subscription.amount, &frequency);
    subscription.yearly_estimate = yearly_estimate(subscription.amount, &frequency);
    Ok(subscription)
}

pub fn list_payments(conn: &Connection, subscription_id: &str) -> AppResult<Vec<SubscriptionPayment>> {
    repos::subscriptions::list_payments(conn, subscription_id)
}

pub fn create(conn: &Connection, input: &SubscriptionInput, default_currency: &str) -> AppResult<Subscription> {
    let name = services::text(Some(&input.name), "el nombre del servicio")?;
    let amount = services::positive(input.amount)?;
    let currency = if input.currency_code.trim().is_empty() {
        default_currency.to_string()
    } else {
        services::ensure_currency(conn, &input.currency_code)?
    };
    let frequency = Frequency::from_str(&input.frequency, input.frequency_days)?;
    let start_date = dates::format_date(dates::parse_date(&input.start_date)?);
    let next_charge = match input.next_charge_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => Some(start_date.clone()),
    };
    let account_id = services::ensure_account(conn, input.account_id.as_deref())?;
    let category_id = services::ensure_category(conn, input.category_id.as_deref(), Some("expense"))?;
    let contact_id = services::resolve_contact(
        conn,
        input.contact_id.as_deref(),
        input.contact_name.as_deref(),
        "merchant",
    )?;
    let responsible = services::ensure_contact(conn, input.responsible_contact_id.as_deref())?;
    let status = match input.status.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => "active".to_string(),
    };
    let id = new_id();
    repos::subscriptions::insert(
        conn,
        &id,
        &SubscriptionInput {
            name,
            category_id,
            contact_id,
            contact_name: None,
            url: services::optional_text(input.url.clone()),
            amount,
            currency_code: currency,
            frequency: frequency.as_str().to_string(),
            frequency_days: input.frequency_days,
            start_date,
            next_charge_date: next_charge,
            account_id,
            payment_method: services::optional_text(input.payment_method.clone()),
            responsible_contact_id: responsible,
            status: Some(status),
            notes: services::optional_text(input.notes.clone()),
        },
        &dates::now_str(),
    )?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &SubscriptionInput) -> AppResult<Subscription> {
    let current = repos::subscriptions::get(conn, id)?;
    let name = services::text(Some(&input.name), "el nombre del servicio")?;
    let amount = services::positive(input.amount)?;
    let currency = services::ensure_currency(conn, &input.currency_code)?;
    let frequency = Frequency::from_str(&input.frequency, input.frequency_days)?;
    let start_date = dates::format_date(dates::parse_date(&input.start_date)?);
    let next_charge = match input.next_charge_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => current.next_charge_date.clone(),
    };
    let account_id = services::ensure_account(conn, input.account_id.as_deref())?;
    let category_id = services::ensure_category(conn, input.category_id.as_deref(), Some("expense"))?;
    let contact_id = services::resolve_contact(
        conn,
        input.contact_id.as_deref(),
        input.contact_name.as_deref(),
        "merchant",
    )?;
    let responsible = services::ensure_contact(conn, input.responsible_contact_id.as_deref())?;
    let status = match input.status.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => current.status.clone(),
    };

    repos::subscriptions::update(
        conn,
        id,
        &SubscriptionInput {
            name,
            category_id,
            contact_id,
            contact_name: None,
            url: services::optional_text(input.url.clone()),
            amount,
            currency_code: currency,
            frequency: frequency.as_str().to_string(),
            frequency_days: input.frequency_days,
            start_date,
            next_charge_date: next_charge,
            account_id,
            payment_method: services::optional_text(input.payment_method.clone()),
            responsible_contact_id: responsible,
            status: Some(status),
            notes: services::optional_text(input.notes.clone()),
        },
    )?;
    get(conn, id)
}

/// Registra un cobro real: crea el pago y su movimiento de gasto.
pub fn charge(
    conn: &mut Connection,
    subscription_id: &str,
    input: &SubscriptionPaymentInput,
) -> AppResult<SubscriptionPayment> {
    let charged_at = dates::format_date(dates::parse_date(&input.charged_at)?);
    let tx = conn.transaction().map_err(AppError::from_sqlite)?;

    let subscription = repos::subscriptions::get(&tx, subscription_id)?;
    if subscription.status == "cancelled" {
        return Err(AppError::conflict("La suscripción está cancelada."));
    }
    let amount = match input.amount {
        Some(a) => services::positive(a)?,
        None => subscription.amount,
    };
    let account_id = match input.account_id.as_deref().filter(|a| !a.trim().is_empty()) {
        Some(a) => Some(a.to_string()),
        None => subscription.account_id.clone(),
    };
    let account_id = services::ensure_account(&tx, account_id.as_deref())?;

    let frequency = Frequency::from_str(&subscription.frequency, subscription.frequency_days)?;
    let charged_date = dates::parse_date(&charged_at)?;
    let next_charge = match input.next_charge_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => Some(dates::format_date(dates::parse_date(d)?)),
        None => Some(dates::format_date(frequency.advance(charged_date))),
    };

    let transaction = tx_service::create_linked(
        &tx,
        &TransactionInput {
            transaction_type: "expense".to_string(),
            description: subscription.name.clone(),
            amount,
            currency_code: subscription.currency_code.clone(),
            date: charged_at.clone(),
            category_id: subscription.category_id.clone(),
            account_id: account_id.clone(),
            transfer_account_id: None,
            contact_id: subscription.contact_id.clone(),
            payment_method: subscription.payment_method.clone(),
            notes: None,
            tags: None,
            related_transaction_id: None,
        },
        "subscription",
        subscription_id,
    )?;

    let payment_id = new_id();
    repos::subscriptions::insert_payment(
        &tx,
        &payment_id,
        subscription_id,
        amount,
        &charged_at,
        next_charge.as_deref(),
        account_id.as_deref(),
        Some(&transaction.id),
        &dates::now_str(),
    )?;
    repos::subscriptions::set_next_charge(&tx, subscription_id, next_charge.as_deref())?;
    if subscription.status == "expired" {
        repos::subscriptions::set_status(&tx, subscription_id, "active", None)?;
    }

    tx.commit().map_err(AppError::from_sqlite)?;
    let payments = repos::subscriptions::list_payments(conn, subscription_id)?;
    payments
        .into_iter()
        .find(|p| p.id == payment_id)
        .ok_or_else(|| AppError::other("No se pudo registrar el cobro."))
}

/// Cambia el estado de la suscripción conservando todo el historial.
pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<Subscription> {
    let subscription = repos::subscriptions::get(conn, id)?;
    let now = dates::now_str();
    let cancelled_at = if status == "cancelled" {
        Some(now.as_str())
    } else {
        None
    };
    repos::subscriptions::set_status(conn, id, status, cancelled_at)?;

    if status == "active" && subscription.status != "active" {
        // Al reanudar se recalcula el próximo vencimiento.
        let frequency = Frequency::from_str(&subscription.frequency, subscription.frequency_days)?;
        let today = dates::today();
        let next = match subscription.next_charge_date.as_ref().and_then(|d| {
            chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
        }) {
            Some(existing) if existing > today => dates::format_date(existing),
            _ => dates::format_date(frequency.advance(today)),
        };
        repos::subscriptions::set_next_charge(conn, id, Some(&next))?;
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repos::subscriptions::delete(conn, id)
}

// ---------------------------------------------------------------------------
// Participación
// ---------------------------------------------------------------------------

pub fn list_shares(conn: &Connection, subscription_id: &str) -> AppResult<Vec<SubscriptionShare>> {
    repos::subscriptions::list_shares(conn, subscription_id)
}

pub fn add_share(
    conn: &Connection,
    subscription_id: &str,
    input: &SubscriptionShareInput,
) -> AppResult<SubscriptionShare> {
    repos::subscriptions::get(conn, subscription_id)?;
    let contact_id = services::ensure_contact(conn, Some(&input.contact_id))?
        .ok_or_else(|| AppError::validation("Indica con quién se comparte la suscripción."))?;
    if input.share_type.as_str() == "percent" {
        if input.share_value <= 0 || input.share_value > 100 {
            return Err(AppError::validation(
                "El porcentaje debe estar entre 1 y 100.",
            ));
        }
    } else if input.share_type.as_str() == "amount" {
        services::positive(input.share_value)?;
    } else {
        return Err(AppError::validation("Indica si se comparte por porcentaje o por monto."));
    }

    // Una persona no puede participar dos veces de la misma suscripción.
    let ya_participa: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM subscription_shares WHERE subscription_id = ?1 AND contact_id = ?2",
            params![subscription_id, contact_id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    if ya_participa > 0 {
        return Err(AppError::conflict(
            "Esa persona ya participa en esta suscripción.",
        ));
    }
    let receivable_id = input
        .receivable_id
        .as_ref()
        .filter(|v| !v.trim().is_empty())
        .cloned();
    let id = new_id();
    repos::subscriptions::insert_share(
        conn,
        &id,
        subscription_id,
        &contact_id,
        &input.share_type,
        input.share_value,
        receivable_id.as_deref(),
    )?;
    let shares = repos::subscriptions::list_shares(conn, subscription_id)?;
    shares
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::other("No se pudo guardar la participación."))
}

pub fn remove_share(conn: &Connection, share_id: &str) -> AppResult<()> {
    repos::subscriptions::delete_share(conn, share_id)
}

/// Monto que corresponde a una persona según el tipo de participación.
pub fn share_amount(subscription: &Subscription, share: &SubscriptionShare) -> i64 {
    match share.share_type.as_str() {
        "percent" => {
            let numerator = subscription.amount as i128 * share.share_value as i128;
            ((numerator + 50) / 100) as i64
        }
        _ => share.share_value,
    }
}
