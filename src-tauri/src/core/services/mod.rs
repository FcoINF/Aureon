//! Servicios de aplicación.
//!
//! Aquí viven las reglas financieras: validaciones, coherencia de saldos y
//! operaciones que deben ser atómicas. Ninguna operación escribe en la base de
//! datos sin pasar antes por estas comprobaciones.

pub mod accounts;
pub mod backup;
pub mod budgets;
pub mod categories;
pub mod deposits;
pub mod export;
pub mod payables;
pub mod purchases;
pub mod receivables;
pub mod reports;
pub mod settings;
pub mod subscriptions;
pub mod transactions;
pub mod wishlist;

use rusqlite::Connection;

use crate::core::error::{AppError, AppResult};
use crate::core::repos;

/// Identificador estable y único.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Texto obligatorio, sin espacios sobrantes.
pub fn text(value: Option<&str>, field: &str) -> AppResult<String> {
    let trimmed = value.unwrap_or("").trim();
    if trimmed.is_empty() {
        return Err(AppError::validation(format!("Ingresa {field}.")));
    }
    Ok(trimmed.to_string())
}

/// Texto opcional: si está vacío se guarda como `NULL`.
pub fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Importe positivo en unidades mínimas.
pub fn positive(amount: i64) -> AppResult<i64> {
    if amount <= 0 {
        return Err(AppError::validation("El monto debe ser mayor que cero."));
    }
    Ok(amount)
}

/// Importe no negativo en unidades mínimas.
pub fn non_negative(amount: i64) -> AppResult<i64> {
    if amount < 0 {
        return Err(AppError::validation("El monto no puede ser negativo."));
    }
    Ok(amount)
}

/// Comprueba que la cuenta exista y devuelve su identificador.
pub fn ensure_account(conn: &Connection, id: Option<&str>) -> AppResult<Option<String>> {
    let id = match id {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return Ok(None),
    };
    if !repos::accounts::exists(conn, &id)? {
        return Err(AppError::validation("La cuenta seleccionada no existe."));
    }
    Ok(Some(id))
}

/// Comprueba que la categoría exista y sea del tipo indicado.
pub fn ensure_category(
    conn: &Connection,
    id: Option<&str>,
    expected_kind: Option<&str>,
) -> AppResult<Option<String>> {
    let id = match id {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return Ok(None),
    };
    let category = repos::categories::get(conn, &id, ("0000-00-00", "9999-99-99"))?;
    if let Some(kind) = expected_kind {
        if category.kind != kind {
            return Err(AppError::validation(
                "La categoría seleccionada no corresponde a este tipo de movimiento.",
            ));
        }
    }
    Ok(Some(id))
}

/// Comprueba que el contacto exista.
pub fn ensure_contact(conn: &Connection, id: Option<&str>) -> AppResult<Option<String>> {
    let id = match id {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return Ok(None),
    };
    repos::contacts::get(conn, &id)?;
    Ok(Some(id))
}

/// Comprueba que la moneda exista y devuelve su código normalizado.
pub fn ensure_currency(conn: &Connection, code: &str) -> AppResult<String> {
    let code = code.trim().to_uppercase();
    if code.is_empty() {
        return Err(AppError::validation("Indica la moneda."));
    }
    repos::ensure_currency(conn, &code)?;
    Ok(code)
}

/// Cuenta de la cuenta por cobrar indicada, o `None`.
pub fn resolve_contact(
    conn: &Connection,
    id: Option<&str>,
    name: Option<&str>,
    kind: &str,
) -> AppResult<Option<String>> {
    if let Some(id) = id.filter(|v| !v.trim().is_empty()) {
        return ensure_contact(conn, Some(id));
    }
    let name = match name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => return Ok(None),
    };
    let existing = repos::contacts::list(conn, Some(kind), Some(&name))?;
    if let Some(found) = existing.into_iter().find(|c| c.name == name) {
        return Ok(Some(found.id));
    }
    let id = new_id();
    repos::contacts::insert(
        conn,
        &id,
        &crate::core::models::ContactInput {
            name: name.clone(),
            kind: kind.to_string(),
            notes: None,
        },
        &crate::core::dates::now_str(),
    )?;
    Ok(Some(id))
}
