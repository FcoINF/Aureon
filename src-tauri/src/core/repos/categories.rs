//! Consultas de categorías.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Category, CategoryInput};
use crate::core::repos::to_bool;

fn map_category(row: &Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        color: row.get(3)?,
        archived: to_bool(row.get(4)?),
        created_at: row.get(5)?,
        spent: row.get(6)?,
        budget: row.get(7)?,
    })
}

/// `?1` es el identificador; `?2` y `?3` delimitan el período de gasto y
/// `?4` la moneda (si se indica, sólo se cuenta lo de esa moneda).
const SELECT_CATEGORY: &str = "
    SELECT c.id, c.name, c.kind, c.color, c.archived, c.created_at,
        (SELECT COALESCE(SUM(t.amount), 0) FROM transactions t
            WHERE t.category_id = c.id AND t.voided = 0 AND t.type = 'expense'
              AND t.date >= ?2 AND t.date <= ?3
              AND (?4 IS NULL OR t.currency_code = ?4)) AS spent,
        (SELECT b.amount FROM budgets b WHERE b.category_id = c.id LIMIT 1) AS budget
    FROM categories c";

pub fn list(
    conn: &Connection,
    kind: Option<&str>,
    include_archived: bool,
    period: (&str, &str),
    currency: Option<&str>,
) -> AppResult<Vec<Category>> {
    let mut sql = format!("{SELECT_CATEGORY} WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(String::new()),
        Box::new(period.0.to_string()),
        Box::new(period.1.to_string()),
        Box::new(currency.map(|c| c.to_string())),
    ];
    if let Some(k) = kind {
        sql.push_str(" AND c.kind = ?5");
        args.push(Box::new(k.to_string()));
    }
    if !include_archived {
        sql.push_str(" AND c.archived = 0");
    }
    sql.push_str(" ORDER BY c.kind ASC, c.name COLLATE NOCASE ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_category)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(
    conn: &Connection,
    id: &str,
    period: (&str, &str),
    currency: Option<&str>,
) -> AppResult<Category> {
    let sql = format!("{SELECT_CATEGORY} WHERE c.id = ?1");
    conn.query_row(
        &sql,
        params![id, period.0, period.1, currency],
        map_category,
    )
    .map_err(|_| AppError::not_found("La categoría no existe."))
}

pub fn insert(conn: &Connection, id: &str, input: &CategoryInput, created_at: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO categories (id, name, kind, color, archived, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)",
        params![id, input.name, input.kind, input.color, created_at],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &CategoryInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE categories SET name = ?2, kind = ?3, color = ?4 WHERE id = ?1",
            params![id, input.name, input.kind, input.color],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La categoría no existe."));
    }
    Ok(())
}

pub fn set_archived(conn: &Connection, id: &str, archived: bool) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE categories SET archived = ?2 WHERE id = ?1",
            params![id, if archived { 1 } else { 0 }],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("La categoría no existe."));
    }
    Ok(())
}

/// Archiva la categoría; nunca se borra para no perder el historial.
pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    set_archived(conn, id, true)
}
