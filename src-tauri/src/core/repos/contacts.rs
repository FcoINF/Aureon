//! Consultas de contactos (personas y comercios).

use rusqlite::{params, Connection, Row};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Contact, ContactInput};

fn map_contact(row: &Row<'_>) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        notes: row.get(3)?,
        created_at: row.get(4)?,
    })
}

pub fn list(conn: &Connection, kind: Option<&str>, search: Option<&str>) -> AppResult<Vec<Contact>> {
    let mut sql = String::from(
        "SELECT id, name, kind, notes, created_at FROM contacts WHERE 1 = 1",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(k) = kind {
        sql.push_str(" AND kind = ?1");
        args.push(Box::new(k.to_string()));
    }
    if let Some(q) = search {
        if !q.trim().is_empty() {
            sql.push_str(" AND name LIKE ?2");
            args.push(Box::new(format!("%{}%", q.trim())));
        }
    }
    sql.push_str(" ORDER BY name COLLATE NOCASE ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), map_contact)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Contact> {
    conn.query_row(
        "SELECT id, name, kind, notes, created_at FROM contacts WHERE id = ?1",
        [id],
        map_contact,
    )
    .map_err(|_| AppError::not_found("El contacto no existe."))
}

pub fn insert(conn: &Connection, id: &str, input: &ContactInput, created_at: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO contacts (id, name, kind, notes, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, input.name, input.kind, input.notes, created_at],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &ContactInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE contacts SET name = ?2, kind = ?3, notes = ?4 WHERE id = ?1",
            params![id, input.name, input.kind, input.notes],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El contacto no existe."));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let used: i64 = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM transactions WHERE contact_id = ?1)
                  + (SELECT COUNT(*) FROM receivables WHERE contact_id = ?1)
                  + (SELECT COUNT(*) FROM payables WHERE contact_id = ?1)",
            [id],
            |row| row.get(0),
        )
        .map_err(AppError::from_sqlite)?;
    if used > 0 {
        return Err(AppError::conflict(
            "El contacto tiene movimientos asociados y no se puede eliminar.",
        ));
    }
    conn.execute("DELETE FROM contacts WHERE id = ?1", [id])
        .map_err(AppError::from_sqlite)?;
    Ok(())
}
