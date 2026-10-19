//! Mantenimiento de estados vencidos.
//!
//! Aquí sólo se detectan vencimientos. Ninguna operación marca un pago como
//! realizado: eso siempre es una decisión explícita del usuario.

use rusqlite::Connection;

use crate::core::error::AppResult;

/// Marca vencidas las obligaciones, depósitos y suscripciones cuya fecha pasó.
pub fn refresh(conn: &Connection, today: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE receivables SET status = 'overdue'
          WHERE status IN ('pending', 'partial') AND due_date IS NOT NULL AND due_date < ?1",
        [today],
    )
    .map_err(crate::core::error::AppError::from_sqlite)?;
    conn.execute(
        "UPDATE payables SET status = 'overdue'
          WHERE status IN ('pending', 'partial') AND due_date IS NOT NULL AND due_date < ?1",
        [today],
    )
    .map_err(crate::core::error::AppError::from_sqlite)?;
    conn.execute(
        "UPDATE deposits SET status = 'overdue'
          WHERE status IN ('pending', 'scheduled') AND due_date IS NOT NULL AND due_date < ?1",
        [today],
    )
    .map_err(crate::core::error::AppError::from_sqlite)?;
    conn.execute(
        "UPDATE subscriptions SET status = 'expired'
          WHERE status = 'active' AND next_charge_date IS NOT NULL AND next_charge_date < ?1",
        [today],
    )
    .map_err(crate::core::error::AppError::from_sqlite)?;
    Ok(())
}
