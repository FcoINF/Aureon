//! Servicios de categorías y contactos.

use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{Category, CategoryInput, Contact, ContactInput};
use crate::core::repos;
use crate::core::services::{self, new_id};

pub fn list_categories(
    conn: &Connection,
    kind: Option<&str>,
    include_archived: bool,
    period: (&str, &str),
    currency: Option<&str>,
) -> AppResult<Vec<Category>> {
    repos::categories::list(conn, kind, include_archived, period, currency)
}

pub fn create_category(conn: &Connection, input: &CategoryInput) -> AppResult<Category> {
    let name = services::text(Some(&input.name), "el nombre de la categoría")?;
    let kind = match input.kind.as_str() {
        "income" => "income",
        "expense" => "expense",
        _ => return Err(AppError::validation("Indica si la categoría es de ingresos o de gastos.")),
    };
    let id = new_id();
    let normalized = CategoryInput {
        name,
        kind: kind.to_string(),
        color: services::optional_text(input.color.clone()),
    };
    repos::categories::insert(conn, &id, &normalized, &dates::now_str())?;
    repos::categories::get(conn, &id, ("0000-00-00", "9999-99-99"), None)
}

pub fn update_category(conn: &Connection, id: &str, input: &CategoryInput) -> AppResult<Category> {
    let name = services::text(Some(&input.name), "el nombre de la categoría")?;
    let kind = match input.kind.as_str() {
        "income" => "income",
        "expense" => "expense",
        _ => return Err(AppError::validation("Indica si la categoría es de ingresos o de gastos.")),
    };
    let normalized = CategoryInput {
        name,
        kind: kind.to_string(),
        color: services::optional_text(input.color.clone()),
    };
    repos::categories::update(conn, id, &normalized)?;
    repos::categories::get(conn, id, ("0000-00-00", "9999-99-99"), None)
}

pub fn archive_category(conn: &Connection, id: &str, archived: bool) -> AppResult<Category> {
    repos::categories::set_archived(conn, id, archived)?;
    repos::categories::get(conn, id, ("0000-00-00", "9999-99-99"), None)
}

pub fn list_contacts(
    conn: &Connection,
    kind: Option<&str>,
    search: Option<&str>,
) -> AppResult<Vec<Contact>> {
    repos::contacts::list(conn, kind, search)
}

pub fn create_contact(conn: &Connection, input: &ContactInput) -> AppResult<Contact> {
    let name = services::text(Some(&input.name), "el nombre")?;
    let kind = match input.kind.as_str() {
        "person" | "merchant" => input.kind.clone(),
        _ => return Err(AppError::validation("Indica si es una persona o un comercio.")),
    };
    let id = new_id();
    let normalized = ContactInput {
        name,
        kind,
        notes: services::optional_text(input.notes.clone()),
    };
    repos::contacts::insert(conn, &id, &normalized, &dates::now_str())?;
    repos::contacts::get(conn, &id)
}

pub fn update_contact(conn: &Connection, id: &str, input: &ContactInput) -> AppResult<Contact> {
    let name = services::text(Some(&input.name), "el nombre")?;
    let kind = match input.kind.as_str() {
        "person" | "merchant" => input.kind.clone(),
        _ => return Err(AppError::validation("Indica si es una persona o un comercio.")),
    };
    let normalized = ContactInput {
        name,
        kind,
        notes: services::optional_text(input.notes.clone()),
    };
    repos::contacts::update(conn, id, &normalized)?;
    repos::contacts::get(conn, id)
}

pub fn delete_contact(conn: &Connection, id: &str) -> AppResult<()> {
    repos::contacts::delete(conn, id)
}
