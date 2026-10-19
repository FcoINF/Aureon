//! Consultas de movimientos financieros.

use rusqlite::{params, Connection, Row, ToSql};

use crate::core::error::{AppError, AppResult};
use crate::core::models::{Transaction, TransactionFilter, TransactionInput};
use crate::core::repos::to_bool;

const SELECT_TRANSACTION: &str = "
    SELECT t.id, t.type, t.description, t.amount, t.currency_code, t.date,
           t.category_id, c.name, t.account_id, a.name,
           t.transfer_account_id, ta.name,
           t.contact_id, ct.name, t.payment_method, t.notes, t.tags,
           t.voided, t.voided_at, t.linked_type, t.linked_id,
           t.related_transaction_id, t.created_at
    FROM transactions t
    LEFT JOIN categories c ON c.id = t.category_id
    LEFT JOIN accounts a   ON a.id = t.account_id
    LEFT JOIN accounts ta  ON ta.id = t.transfer_account_id
    LEFT JOIN contacts ct  ON ct.id = t.contact_id";

fn map_transaction(row: &Row<'_>) -> rusqlite::Result<Transaction> {
    Ok(Transaction {
        id: row.get(0)?,
        transaction_type: row.get(1)?,
        description: row.get(2)?,
        amount: row.get(3)?,
        currency_code: row.get(4)?,
        date: row.get(5)?,
        category_id: row.get(6)?,
        category_name: row.get(7)?,
        account_id: row.get(8)?,
        account_name: row.get(9)?,
        transfer_account_id: row.get(10)?,
        transfer_account_name: row.get(11)?,
        contact_id: row.get(12)?,
        contact_name: row.get(13)?,
        payment_method: row.get(14)?,
        notes: row.get(15)?,
        tags: row.get(16)?,
        voided: to_bool(row.get(17)?),
        voided_at: row.get(18)?,
        linked_type: row.get(19)?,
        linked_id: row.get(20)?,
        related_transaction_id: row.get(21)?,
        created_at: row.get(22)?,
    })
}

/// Construye la cláusula `WHERE` y sus argumentos a partir de un filtro.
pub fn build_where(filter: &TransactionFilter) -> (String, Vec<Box<dyn ToSql>>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut args: Vec<Box<dyn ToSql>> = Vec::new();
    let mut next = 1usize;

    if !filter.include_voided.unwrap_or(false) {
        clauses.push("t.voided = 0".to_string());
    }
    if let Some(start) = filter.start_date.as_ref().filter(|s| !s.is_empty()) {
        clauses.push(format!("t.date >= ?{next}"));
        args.push(Box::new(start.clone()));
        next += 1;
    }
    if let Some(end) = filter.end_date.as_ref().filter(|s| !s.is_empty()) {
        clauses.push(format!("t.date <= ?{next}"));
        args.push(Box::new(end.clone()));
        next += 1;
    }
    if let Some(types) = filter.types.as_ref().filter(|t| !t.is_empty()) {
        let mut placeholders = Vec::new();
        for t in types {
            placeholders.push(format!("?{next}"));
            args.push(Box::new(t.clone()));
            next += 1;
        }
        clauses.push(format!("t.type IN ({})", placeholders.join(", ")));
    }
    if let Some(ids) = filter.category_ids.as_ref().filter(|t| !t.is_empty()) {
        let mut placeholders = Vec::new();
        for t in ids {
            placeholders.push(format!("?{next}"));
            args.push(Box::new(t.clone()));
            next += 1;
        }
        clauses.push(format!("t.category_id IN ({})", placeholders.join(", ")));
    }
    if let Some(ids) = filter.account_ids.as_ref().filter(|t| !t.is_empty()) {
        let mut placeholders = Vec::new();
        for t in ids {
            placeholders.push(format!("?{next}"));
            args.push(Box::new(t.clone()));
            next += 1;
        }
        clauses.push(format!(
            "(t.account_id IN ({}) OR t.transfer_account_id IN ({}))",
            placeholders.join(", "),
            placeholders.join(", ")
        ));
    }
    if let Some(ids) = filter.contact_ids.as_ref().filter(|t| !t.is_empty()) {
        let mut placeholders = Vec::new();
        for t in ids {
            placeholders.push(format!("?{next}"));
            args.push(Box::new(t.clone()));
            next += 1;
        }
        clauses.push(format!("t.contact_id IN ({})", placeholders.join(", ")));
    }
    if let Some(method) = filter.payment_method.as_ref().filter(|s| !s.is_empty()) {
        clauses.push(format!("t.payment_method = ?{next}"));
        args.push(Box::new(method.clone()));
        next += 1;
    }
    if let Some(min) = filter.min_amount {
        clauses.push(format!("t.amount >= ?{next}"));
        args.push(Box::new(min));
        next += 1;
    }
    if let Some(max) = filter.max_amount {
        clauses.push(format!("t.amount <= ?{next}"));
        args.push(Box::new(max));
        next += 1;
    }
    if let Some(search) = filter.search.as_ref().filter(|s| !s.trim().is_empty()) {
        clauses.push(format!(
            "(t.description LIKE ?{next} OR t.notes LIKE ?{next} OR ct.name LIKE ?{next} OR t.tags LIKE ?{next})"
        ));
        args.push(Box::new(format!("%{}%", search.trim())));
        next += 1;
    }
    if let Some(linked_type) = filter.linked_type.as_ref().filter(|s| !s.is_empty()) {
        clauses.push(format!("t.linked_type = ?{next}"));
        args.push(Box::new(linked_type.clone()));
        next += 1;
    }
    if let Some(linked_id) = filter.linked_id.as_ref().filter(|s| !s.is_empty()) {
        clauses.push(format!("t.linked_id = ?{next}"));
        args.push(Box::new(linked_id.clone()));
    }

    if clauses.is_empty() {
        (String::new(), args)
    } else {
        (format!(" WHERE {}", clauses.join(" AND ")), args)
    }
}

