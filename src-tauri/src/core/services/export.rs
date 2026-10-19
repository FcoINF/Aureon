//! Exportación de datos a CSV.
//!
//! Se usa el separador configurado por el usuario (punto y coma por defecto,
//! el habitual en Excel en español) y un BOM UTF-8 para que los archivos
//! abran correctamente. Las fechas se exportan como DD/MM/AAAA y los importes
//! con punto de miles y coma decimal.

use rusqlite::Connection;

use crate::core::error::AppResult;
use crate::core::models::TransactionFilter;
use crate::core::repos;
use crate::core::services::transactions as tx_service;

pub const CSV_BOM: &str = "\u{feff}";

/// Tope de filas por exportación. Es muy superior al que muestra la interfaz,
/// de modo que el archivo no se trunca en silencio.
const EXPORT_LIMIT: i64 = 1_000_000;

/// Separador configurado por el usuario (punto y coma por defecto).
pub fn separator(conn: &Connection) -> String {
    repos::settings::get_setting(conn, "export_separator")
        .ok()
        .flatten()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ";".to_string())
}

fn escape(value: &str, sep: &str) -> String {
    if value.contains(sep) || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn row(values: &[String], sep: &str) -> String {
    let cells: Vec<String> = values.iter().map(|v| escape(v, sep)).collect();
    cells.join(sep)
}

/// Convierte `AAAA-MM-DD` en `DD/MM/AAAA`.
pub fn format_date(value: &str) -> String {
    if value.len() == 10 && value.as_bytes()[4] == b'-' && value.as_bytes()[7] == b'-' {
        format!("{}/{}/{}", &value[8..10], &value[5..7], &value[0..4])
    } else {
        value.to_string()
    }
}

fn header(columns: &[&str], sep: &str) -> String {
    row(
        &columns.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
        sep,
    )
}

/// Exporta los movimientos aplicando los filtros del historial.
pub fn transactions_csv(conn: &Connection, filter: &TransactionFilter) -> AppResult<String> {
    let sep = separator(conn);
    // La exportación no se limita a las 500 filas que muestra la tabla.
    let filtro = TransactionFilter {
        limit: Some(EXPORT_LIMIT),
        ..filter.clone()
    };
    let transactions = tx_service::list(conn, &filtro)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Fecha",
            "Tipo",
            "Descripción",
            "Monto",
            "Moneda",
            "Categoría",
            "Cuenta",
            "Cuenta destino",
            "Comercio o persona",
            "Método de pago",
            "Notas",
            "Anulado",
        ],
        &sep,
    ));
    for t in transactions {
        let kind = match t.transaction_type.as_str() {
            "income" => "Ingreso",
            "expense" => "Gasto",
            "transfer" => "Transferencia",
            "refund" => "Devolución",
            _ => t.transaction_type.as_str(),
        };
        out.push('\n');
        out.push_str(&row(
            &[
                format_date(&t.date),
                kind.to_string(),
                t.description,
                t.amount.to_string(),
                t.currency_code,
                t.category_name.unwrap_or_default(),
                t.account_name.unwrap_or_default(),
                t.transfer_account_name.unwrap_or_default(),
                t.contact_name.unwrap_or_default(),
                t.payment_method.unwrap_or_default(),
                t.notes.unwrap_or_default(),
                if t.voided { "Sí".to_string() } else { "No".to_string() },
            ],
            &sep,
        ));
    }
    Ok(out)
}

/// Exporta las compras con el estado de sus cuotas.
pub fn purchases_csv(conn: &Connection) -> AppResult<String> {
    let sep = separator(conn);
    let purchases = repos::purchases::list(conn, None)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Compra",
            "Comercio",
            "Fecha de compra",
            "Monto total",
            "Moneda",
            "Cuotas",
            "Cuotas pagadas",
            "Monto pagado",
            "Saldo pendiente",
            "Próximo vencimiento",
            "Estado",
        ],
        &sep,
    ));
    for p in purchases {
        out.push('\n');
        out.push_str(&row(
            &[
                p.name,
                p.contact_name.unwrap_or_default(),
                format_date(&p.purchase_date),
                p.total_amount.to_string(),
                p.currency_code,
                p.installments_count.to_string(),
                p.installments_paid.to_string(),
                p.paid_amount.to_string(),
                p.pending_amount.to_string(),
                p.next_due_date.map(|d| format_date(&d)).unwrap_or_default(),
                p.status,
            ],
            &sep,
        ));
    }
    Ok(out)
}

/// Exporta el detalle de cuotas de todas las compras.
pub fn installments_csv(conn: &Connection) -> AppResult<String> {
    let sep = separator(conn);
    let purchases = repos::purchases::list(conn, None)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Compra",
            "Nº cuota",
            "Monto",
            "Vencimiento",
            "Estado",
            "Pagado",
            "Fecha de pago",
        ],
        &sep,
    ));
    for p in purchases {
        for i in repos::purchases::list_installments(conn, &p.id)? {
            out.push('\n');
            out.push_str(&row(
                &[
                    p.name.clone(),
                    i.number.to_string(),
                    i.amount.to_string(),
                    format_date(&i.due_date),
                    i.status,
                    i.paid_amount.to_string(),
                    i.paid_at.map(|d| format_date(&d)).unwrap_or_default(),
                ],
                &sep,
            ));
        }
    }
    Ok(out)
}

