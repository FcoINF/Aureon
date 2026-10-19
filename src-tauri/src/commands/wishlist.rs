//! Comandos de la lista de deseos.

use tauri::State;

use crate::core::models::{Purchase, PurchaseInput, WishlistInput, WishlistItem};
use crate::core::services::{settings, wishlist};
use crate::state::AppState;

#[tauri::command]
pub fn list_wishlist(
    state: State<'_, AppState>,
    status: Option<String>,
) -> Result<Vec<WishlistItem>, String> {
    state
        .read_or(|conn| wishlist::list(conn, status.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_wishlist_item(
    state: State<'_, AppState>,
    input: WishlistInput,
) -> Result<WishlistItem, String> {
    state
        .write(|conn| {
            let currency = settings::default_currency(conn);
            wishlist::create(conn, &input, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_wishlist_item(
    state: State<'_, AppState>,
    id: String,
    input: WishlistInput,
) -> Result<WishlistItem, String> {
    state
        .write(|conn| {
            let currency = settings::default_currency(conn);
            wishlist::update(conn, &id, &input, &currency)
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_wishlist_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<WishlistItem, String> {
    state
        .write(|conn| wishlist::set_status(conn, &id, &status))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn convert_wishlist_item(
    state: State<'_, AppState>,
    id: String,
    input: PurchaseInput,
) -> Result<Purchase, String> {
    state
        .write(|conn| wishlist::convert_to_purchase(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_wishlist_item(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| wishlist::delete(conn, &id))
        .map_err(|e| e.to_string())
}