fn order_clause(filter: &TransactionFilter) -> String {
    let column = match filter.sort.as_deref() {
        Some("amount") => "t.amount",
        Some("description") => "t.description",
        Some("category") => "c.name",
        Some("account") => "a.name",
        Some("type") => "t.type",
        _ => "t.date",
    };
    let direction = match filter.direction.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };
    format!(" ORDER BY {column} {direction}, t.created_at DESC, t.id DESC")
}

pub fn list(conn: &Connection, filter: &TransactionFilter) -> AppResult<Vec<Transaction>> {
    let (where_clause, args) = build_where(filter);
    let mut sql = format!("{SELECT_TRANSACTION}{where_clause}{}", order_clause(filter));
    let limit = filter.limit.unwrap_or(500).clamp(1, 5000);
    let offset = filter.offset.unwrap_or(0).max(0);
    sql.push_str(" LIMIT ? OFFSET ?");
    let mut all_args = args;
    all_args.push(Box::new(limit));
    all_args.push(Box::new(offset));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(all_args.iter()), map_transaction)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from_sqlite)
}

pub fn count(conn: &Connection, filter: &TransactionFilter) -> AppResult<i64> {
    let (where_clause, args) = build_where(filter);
    let sql = format!(
        "SELECT COUNT(*) FROM transactions t
         LEFT JOIN categories c ON c.id = t.category_id
         LEFT JOIN accounts a   ON a.id = t.account_id
         LEFT JOIN accounts ta  ON ta.id = t.transfer_account_id
         LEFT JOIN contacts ct  ON ct.id = t.contact_id{where_clause}"
    );
    conn.query_row(&sql, rusqlite::params_from_iter(args.iter()), |row| {
        row.get(0)
    })
    .map_err(AppError::from_sqlite)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Transaction> {
    let sql = format!("{SELECT_TRANSACTION} WHERE t.id = ?1");
    conn.query_row(&sql, [id], map_transaction)
        .map_err(|_| AppError::not_found("El movimiento no existe."))
}

pub fn insert(
    conn: &Connection,
    id: &str,
    input: &TransactionInput,
    linked_type: Option<&str>,
    linked_id: Option<&str>,
    created_at: &str,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO transactions
            (id, type, description, amount, currency_code, date, category_id, account_id,
             transfer_account_id, contact_id, payment_method, notes, tags, voided,
             linked_type, linked_id, related_transaction_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, ?14, ?15, ?16, ?17)",
        params![
            id,
            input.transaction_type,
            input.description,
            input.amount,
            input.currency_code,
            input.date,
            input.category_id,
            input.account_id,
            input.transfer_account_id,
            input.contact_id,
            input.payment_method,
            input.notes,
            input.tags,
            linked_type,
            linked_id,
            input.related_transaction_id,
            created_at
        ],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}

pub fn update(conn: &Connection, id: &str, input: &TransactionInput) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE transactions SET type = ?2, description = ?3, amount = ?4, currency_code = ?5,
                date = ?6, category_id = ?7, account_id = ?8, transfer_account_id = ?9,
                contact_id = ?10, payment_method = ?11, notes = ?12, tags = ?13
             WHERE id = ?1 AND voided = 0",
            params![
                id,
                input.transaction_type,
                input.description,
                input.amount,
                input.currency_code,
                input.date,
                input.category_id,
                input.account_id,
                input.transfer_account_id,
                input.contact_id,
                input.payment_method,
                input.notes,
                input.tags
            ],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El movimiento no existe o fue anulado."));
    }
    Ok(())
}

/// Anulación lógica: el registro se conserva para la auditoría.
pub fn void(conn: &Connection, id: &str, voided_at: &str) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE transactions SET voided = 1, voided_at = ?2 WHERE id = ?1 AND voided = 0",
            params![id, voided_at],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El movimiento no existe o ya estaba anulado."));
    }
    Ok(())
}

pub fn unvoid(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn
        .execute(
            "UPDATE transactions SET voided = 0, voided_at = NULL WHERE id = ?1",
            [id],
        )
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::not_found("El movimiento no existe."));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    let changed = conn
        .execute("DELETE FROM transactions WHERE id = ?1 AND voided = 1", [id])
        .map_err(AppError::from_sqlite)?;
    if changed == 0 {
        return Err(AppError::conflict(
            "Sólo se pueden eliminar movimientos que ya estaban anulados.",
        ));
    }
    Ok(())
}

pub fn exists(conn: &Connection, id: &str) -> AppResult<bool> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transactions WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(AppError::from_sqlite)?;
    Ok(count > 0)
}

/// Marca el vínculo de un movimiento con su origen (cuota, suscripción, etc.).
pub fn set_link(conn: &Connection, id: &str, linked_type: &str, linked_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE transactions SET linked_type = ?2, linked_id = ?3 WHERE id = ?1",
        params![id, linked_type, linked_id],
    )
    .map_err(AppError::from_sqlite)?;
    Ok(())
}
