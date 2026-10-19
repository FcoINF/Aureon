//! Consultas de la lista de deseos.

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{WishlistInput, WishlistItem};

const SELECT_ITEM: &str = "
    SELECT w.id, w.name, w.url, w.store, w.estimated_price, w.currency_code, w.added_date,
           w.category_id, c.name, w.priority, w.savings_goal, w.status, w.notes, w.purchase_id,
           w.created_at
    FROM wishlist_items w
    LEFT JOIN categories c ON c.id = w.category_id";

fn map_item(row: &Row<'_>) -> rusqlite::Result<WishlistItem> {
    Ok(WishlistItem {
        id: row.get(0)?,
        name: row.get(1)?,
        url: row.get(2)?,
        store: row.get(3)?,
        estimated_price: row.get(4)?,
        currency_code: row.get(5)?,
        added_date: row.get(6)?,
        category_id: row.get(7)?,
        category_name: row.get(8)?,
        priority: row.get(9)?,
        savings_goal: row.get(10)?,
        status: row.get(11)?,
        notes: row.get(12)?,
        purchase_id: row.get(13)?,
        created_at: row.get(14)?,
    })
}

pub fn list(conn: &Connection, status: Option<&str>) -> AppResult<Vec<WishlistItem>> {
    let mut sql = format!("{SELECT_ITEM} WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(s) = status.filter(|s| !s.is_empty()) {
        sql.push_str(" AND w.status = ?1");
        args.push(Box::new(s.to_string()));
    }
    sql.push_str(" ORDER BY w.priority ASC, w.added_date DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_item)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<WishlistItem> {
    let sql = format!("{SELECT_ITEM} WHERE w.id = ?1");
    conn.query_row(&sql, [id], map_item)
        .map_err(|_| AppError::not_found("El producto no existe."))
}

pub fn insert(
    conn: &Connection,
    id: &str,
    input: &WishlistInput,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO wishlist_items (id, name, url, store, estimated_price, currency_code, added_date,
            category_id, priority, savings_goal, status, notes, purchase_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13)",
        params![
            id,
            input.name,
            input.url,
            input.store,
            input.estimated_price,
            input.currency_code.clone().unwrap_or_else(|| "CLP".to_string()),
            input.added_date.clone().unwrap_or_else(|| created_at.chars().take(10).collect()),
            input.category_id,
            input.priority.unwrap_or(2),
            input.savings_goal,
            input.status.clone().unwrap_or_else(|| "wanted".to_string()),
            input.notes,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &WishlistInput, default_currency: &str) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE wishlist_items SET name = ?2, url = ?3, store = ?4, estimated_price = ?5,
                currency_code = ?6, added_date = ?7, category_id = ?8, priority = ?9,
                savings_goal = ?10, status = ?11, notes = ?12 WHERE id = ?1",
            params![
                id,
                input.name,
                input.url,
                input.store,
                input.estimated_price,
                input.currency_code.clone().unwrap_or_else(|| default_currency.to_string()),
                input.added_date,
                input.category_id,
                input.priority.unwrap_or(2),
                input.savings_goal,
                input.status.clone().unwrap_or_else(|| "wanted".to_string()),
                input.notes
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El producto no existe."));
    }
    Ok(())
}

pub fn set_purchase(conn: &Connection, id: &str, purchase_id: &str, status: &str) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE wishlist_items SET purchase_id = ?2, status = ?3 WHERE id = ?1",
            params![id, purchase_id, status],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El producto no existe."));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM wishlist_items WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}