/// Exporta las suscripciones.
pub fn subscriptions_csv(conn: &Connection) -> AppResult<String> {
    let sep = separator(conn);
    let subscriptions = repos::subscriptions::list(conn, None)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Servicio",
            "Categoría",
            "Proveedor",
            "Monto",
            "Moneda",
            "Frecuencia",
            "Próximo cobro",
            "Estado",
            "Total pagado",
        ],
        &sep,
    ));
    for s in subscriptions {
        out.push('\n');
        out.push_str(&row(
            &[
                s.name,
                s.category_name.unwrap_or_default(),
                s.contact_name.unwrap_or_default(),
                s.amount.to_string(),
                s.currency_code,
                s.frequency,
                s.next_charge_date.map(|d| format_date(&d)).unwrap_or_default(),
                s.status,
                s.paid_total.to_string(),
            ],
            &sep,
        ));
    }
    Ok(out)
}

/// Exporta las cuentas por cobrar.
pub fn receivables_csv(conn: &Connection) -> AppResult<String> {
    let sep = separator(conn);
    let receivables = repos::receivables::list(conn, None)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Motivo",
            "Persona",
            "Tipo",
            "Origen",
            "Vencimiento",
            "Monto",
            "Recibido",
            "Saldo",
            "Moneda",
            "Estado",
        ],
        &sep,
    ));
    for r in receivables {
        out.push('\n');
        out.push_str(&row(
            &[
                r.title,
                r.contact_name,
                r.kind,
                format_date(&r.origin_date),
                r.due_date.map(|d| format_date(&d)).unwrap_or_default(),
                r.total_amount.to_string(),
                r.received_amount.to_string(),
                r.pending_amount.to_string(),
                r.currency_code,
                r.status,
            ],
            &sep,
        ));
    }
    Ok(out)
}

/// Exporta las cuentas por pagar.
pub fn payables_csv(conn: &Connection) -> AppResult<String> {
    let sep = separator(conn);
    let payables = repos::payables::list(conn, None)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Descripción",
            "Acreedor",
            "Creación",
            "Vencimiento",
            "Monto",
            "Pagado",
            "Saldo",
            "Moneda",
            "Estado",
        ],
        &sep,
    ));
    for p in payables {
        out.push('\n');
        out.push_str(&row(
            &[
                p.title,
                p.contact_name,
                format_date(&p.created_date),
                p.due_date.map(|d| format_date(&d)).unwrap_or_default(),
                p.total_amount.to_string(),
                p.paid_amount.to_string(),
                p.pending_amount.to_string(),
                p.currency_code,
                p.status,
            ],
            &sep,
        ));
    }
    Ok(out)
}

/// Exporta los depósitos pendientes.
pub fn deposits_csv(conn: &Connection) -> AppResult<String> {
    let sep = separator(conn);
    let deposits = repos::deposits::list(conn, None)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Motivo",
            "Destinatario",
            "Creación",
            "Fecha límite",
            "Monto",
            "Moneda",
            "Cuenta de origen",
            "Institución de destino",
            "Estado",
            "Fecha de realización",
        ],
        &sep,
    ));
    for d in deposits {
        out.push('\n');
        out.push_str(&row(
            &[
                d.title,
                d.contact_name.unwrap_or_default(),
                format_date(&d.created_date),
                d.due_date.map(|x| format_date(&x)).unwrap_or_default(),
                d.amount.to_string(),
                d.currency_code,
                d.account_name.unwrap_or_default(),
                d.destination_institution.unwrap_or_default(),
                d.status,
                d.completed_at.map(|x| format_date(&x)).unwrap_or_default(),
            ],
            &sep,
        ));
    }
    Ok(out)
}

/// Exporta la lista de deseos.
pub fn wishlist_csv(conn: &Connection) -> AppResult<String> {
    let sep = separator(conn);
    let items = repos::wishlist::list(conn, None)?;
    let mut out = String::from(CSV_BOM);
    out.push_str(&header(
        &[
            "Producto",
            "Tienda",
            "Enlace",
            "Precio estimado",
            "Moneda",
            "Prioridad",
            "Meta de ahorro",
            "Estado",
            "Fecha",
        ],
        &sep,
    ));
    for i in items {
        out.push('\n');
        out.push_str(&row(
            &[
                i.name.unwrap_or_default(),
                i.store.unwrap_or_default(),
                i.url.unwrap_or_default(),
                i.estimated_price.map(|p| p.to_string()).unwrap_or_default(),
                i.currency_code,
                i.priority.to_string(),
                i.savings_goal.map(|p| p.to_string()).unwrap_or_default(),
                i.status,
                format_date(&i.added_date),
            ],
            &sep,
        ));
    }
    Ok(out)
}
