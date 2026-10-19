//! Comandos de cuentas, categorías, contactos y monedas.

use tauri::State;

use crate::core::models::{AccountInput, CategoryInput, ContactInput};
use crate::core::repos;
use crate::core::services::{accounts, categories};
use crate::state::AppState;

#[tauri::command]
pub fn list_currencies(state: State<'_, AppState>) -> Result<Vec<crate::core::models::Currency>, String> {
    state
        .read_or(|conn| repos::list_currencies(conn))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_accounts(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> Result<Vec<crate::core::models::Account>, String> {
    state
        .read_or(|conn| accounts::list(conn, include_archived.unwrap_or(false)))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_account(
    state: State<'_, AppState>,
    input: AccountInput,
) -> Result<crate::core::models::Account, String> {
    state
        .write(|conn| accounts::create(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_account(
    state: State<'_, AppState>,
    id: String,
    input: AccountInput,
) -> Result<crate::core::models::Account, String> {
    state
        .write(|conn| accounts::update(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_account_archived(
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<crate::core::models::Account, String> {
    state
        .write(|conn| accounts::set_archived(conn, &id, archived))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_account(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| accounts::delete(conn, &id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_categories(
    state: State<'_, AppState>,
    kind: Option<String>,
    include_archived: Option<bool>,
    period: Option<String>,
) -> Result<Vec<crate::core::models::Category>, String> {
    let (start, end) = match period {
        Some(p) => crate::core::dates::period_range(&p, 0).map_err(|e| e.to_string())?,
        None => crate::core::dates::period_range("month", 0).map_err(|e| e.to_string())?,
    };
    state
        .read_or(|conn| {
            categories::list_categories(
                conn,
                kind.as_deref(),
                include_archived.unwrap_or(false),
                (&start, &end),
            )
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_category(
    state: State<'_, AppState>,
    input: CategoryInput,
) -> Result<crate::core::models::Category, String> {
    state
        .write(|conn| categories::create_category(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_category(
    state: State<'_, AppState>,
    id: String,
    input: CategoryInput,
) -> Result<crate::core::models::Category, String> {
    state
        .write(|conn| categories::update_category(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_category_archived(
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<crate::core::models::Category, String> {
    state
        .write(|conn| categories::archive_category(conn, &id, archived))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_contacts(
    state: State<'_, AppState>,
    kind: Option<String>,
    search: Option<String>,
) -> Result<Vec<crate::core::models::Contact>, String> {
    state
        .read_or(|conn| categories::list_contacts(conn, kind.as_deref(), search.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_contact(
    state: State<'_, AppState>,
    input: ContactInput,
) -> Result<crate::core::models::Contact, String> {
    state
        .write(|conn| categories::create_contact(conn, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_contact(
    state: State<'_, AppState>,
    id: String,
    input: ContactInput,
) -> Result<crate::core::models::Contact, String> {
    state
        .write(|conn| categories::update_contact(conn, &id, &input))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_contact(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .write(|conn| categories::delete_contact(conn, &id))
        .map_err(|e| e.to_string())
}
