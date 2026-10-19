//! Copias de seguridad de la base de datos.

use std::path::Path;
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::Connection;

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::migrations;
use crate::core::models::BackupResult;

/// Crea una copia consistente de la base de datos mientras está abierta.
pub fn create_snapshot(conn: &Connection, destination: &Path) -> AppResult<BackupResult> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::File(format!("No se pudo crear la carpeta de destino: {e}"))
        })?;
    }
    if destination.exists() {
        std::fs::remove_file(destination).map_err(|e| {
            AppError::File(format!("No se pudo reemplazar el archivo de destino: {e}"))
        })?;
    }

    let mut dest = Connection::open(destination)
        .map_err(|e| AppError::File(format!("No se pudo crear la copia de seguridad: {e}")))?;
    {
        let backup = Backup::new(conn, &mut dest)
            .map_err(|e| AppError::File(format!("No se pudo crear la copia de seguridad: {e}")))?;
        backup
            .run_to_completion(128, Duration::from_millis(0), None)
            .map_err(|e| AppError::File(format!("No se pudo crear la copia de seguridad: {e}")))?;
    }
    dest.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .ok();

    let version = migrations::current_version(&dest).unwrap_or(0);
    Ok(BackupResult {
        path: destination.to_string_lossy().to_string(),
        schema_version: version,
        created_at: dates::now_str(),
    })
}

/// Comprueba que un archivo sea una copia de Aureon válida antes de restaurar.
pub fn validate_source(source: &Path) -> AppResult<i64> {
    if !source.exists() {
        return Err(AppError::validation("El archivo seleccionado no existe."));
    }
    migrations::validate_backup(source)
}

/// Copia un archivo de forma segura (copia en un temporal y reemplazo).
pub fn copy_file(source: &Path, destination: &Path) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::File(format!("No se pudo crear la carpeta de destino: {e}"))
        })?;
    }
    let temporary = destination.with_extension("restaurando");
    std::fs::copy(source, &temporary).map_err(|e| {
        AppError::File(format!("No se pudo copiar la copia de seguridad: {e}"))
    })?;
    if destination.exists() {
        std::fs::remove_file(destination).map_err(|e| {
            AppError::File(format!("No se pudo reemplazar la base de datos: {e}"))
        })?;
    }
    std::fs::rename(&temporary, destination).map_err(|e| {
        AppError::File(format!("No se pudo restaurar la base de datos: {e}"))
    })?;
    Ok(())
}

/// Elimina los archivos auxiliares de WAL tras restaurar.
pub fn cleanup_wal(database_path: &Path) {
    let wal = database_path.with_extension("db-wal");
    let shm = database_path.with_extension("db-shm");
    let _ = std::fs::remove_file(wal);
    let _ = std::fs::remove_file(shm);
}
