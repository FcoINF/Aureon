//! Servicio de la lista de deseos.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{Purchase, PurchaseInput, WishlistInput, WishlistItem};
use crate::core::repos;
use crate::core::services::{self, new_id, purchases as purchase_service};

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<WishlistItem>> {
    repos::wishlist::list(conn, status)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<WishlistItem> {
    repos::wishlist::get(conn, id)
}

/// Valida una URL: sólo se aceptan enlaces http y https.
pub fn validate_url(url: &str) -> AppResult<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Ingresa un enlace válido."));
    }
    let lower = trimmed.to_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(AppError::validation(
            "El enlace debe comenzar con http:// o https://",
        ));
    }
    if trimmed.contains(char::is_whitespace) {
        return Err(AppError::validation("El enlace no puede contener espacios."));
    }
    Ok(trimmed.to_string())
}

pub fn create(conn: &Connection, input: &WishlistInput, default_currency: &str) -> AppResult<WishlistItem> {
    let url = match input.url.as_ref().filter(|u| !u.trim().is_empty()) {
        Some(u) => Some(validate_url(u)?),
        None => None,
    };
    let name = services::optional_text(input.name.clone());
    if name.is_none() && url.is_none() {
        return Err(AppError::validation(
            "Indica al menos el nombre del producto o su enlace.",
        ));
    }
    if let Some(price) = input.estimated_price {
        services::non_negative(price)?;
    }
    let currency = if input
        .currency_code
        .as_ref()
        .map(|c| !c.trim().is_empty())
        .unwrap_or(false)
    {
        services::ensure_currency(conn, input.currency_code.as_deref().unwrap_or(""))?
    } else {
        default_currency.to_string()
    };
    let added_date = match input.added_date.as_ref().filter(|d| !d.trim().is_empty()) {
        Some(d) => dates::format_date(dates::parse_date(d)?),
        None => dates::today_str(),
    };
    let priority = input.priority.unwrap_or(2).clamp(1, 3);
    let category_id = services::ensure_category(conn, input.category_id.as_deref(), Some("expense"))?;
    let status = match input.status.as_deref() {
        Some("saving") => "saving".to_string(),
        Some("purchased") => "purchased".to_string(),
        Some("discarded") => "discarded".to_string(),
        _ => "wanted".to_string(),
    };

    let id = new_id();
    repos::wishlist::insert(
        conn,
        &id,
        &WishlistInput {
            name,
            url,
            store: services::optional_text(input.store.clone()),
            estimated_price: input.estimated_price,
            currency_code: Some(currency),
            added_date: Some(added_date),
            category_id,
            priority: Some(priority),
            savings_goal: input.savings_goal,
            status: Some(status),
            notes: services::optional_text(input.notes.clone()),
        },
        &dates::now_str(),
    )?;
    repos::wishlist::get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &WishlistInput, default_currency: &str) -> AppResult<WishlistItem> {
    let url = match input.url.as_ref().filter(|u| !u.trim().is_empty()) {
        Some(u) => Some(validate_url(u)?),
        None => None,
    };
    let currency = if input
        .currency_code
        .as_ref()
        .map(|c| !c.trim().is_empty())
        .unwrap_or(false)
    {
        services::ensure_currency(conn, input.currency_code.as_deref().unwrap_or(""))?
    } else {
        default_currency.to_string()
    };
    let category_id = services::ensure_category(conn, input.category_id.as_deref(), Some("expense"))?;
    let status = match input.status.as_deref() {
        Some("saving") => "saving".to_string(),
        Some("purchased") => "purchased".to_string(),
        Some("discarded") => "discarded".to_string(),
        Some("wanted") => "wanted".to_string(),
        _ => "wanted".to_string(),
    };
    repos::wishlist::update(
        conn,
        id,
        &WishlistInput {
            name: services::optional_text(input.name.clone()),
            url,
            store: services::optional_text(input.store.clone()),
            estimated_price: input.estimated_price,
            currency_code: Some(currency),
            added_date: input.added_date.clone(),
            category_id,
            priority: Some(input.priority.unwrap_or(2).clamp(1, 3)),
            savings_goal: input.savings_goal,
            status: Some(status),
            notes: services::optional_text(input.notes.clone()),
        },
        default_currency,
    )?;
    repos::wishlist::get(conn, id)
}

pub fn set_status(conn: &Connection, id: &str, status: &str) -> AppResult<WishlistItem> {
    let valid = ["wanted", "saving", "purchased", "discarded"];
    if !valid.contains(&status) {
        return Err(AppError::validation("El estado indicado no es válido."));
    }
    let item = repos::wishlist::get(conn, id)?;
    repos::wishlist::set_purchase(conn, id, item.purchase_id.as_deref().unwrap_or(""), status)?;
    repos::wishlist::get(conn, id)
}

/// Convierte un producto deseado en una compra. No registra ningún gasto:
/// la compra sigue el mismo flujo que cualquier otra.
pub fn convert_to_purchase(
    conn: &mut Connection,
    wishlist_id: &str,
    input: &PurchaseInput,
) -> AppResult<Purchase> {
    let item = repos::wishlist::get(conn, wishlist_id)?;
    if item.purchase_id.as_ref().is_some_and(|v| !v.is_empty()) {
        return Err(AppError::conflict(
            "Este producto ya fue convertido en una compra.",
        ));
    }

    let mut purchase_input = input.clone();
    if purchase_input.name.trim().is_empty() {
        purchase_input.name = item
            .name
            .clone()
            .unwrap_or_else(|| "Producto".to_string());
    }
    if purchase_input.url.as_ref().map(|u| u.is_empty()).unwrap_or(true) {
        purchase_input.url = item.url.clone();
    }
    if purchase_input.total_amount <= 0 {
        purchase_input.total_amount = item.estimated_price.unwrap_or(0);
    }
    if purchase_input.total_amount <= 0 {
        return Err(AppError::validation(
            "Indica el monto de la compra antes de convertir el producto.",
        ));
    }
    if purchase_input.contact_id.is_none() {
        purchase_input.contact_id = item.category_id.clone().filter(|_| false);
    }

    let purchase = purchase_service::create(conn, &purchase_input, &item.currency_code)?;
    repos::wishlist::set_purchase(conn, wishlist_id, &purchase.id, "purchased")?;
    Ok(purchase)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    repos::wishlist::delete(conn, id)
}
