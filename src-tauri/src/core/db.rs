//! Acceso a la base de datos SQLite.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::core::error::{AppError, AppResult};
use crate::core::migrations;

pub const DB_FILE_NAME: &str = "aureon.db";

pub fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DB_FILE_NAME)
}

/// Abre la base con las pragmas de la aplicación: claves foráneas activas,
/// journal WAL (lecturas concurrentes sin bloquear) y espera ante bloqueos.
pub fn connect(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path).map_err(AppError::from_sqlite)?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(AppError::from_sqlite)?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(AppError::from_sqlite)?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(AppError::from_sqlite)?;
    conn.pragma_update(None, "busy_timeout", 5000)
        .map_err(AppError::from_sqlite)?;
    Ok(conn)
}

/// Abre y migra una base existente.
pub fn open_existing(path: &Path) -> AppResult<Connection> {
    let mut conn = connect(path)?;
    migrations::migrate(&mut conn)?;
    Ok(conn)
}

/// Crea la base de datos desde cero y aplica el esquema inicial.
pub fn create_new(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::File(format!(
                "No se pudo crear la carpeta de datos de Aureon: {e}"
            ))
        })?;
    }
    let mut conn = connect(path)?;
    if let Err(error) = migrations::migrate(&mut conn) {
        // No se deja un archivo vacío: la aplicación no debe quedar con una
        // base de datos a medio crear.
        drop(conn);
        let _ = std::fs::remove_file(path);
        return Err(error);
    }
    Ok(conn)
}

/// Devuelve `true` si el archivo no contiene información (0 bytes).
pub fn is_empty_file(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.len() == 0,
        Err(_) => false,
    }
}

/// Comprueba si un archivo SQLite es utilizable (no corrupto).
pub fn is_readable_database(path: &Path) -> bool {
    match Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(conn) => conn
            .query_row("PRAGMA quick_check", [], |row| {
                row.get::<_, String>(0)
            })
            .map(|result| result == "ok")
            .unwrap_or(false),
        Err(_) => false,
    }
}
