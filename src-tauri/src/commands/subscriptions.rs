//! Comandos de suscripciones y cobros.

use tauri::State;

use crate::core::models::{
    Subscription, SubscriptionInput, SubscriptionPayment, SubscriptionPaymentInput,
    SubscriptionShare, SubscriptionShareInput,
};
use crate::core::services::{settings, subscriptions};
use crate::state::AppState;

#[tauri::command]
pub fn list_subscriptions(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<Subscription>, String> {
    state
        .read_or(|conn| subscriptions::list(conn, status.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_subscription(state: State<'_, AppState>, id: String) -> Result<Subscription, String> {
    state
        .read(|conn| subscriptions::get(conn, &id))?
        .ok_or_else(|| "La suscripción no existe.".to_string())
}

#[tauri::command]
pub fn list_subscription_payments(
    state: State<'_, AppState>,
    subscription_id: String,
) -> Result<Vec<SubscriptionPayment>, String> {
    state
        .read_or(|conn| subscriptions::list_payments(conn, &subscription_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_subscription(
    state: State<'_, AppState>,
    input: SubscriptionInput,
) -> Result<Subscription, String> {
    state
        .write(|conn| {
            let currency = settings::default_currency(conn);
            subscriptions::create(conn, &input, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_subscription(
    state: State<'_, AppState>,
    id: String,
    input: SubscriptionInput,
) -> Result<Subscription, String> {
    state
        .write(|conn| subscriptions::update(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn charge_subscription(
    state: State<'_, AppState>,
    id: String,
    input: SubscriptionPaymentInput,
) -> Result<SubscriptionPayment, String> {
    state
        .write(|conn| subscriptions::charge(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_subscription_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<Subscription, String> {
    state
        .write(|conn| subscriptions::set_status(conn, &id, &status))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_subscription(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| subscriptions::delete(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_subscription_shares(
    state: State<'_, AppState>,
    subscription_id: String,
) -> Result<Vec<SubscriptionShare>, String> {
    state
        .read_or(|conn| subscriptions::list_shares(conn, &subscription_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_subscription_share(
    state: State<'_, AppState>,
    subscription_id: String,
    input: SubscriptionShareInput,
) -> Result<SubscriptionShare, String> {
    state
        .write(|conn| subscriptions::add_share(conn, &subscription_id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_subscription_share(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| subscriptions::remove_share(conn, &id))
        .map_err(|e| e.to_string())
}
