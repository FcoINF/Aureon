//! Migraciones versionadas del esquema.
//!
//! Cada migración se aplica una sola vez dentro de una transacción. La tabla
//! `schema_migrations` registra la versión aplicada, por lo que las versiones
//! nuevas nunca destruyen información existente.

use rusqlite::Connection;

use crate::core::error::{AppError, AppResult};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "esquema_inicial",
    sql: include_str!("../../migrations/0001_init.sql"),
}];

pub const CREATE_MIGRATIONS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS schema_migrations (
        version    INTEGER PRIMARY KEY,
        name       TEXT NOT NULL,
        applied_at TEXT NOT NULL
    );";

pub fn current_version(conn: &Connection) -> AppResult<i64> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
        [],
        |row| row.get(0),
    )?;
    if exists == 0 {
        return Ok(0);
    }
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?)
}

/// Aplica las migraciones pendientes. Es idempotente: si la base ya está al
/// día, no modifica nada.
pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch(CREATE_MIGRATIONS_TABLE)
        .map_err(AppError::from_sqlite)?;
    let current = current_version(conn)?;
    for migration in MIGRATIONS {
        if migration.version <= current {
            continue;
        }
        let tx = conn.transaction().map_err(AppError::from_sqlite)?;
        tx.execute_batch(migration.sql).map_err(AppError::from_sqlite)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![migration.version, migration.name, crate::core::dates::now_str()],
        )
        .map_err(AppError::from_sqlite)?;
        tx.commit().map_err(AppError::from_sqlite)?;
    }
    Ok(())
}

/// Comprueba que un archivo sea una base de datos de Aureon válida y sana.
pub fn validate_backup(path: &std::path::Path) -> AppResult<i64> {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|_| AppError::validation("El archivo seleccionado no es una base de datos válida."))?;

    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| AppError::validation("El archivo no es una copia de seguridad válida."))?;
    if integrity != "ok" {
        return Err(AppError::validation(
            "La copia de seguridad está dañada y no se puede restaurar.",
        ));
    }

    let has_table: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| AppError::validation("El archivo no es una copia de Aureon."))?;
    if has_table == 0 {
        return Err(AppError::validation("El archivo no es una copia de Aureon."));
    }
    current_version(&conn)
}
