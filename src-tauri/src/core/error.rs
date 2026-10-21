//! Errores de la aplicación.
//!
//! La interfaz nunca debe mostrar trazas técnicas. Cada error lleva un mensaje
//! en español apto para el usuario final y, opcionalmente, un detalle técnico
//! que sólo se registra en la consola durante el desarrollo.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    File(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }
    pub fn other(msg: impl Into<String>) -> Self {
        AppError::Other(msg.into())
    }

    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Validation(_) => "validation",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            AppError::Database(_) => "database",
            AppError::File(_) => "file",
            AppError::Other(_) => "other",
        }
    }

    /// Traduce errores de SQLite a mensajes comprensibles, sin filtrar
    /// detalles técnicos a la interfaz.
    pub fn from_sqlite(err: rusqlite::Error) -> Self {
        let detail = err.to_string();
        let message = match &err {
            rusqlite::Error::QueryReturnedNoRows => {
                "No se encontró el registro solicitado.".to_string()
            }
            rusqlite::Error::SqliteFailure(ffi_error, _) => {
                use rusqlite::ffi;
                if ffi_error.code == ffi::ErrorCode::ConstraintViolation {
                    if detail.contains("UNIQUE") {
                        "Ya existe un registro con esos mismos datos.".to_string()
                    } else if detail.contains("FOREIGN KEY") {
                        "El registro está relacionado con otros datos y no se puede modificar.".to_string()
                    } else {
                        "Los datos no cumplen las reglas de la aplicación.".to_string()
                    }
                } else if ffi_error.code == ffi::ErrorCode::DatabaseCorrupt
                    || ffi_error.code == ffi::ErrorCode::NotADatabase
                {
                    "La base de datos está dañada. Restaura una copia de seguridad para continuar.".to_string()
                } else {
                    "No se pudo completar la operación sobre los datos.".to_string()
                }
            }
            _ => "No se pudo completar la operación sobre los datos.".to_string(),
        };
        eprintln!("[aureon] error sqlite: {detail}");
        AppError::Database(message)
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::from_sqlite(err)
    }
}

impl From<rust_xlsxwriter::XlsxError> for AppError {
    fn from(err: rust_xlsxwriter::XlsxError) -> Self {
        AppError::other(format!("No se pudo generar el libro de Excel: {err}"))
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("kind", &self.kind())?;
        state.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
