//! Exportación del reporte financiero a Excel.
//!
//! A diferencia del CSV, este libro está pensado para leerse: incluye un
//! resumen, la evolución del período, las categorías realmente utilizadas y
//! el detalle de los movimientos. Los gráficos usan los colores configurados
//! para cada categoría; el resto del libro mantiene el tema monocromático de
//! Aureon.

use std::collections::HashMap;
use std::path::Path;

use chrono::{Datelike, Duration, NaiveDate};
use rust_xlsxwriter::{
    Chart, ChartDataLabel, ChartFont, ChartLegendPosition, ChartSolidFill, Color, Format,
    FormatAlign, FormatBorder, Workbook,
};

use crate::core::dates;
use crate::core::error::{AppError, AppResult};
use crate::core::models::{Currency, HistorySummary, Transaction, TransactionFilter};
use crate::core::repos;
use crate::core::services::transactions as tx_service;

/// Tope de filas por exportación. Es superior al de la interfaz, igual que en
/// el CSV, para que el libro no se trunque en silencio.
const EXPORT_LIMIT: i64 = 1_000_000;

const INK: u32 = 0x1A1A1A;
const MUTED: u32 = 0x595959;
const SOFT: u32 = 0x8C8C8C;
const HAIRLINE: u32 = 0xD9D9D9;
const PAPER: u32 = 0xF7F7F7;
const PAPER_ALT: u32 = 0xFBFBFB;
const WHITE: u32 = 0xFFFFFF;
const GRAY_SWATCH: u32 = 0xBDBDBD;

const MONTHS: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov",
    "dic",
];

// ---------------------------------------------------------------------------
// Datos del reporte
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct Usage {
    category_id: Option<String>,
    name: String,
    color: Option<String>,
    kind: String,
    count: i64,
    income: i64,
    expenses: i64,
    refunds: i64,
    transfers: i64,
}

impl Usage {
    fn balance(&self) -> i64 {
        self.income - self.expenses + self.refunds
    }
}

#[derive(Debug, Clone)]
struct Bucket {
    key: String,
    label: String,
    income: i64,
    expenses: i64,
    refunds: i64,
    count: i64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Granularity {
    Day,
    Week,
    Month,
    Year,
}

impl Granularity {
    fn label(self) -> &'static str {
        match self {
            Granularity::Day => "día",
            Granularity::Week => "semana",
            Granularity::Month => "mes",
            Granularity::Year => "año",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CategoryMetric {
    Expenses,
    Income,
}

// ---------------------------------------------------------------------------
// Punto de entrada
// ---------------------------------------------------------------------------

/// Genera el libro de Excel en `path` con los movimientos que cumplen
/// `filter`.
pub fn report(
    conn: &rusqlite::Connection,
    path: &Path,
    filter: &TransactionFilter,
    currency: &str,
    period_label: &str,
) -> AppResult<()> {
    let mut export_filter = filter.clone();
    export_filter.limit = Some(EXPORT_LIMIT);
    let transactions = tx_service::list(conn, &export_filter)?;

    let currencies = repos::list_currencies(conn)?;
    let currency_map: HashMap<String, Currency> = currencies
        .into_iter()
        .map(|item| (item.code.clone(), item))
        .collect();
    let selected_currency = currency_map
        .get(currency)
        .cloned()
        .unwrap_or_else(|| Currency {
            code: currency.to_string(),
            name: currency.to_string(),
            symbol: currency.to_string(),
            exponent: 0,
        });

    let categories = repos::categories::list(
        conn,
        None,
        true,
        ("0000-01-01", "9999-12-31"),
        None,
    )?;
    let category_map: HashMap<String, (String, String, Option<String>)> = categories
        .into_iter()
        .map(|item| (item.id, (item.name, item.kind, item.color)))
        .collect();

    let data = collect_data(
        &transactions,
        &category_map,
        filter,
        currency,
        period_label,
    );

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::File(format!("No se pudo crear la carpeta de destino: {e}")))?;
    }

    let mut workbook = Workbook::new();
    write_summary_sheet(&mut workbook, &data, &selected_currency)?;
    write_monthly_sheet(&mut workbook, &data, &selected_currency)?;
    write_categories_sheet(&mut workbook, &data, &selected_currency)?;
    write_movements_sheet(&mut workbook, &transactions, &category_map, &currency_map)?;

    workbook
        .save(path)
        .map_err(|e| AppError::File(format!("No se pudo guardar el libro de Excel: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Agregaciones
// ---------------------------------------------------------------------------

fn collect_data(
    transactions: &[Transaction],
    categories: &HashMap<String, (String, String, Option<String>)>,
    filter: &TransactionFilter,
    currency: &str,
    period_label: &str,
) -> ReportData {
    let filter_start = filter
        .start_date
        .as_deref()
        .and_then(|value| dates::parse_date(value).ok());
    let filter_end = filter
        .end_date
        .as_deref()
        .and_then(|value| dates::parse_date(value).ok());
    let start = filter_start
        .or_else(|| {
            transactions
                .iter()
                .filter_map(|item| dates::parse_date(&item.date).ok())
                .min()
        })
        .unwrap_or_else(dates::today);
    let end = filter_end
        .or_else(|| {
            transactions
                .iter()
                .filter_map(|item| dates::parse_date(&item.date).ok())
                .max()
        })
        .unwrap_or_else(dates::today);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let granularity = choose_granularity((end - start).num_days());
    let mut buckets = build_buckets(start, end, granularity);
    let bucket_index: HashMap<String, usize> = buckets
        .iter()
        .enumerate()
        .map(|(index, bucket)| (bucket.key.clone(), index))
        .collect();

    let mut summary = HistorySummary {
        currency_code: currency.to_string(),
        income: 0,
        expenses: 0,
        net: 0,
        refunds: 0,
        transfers: 0,
        purchases_total: 0,
        installments_paid: 0,
        subscriptions_paid: 0,
        money_received: 0,
        money_paid: 0,
        pending_commitments: 0,
        overdue_commitments: 0,
        transaction_count: 0,
    };
    let mut usages: HashMap<String, Usage> = HashMap::new();

    for item in transactions {
        if item.voided || item.currency_code != currency {
            continue;
        }

        summary.transaction_count += 1;
        match item.transaction_type.as_str() {
            "income" => {
                summary.income += item.amount;
                if item.linked_type.as_deref() == Some("receivable") {
                    summary.money_received += item.amount;
                }
            }
            "expense" => {
                summary.expenses += item.amount;
                match item.linked_type.as_deref() {
                    Some("purchase") => summary.purchases_total += item.amount,
                    Some("installment") => summary.installments_paid += item.amount,
                    Some("subscription") => summary.subscriptions_paid += item.amount,
                    Some("payable") => summary.money_paid += item.amount,
                    _ => {}
                }
            }
            "refund" => summary.refunds += item.amount,
            "transfer" => summary.transfers += item.amount,
            _ => {}
        }

        if item.transaction_type == "income"
            || item.transaction_type == "expense"
            || item.transaction_type == "refund"
            || item.transaction_type == "transfer"
        {
            let key = bucket_key(&item.date, granularity);
            if let Some(index) = bucket_index.get(&key).copied() {
                match item.transaction_type.as_str() {
                    "income" => buckets[index].income += item.amount,
                    "expense" => buckets[index].expenses += item.amount,
                    "refund" => buckets[index].refunds += item.amount,
                    _ => {}
                }
                buckets[index].count += 1;
            }
        }

        let (key, fallback_name, fallback_kind, fallback_color) = match item.category_id.as_ref() {
            Some(id) => {
                let (name, kind, color) = categories
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| (String::new(), String::new(), None));
                (
                    id.clone(),
                    item.category_name
                        .clone()
                        .filter(|value| !value.trim().is_empty())
                        .or_else(|| Some(name).filter(|value| !value.trim().is_empty()))
                        .unwrap_or_else(|| "Sin nombre".to_string()),
                    kind,
                    color,
                )
            }
            None => (
                "__none__".to_string(),
                "Sin categoría".to_string(),
                "none".to_string(),
                None,
            ),
        };

        let usage = usages.entry(key).or_insert_with(|| Usage {
            category_id: item.category_id.clone(),
            name: fallback_name,
            color: fallback_color,
            kind: fallback_kind,
            ..Usage::default()
        });
        usage.count += 1;
        match item.transaction_type.as_str() {
            "income" => usage.income += item.amount,
            "expense" => usage.expenses += item.amount,
            "refund" => usage.refunds += item.amount,
            "transfer" => usage.transfers += item.amount,
            _ => {}
        }
    }

    summary.net = summary.income - summary.expenses + summary.refunds;

    let mut category_rows: Vec<Usage> = usages.into_values().collect();
    category_rows.sort_by(|a, b| {
        b.expenses
            .cmp(&a.expenses)
            .then_with(|| b.income.cmp(&a.income))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let expense_total: i64 = category_rows.iter().map(|item| item.expenses).sum();
    let income_total: i64 = category_rows.iter().map(|item| item.income).sum();
    let metric = if expense_total > 0 {
        Some(CategoryMetric::Expenses)
    } else if income_total > 0 {
        Some(CategoryMetric::Income)
    } else {
        None
    };

    ReportData {
        summary,
        buckets,
        categories: category_rows,
        metric,
        granularity,
        period_label: period_label.trim().to_string(),
        range_label: format!(
            "{} – {}",
            display_date(start),
            display_date(end)
        ),
        generated_at: display_date(dates::today()),
        start,
        end,
    }
}

#[derive(Debug, Clone)]
struct ReportData {
    summary: HistorySummary,
    buckets: Vec<Bucket>,
    categories: Vec<Usage>,
    metric: Option<CategoryMetric>,
    granularity: Granularity,
    period_label: String,
    range_label: String,
    generated_at: String,
    start: NaiveDate,
    end: NaiveDate,
}

fn choose_granularity(days: i64) -> Granularity {
    if days <= 14 {
        Granularity::Day
    } else if days <= 120 {
        Granularity::Week
    } else if days <= 730 {
        Granularity::Month
    } else {
        Granularity::Year
    }
}

fn build_buckets(start: NaiveDate, end: NaiveDate, granularity: Granularity) -> Vec<Bucket> {
    let mut buckets = Vec::new();
    match granularity {
        Granularity::Day => {
            let mut cursor = start;
            while cursor <= end && buckets.len() < 10_000 {
                buckets.push(Bucket {
                    key: cursor.format("%Y-%m-%d").to_string(),
                    label: if start.year() == end.year() {
                        cursor.format("%d/%m").to_string()
                    } else {
                        cursor.format("%d/%m/%y").to_string()
                    },
                    income: 0,
                    expenses: 0,
                    refunds: 0,
                    count: 0,
                });
                cursor += Duration::days(1);
            }
        }
        Granularity::Week => {
            let offset = i64::from(start.weekday().num_days_from_monday());
            let mut cursor = start - Duration::days(offset);
            while cursor <= end && buckets.len() < 10_000 {
                buckets.push(Bucket {
                    key: cursor.format("%Y-%m-%d").to_string(),
                    label: if start.year() == end.year() {
                        cursor.format("%d/%m").to_string()
                    } else {
                        cursor.format("%d/%m/%y").to_string()
                    },
                    income: 0,
                    expenses: 0,
                    refunds: 0,
                    count: 0,
                });
                cursor += Duration::days(7);
            }
        }
        Granularity::Month => {
            let mut cursor = NaiveDate::from_ymd_opt(start.year(), start.month(), 1)
                .unwrap_or(start);
            while cursor <= end && buckets.len() < 10_000 {
                let month = (cursor.month() - 1) as usize;
                buckets.push(Bucket {
                    key: cursor.format("%Y-%m").to_string(),
                    label: format!("{} {}", MONTHS[month], cursor.year()),
                    income: 0,
                    expenses: 0,
                    refunds: 0,
                    count: 0,
                });
                cursor = next_month(cursor);
            }
        }
        Granularity::Year => {
            let mut year = start.year();
            while year <= end.year() && buckets.len() < 10_000 {
                buckets.push(Bucket {
                    key: year.to_string(),
                    label: year.to_string(),
                    income: 0,
                    expenses: 0,
                    refunds: 0,
                    count: 0,
                });
                year += 1;
            }
        }
    }
    buckets
}

fn next_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1).unwrap_or(date)
}

fn bucket_key(raw: &str, granularity: Granularity) -> String {
    match dates::parse_date(raw) {
        Ok(date) => match granularity {
            Granularity::Day => date.format("%Y-%m-%d").to_string(),
            Granularity::Week => {
                let offset = i64::from(date.weekday().num_days_from_monday());
                (date - Duration::days(offset))
                    .format("%Y-%m-%d")
                    .to_string()
            }
            Granularity::Month => date.format("%Y-%m").to_string(),
            Granularity::Year => date.format("%Y").to_string(),
        },
        Err(_) => raw.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Hojas
// ---------------------------------------------------------------------------

fn write_summary_sheet(
    workbook: &mut Workbook,
    data: &ReportData,
    currency: &Currency,
) -> AppResult<()> {
    let ws = workbook.add_worksheet();
    let money = money_format(currency);
    let axis_money = axis_money_format(currency);

    let title = Format::new()
        .set_font_name("Calibri")
        .set_font_size(24)
        .set_bold()
        .set_font_color(Color::RGB(INK));
    let subtitle = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_font_color(Color::RGB(MUTED));
    let right_meta = Format::new()
        .set_font_name("Calibri")
        .set_font_size(9)
        .set_font_color(Color::RGB(MUTED))
        .set_align(FormatAlign::Right);
    let section = Format::new()
        .set_font_name("Calibri")
        .set_font_size(11)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_border_bottom(FormatBorder::Thin)
        .set_border_bottom_color(Color::RGB(HAIRLINE));
    let kpi_label = Format::new()
        .set_font_name("Calibri")
        .set_font_size(8)
        .set_font_color(Color::RGB(MUTED))
        .set_background_color(Color::RGB(PAPER))
        .set_border_left(FormatBorder::Thick)
        .set_border_left_color(Color::RGB(INK))
        .set_indent(1);
    let kpi_value = Format::new()
        .set_font_name("Calibri")
        .set_font_size(17)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_left(FormatBorder::Thick)
        .set_border_left_color(Color::RGB(INK))
        .set_num_format(money.clone())
        .set_indent(1);
    let kpi_count = Format::new()
        .set_font_name("Calibri")
        .set_font_size(17)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_left(FormatBorder::Thick)
        .set_border_left_color(Color::RGB(INK))
        .set_num_format("#,##0")
        .set_indent(1);
    let kpi_meta = Format::new()
        .set_font_name("Calibri")
        .set_font_size(8)
        .set_font_color(Color::RGB(SOFT))
        .set_background_color(Color::RGB(PAPER))
        .set_border_left(FormatBorder::Thick)
        .set_border_left_color(Color::RGB(INK))
        .set_indent(1);
    let secondary_label = Format::new()
        .set_font_name("Calibri")
        .set_font_size(8)
        .set_font_color(Color::RGB(MUTED))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(HAIRLINE));
    let secondary_money = Format::new()
        .set_font_name("Calibri")
        .set_font_size(13)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(HAIRLINE))
        .set_num_format(money.clone());
    let secondary_count = Format::new()
        .set_font_name("Calibri")
        .set_font_size(13)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(HAIRLINE))
        .set_num_format("#,##0");
    let note = Format::new()
        .set_font_name("Calibri")
        .set_font_size(8)
        .set_italic()
        .set_font_color(Color::RGB(SOFT));

    ws.set_name("Resumen")?;
    ws.set_screen_gridlines(false);
    ws.set_zoom(85);
    ws.set_tab_color(Color::RGB(INK));
    ws.set_column_width(0, 2)?;
    for col in [1_u16, 3, 5, 7] {
        ws.set_column_width(col, 23)?;
    }
    for col in [2_u16, 4, 6, 8, 9, 10] {
        ws.set_column_width(col, 2.5)?;
    }
    ws.set_row_height(1, 38)?;
    ws.set_row_height(2, 18)?;
    ws.set_row_height(3, 30)?;
    ws.set_row_height(7, 10)?;

    ws.merge_range(1, 1, 1, 5, "Aureon", &title)?;
    ws.merge_range(2, 1, 2, 5, "Reporte financiero", &subtitle)?;
    ws.merge_range(1, 7, 1, 10, &data.period_label_or_range(), &right_meta)?;
    ws.merge_range(
        2,
        7,
        2,
        10,
        &format!("{} · Moneda {}", data.range_label, currency.code),
        &right_meta,
    )?;
    ws.merge_range(
        3,
        7,
        3,
        10,
        &format!(
            "Generado el {} · {} movimientos",
            data.generated_at, data.summary.transaction_count
        ),
        &right_meta,
    )?;

    // Tarjetas principales. Cada una ocupa una columna ancha, separada por
    // una columna estrecha para que el bloque respire.
    let cards = [
        (1_u16, "INGRESOS", data.summary.income, true, "Dinero recibido en el período"),
        (3_u16, "GASTOS", data.summary.expenses, true, "Dinero pagado en el período"),
        (5_u16, "BALANCE", data.summary.net, true, "Ingresos menos gastos, más devoluciones"),
        (7_u16, "MOVIMIENTOS", data.summary.transaction_count, false, "Filas incluidas en el reporte"),
    ];
    for (col, label, amount, is_money, meta) in cards {
        ws.write_string_with_format(8, col, label, &kpi_label)?;
        ws.write_string_with_format(9, col, "", if is_money { &kpi_value } else { &kpi_count })?;
        if is_money {
            ws.write_number_with_format(9, col, to_units(amount, currency.exponent), &kpi_value)?;
        } else {
            ws.write_number_with_format(9, col, amount as f64, &kpi_count)?;
        }
        ws.write_string_with_format(10, col, meta, &kpi_meta)?;
    }

    ws.merge_range(12, 1, 12, 10, "Ingresos y gastos", &section)?;

    // El gráfico se alimenta de la hoja Mensual.
    let last = 5 + data.buckets.len() as u32;
    let mut chart = Chart::new_column();
    chart
        .title()
        .set_name("Evolución del período");
    chart
        .set_width(880)
        .set_height(390)
        .set_style(10);
    {
        let mut title_font = ChartFont::new();
        title_font
            .set_size(12)
            .set_bold()
            .set_color(Color::RGB(INK));
        chart.title().set_font(&title_font);
    }
    {
        let mut axis_font = ChartFont::new();
        axis_font.set_size(8).set_color(Color::RGB(MUTED));
        chart.x_axis().set_font(&axis_font);
        chart.y_axis().set_font(&axis_font);
        chart.y_axis().set_num_format(&axis_money);
    }
    chart
        .x_axis()
        .set_label_interval(label_interval(data.buckets.len()));
    chart
        .legend()
        .set_position(ChartLegendPosition::Bottom)
        .set_font(&label_font());
    chart
        .add_series()
        .set_name("Ingresos")
        .set_categories(&format!("Mensual!$B$6:$B${last}"))
        .set_values(&format!("Mensual!$C$6:$C${last}"))
        .set_format(ChartSolidFill::new().set_color(Color::RGB(INK)))
        .set_gap(65);
    chart
        .add_series()
        .set_name("Gastos")
        .set_categories(&format!("Mensual!$B$6:$B${last}"))
        .set_values(&format!("Mensual!$D$6:$D${last}"))
        .set_format(ChartSolidFill::new().set_color(Color::RGB(0xA6A6A6)));

    ws.insert_chart(14, 1, &chart)?;

    ws.merge_range(38, 1, 38, 10, "Otros indicadores", &section)?;

    let secondary = [
        (39_u32, 1_u16, "Devoluciones", data.summary.refunds, true),
        (39, 3, "Transferencias", data.summary.transfers, true),
        (39, 5, "Compras", data.summary.purchases_total, true),
        (39, 7, "Cuotas pagadas", data.summary.installments_paid, true),
        (41, 1, "Suscripciones", data.summary.subscriptions_paid, true),
        (41, 3, "Dinero recibido", data.summary.money_received, true),
        (41, 5, "Dinero pagado", data.summary.money_paid, true),
        (41, 7, "Movimientos", data.summary.transaction_count, false),
    ];
    for (row, col, label, amount, is_money) in secondary {
        ws.write_string_with_format(row, col, label, &secondary_label)?;
        if is_money {
            ws.write_number_with_format(
                row + 1,
                col,
                to_units(amount, currency.exponent),
                &secondary_money,
            )?;
        } else {
            ws.write_number_with_format(row + 1, col, amount as f64, &secondary_count)?;
        }
    }

    ws.merge_range(
        45,
        1,
        45,
        10,
        "Los colores de las categorías provienen de la configuración de Aureon. Los importes se muestran en la moneda seleccionada.",
        &note,
    )?;

    Ok(())
}

fn write_monthly_sheet(
    workbook: &mut Workbook,
    data: &ReportData,
    currency: &Currency,
) -> AppResult<()> {
    let ws = workbook.add_worksheet();
    let money = money_format(currency);
    let title = Format::new()
        .set_font_name("Calibri")
        .set_font_size(20)
        .set_bold()
        .set_font_color(Color::RGB(INK));
    let subtitle = Format::new()
        .set_font_name("Calibri")
        .set_font_size(9)
        .set_font_color(Color::RGB(MUTED));
    let header = header_format(FormatAlign::Right);
    let header_left = header_format(FormatAlign::Left);
    let cell_text = body_format(FormatAlign::Left, false);
    let cell_text_alt = body_format(FormatAlign::Left, true);
    let cell_number = body_number_format(&money, false, false);
    let cell_number_alt = body_number_format(&money, true, false);
    let cell_count = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_font_color(Color::RGB(INK))
        .set_align(FormatAlign::Right)
        .set_num_format("#,##0")
        .set_border_bottom(FormatBorder::Thin)
        .set_border_bottom_color(Color::RGB(HAIRLINE));
    let cell_count_alt = cell_count.clone().set_background_color(Color::RGB(PAPER_ALT));
    let total_label = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(INK));
    let total_number = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(INK))
        .set_num_format(money.clone())
        .set_align(FormatAlign::Right);
    let total_count_style = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(INK))
        .set_num_format("#,##0")
        .set_align(FormatAlign::Right);
    let note = Format::new()
        .set_font_name("Calibri")
        .set_font_size(8)
        .set_italic()
        .set_font_color(Color::RGB(SOFT));

    ws.set_name("Mensual")?;
    ws.set_screen_gridlines(false);
    ws.set_zoom(90);
    ws.set_tab_color(Color::RGB(SOFT));
    ws.set_column_width(0, 2)?;
    ws.set_column_width(1, 20)?;
    ws.set_column_width(2, 18)?;
    ws.set_column_width(3, 18)?;
    ws.set_column_width(4, 15)?;
    ws.set_column_width(5, 18)?;
    ws.set_column_width(6, 13)?;
    ws.set_row_height(1, 30)?;
    ws.set_row_height(2, 18)?;
    ws.set_row_height(4, 24)?;

    ws.merge_range(1, 1, 1, 6, "Evolución del período", &title)?;
    ws.merge_range(
        2,
        1,
        2,
        6,
        &format!(
            "Agrupado por {} · {} a {}",
            data.granularity.label(),
            display_date(data.start),
            display_date(data.end)
        ),
        &subtitle,
    )?;

    let labels = [
        "Período",
        "Ingresos",
        "Gastos",
        "Devoluciones",
        "Balance",
        "Movimientos",
    ];
    for (index, label) in labels.iter().enumerate() {
        let format = if index == 0 { &header_left } else { &header };
        ws.write_string_with_format(4, (index + 1) as u16, *label, format)?;
    }

    let mut row = 5_u32;
    let (mut total_income, mut total_expenses, mut total_refunds, mut total_count) =
        (0_i64, 0_i64, 0_i64, 0_i64);
    for (index, bucket) in data.buckets.iter().enumerate() {
        let alt = index % 2 == 1;
        total_income += bucket.income;
        total_expenses += bucket.expenses;
        total_refunds += bucket.refunds;
        total_count += bucket.count;

        ws.write_string_with_format(
            row,
            1,
            &bucket.label,
            if alt { &cell_text_alt } else { &cell_text },
        )?;
        ws.write_number_with_format(
            row,
            2,
            to_units(bucket.income, currency.exponent),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        ws.write_number_with_format(
            row,
            3,
            to_units(bucket.expenses, currency.exponent),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        ws.write_number_with_format(
            row,
            4,
            to_units(bucket.refunds, currency.exponent),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        ws.write_number_with_format(
            row,
            5,
            to_units(
                bucket.income - bucket.expenses + bucket.refunds,
                currency.exponent,
            ),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        ws.write_number_with_format(
            row,
            6,
            bucket.count as f64,
            if alt { &cell_count_alt } else { &cell_count },
        )?;
        row += 1;
    }

    ws.write_string_with_format(row, 1, "Total", &total_label)?;
    ws.write_number_with_format(
        row,
        2,
        to_units(total_income, currency.exponent),
        &total_number,
    )?;
    ws.write_number_with_format(
        row,
        3,
        to_units(total_expenses, currency.exponent),
        &total_number,
    )?;
    ws.write_number_with_format(
        row,
        4,
        to_units(total_refunds, currency.exponent),
        &total_number,
    )?;
    ws.write_number_with_format(
        row,
        5,
        to_units(
            total_income - total_expenses + total_refunds,
            currency.exponent,
        ),
        &total_number,
    )?;
    ws.write_number_with_format(row, 6, total_count as f64, &total_count_style)?;

    let last_data_row = 5 + data.buckets.len() as u32;
    let mut chart = Chart::new_column();
    chart
        .title()
        .set_name("Ingresos y gastos");
    chart
        .set_width(760)
        .set_height(360)
        .set_style(10);
    chart
        .add_series()
        .set_name("Ingresos")
        .set_categories(&format!("Mensual!$B$6:$B${last_data_row}"))
        .set_values(&format!("Mensual!$C$6:$C${last_data_row}"))
        .set_format(ChartSolidFill::new().set_color(Color::RGB(INK)))
        .set_gap(65);
    chart
        .add_series()
        .set_name("Gastos")
        .set_categories(&format!("Mensual!$B$6:$B${last_data_row}"))
        .set_values(&format!("Mensual!$D$6:$D${last_data_row}"))
        .set_format(ChartSolidFill::new().set_color(Color::RGB(0xA6A6A6)));
    chart
        .legend()
        .set_position(ChartLegendPosition::Bottom)
        .set_font(&label_font());
    chart
        .x_axis()
        .set_font(&axis_font())
        .set_label_interval(label_interval(data.buckets.len()));
    chart
        .y_axis()
        .set_font(&axis_font())
        .set_num_format(&axis_money_format(currency));

    ws.insert_chart(row + 3, 1, &chart)?;
    ws.merge_range(
        row + 23,
        1,
        row + 23,
        5,
        "Este gráfico también alimenta el resumen del libro.",
        &note,
    )?;

    Ok(())
}

fn write_categories_sheet(
    workbook: &mut Workbook,
    data: &ReportData,
    currency: &Currency,
) -> AppResult<()> {
    let ws = workbook.add_worksheet();
    let money = money_format(currency);
    let title = Format::new()
        .set_font_name("Calibri")
        .set_font_size(20)
        .set_bold()
        .set_font_color(Color::RGB(INK));
    let subtitle = Format::new()
        .set_font_name("Calibri")
        .set_font_size(9)
        .set_font_color(Color::RGB(MUTED));
    let header = header_format(FormatAlign::Right);
    let header_left = header_format(FormatAlign::Left);
    let header_center = header_format(FormatAlign::Center);
    let cell_text = body_format(FormatAlign::Left, false);
    let cell_text_alt = body_format(FormatAlign::Left, true);
    let cell_center = body_format(FormatAlign::Center, false);
    let cell_center_alt = body_format(FormatAlign::Center, true);
    let cell_number = body_number_format(&money, false, false);
    let cell_number_alt = body_number_format(&money, true, false);
    let cell_count = body_number_format(&money, false, true);
    let cell_count_alt = body_number_format(&money, true, true);
    let cell_percent = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_font_color(Color::RGB(MUTED))
        .set_align(FormatAlign::Right)
        .set_num_format("0.0%")
        .set_border_bottom(FormatBorder::Thin)
        .set_border_bottom_color(Color::RGB(HAIRLINE));
    let cell_percent_alt = cell_percent
        .clone()
        .set_background_color(Color::RGB(PAPER_ALT));
    let total_label = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(INK));
    let total_money = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(INK))
        .set_num_format(money.clone())
        .set_align(FormatAlign::Right);
    let total_count_style = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(INK))
        .set_num_format("#,##0")
        .set_align(FormatAlign::Right);
    let total_percent = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_bold()
        .set_font_color(Color::RGB(INK))
        .set_background_color(Color::RGB(PAPER))
        .set_border_top(FormatBorder::Thin)
        .set_border_top_color(Color::RGB(INK))
        .set_num_format("0.0%")
        .set_align(FormatAlign::Right);
    let note = Format::new()
        .set_font_name("Calibri")
        .set_font_size(8)
        .set_italic()
        .set_font_color(Color::RGB(SOFT));

    ws.set_name("Categorías")?;
    ws.set_screen_gridlines(false);
    ws.set_zoom(90);
    ws.set_tab_color(Color::RGB(0x606060));
    ws.set_column_width(0, 2)?;
    ws.set_column_width(1, 3.5)?;
    ws.set_column_width(2, 28)?;
    ws.set_column_width(3, 13)?;
    ws.set_column_width(4, 12)?;
    ws.set_column_width(5, 17)?;
    ws.set_column_width(6, 17)?;
    ws.set_column_width(7, 15)?;
    ws.set_column_width(8, 17)?;
    ws.set_column_width(9, 14)?;
    ws.set_column_width(10, 2)?;
    ws.set_column_width(11, 2)?;
    ws.set_row_height(1, 30)?;
    ws.set_row_height(2, 18)?;
    ws.set_row_height(4, 24)?;

    ws.merge_range(1, 1, 1, 9, "Categorías utilizadas", &title)?;
    ws.merge_range(
        2,
        1,
        2,
        9,
        "Solo aparecen las categorías con movimientos en el período y la moneda seleccionados.",
        &subtitle,
    )?;

    let metric_label = match data.metric {
        Some(CategoryMetric::Income) => "Participación sobre los ingresos",
        _ => "Participación sobre los gastos",
    };
    let headers = [
        "Color",
        "Categoría",
        "Tipo",
        "Movs.",
        "Ingresos",
        "Gastos",
        "Devoluciones",
        "Balance",
        "Participación",
    ];
    for (index, label) in headers.iter().enumerate() {
        let format = match index {
            0 => &header_center,
            1 | 2 => &header_left,
            _ => &header,
        };
        let col = (index + 1) as u16;
        ws.write_string_with_format(4, col, *label, format)?;
    }
    ws.merge_range(3, 1, 3, 9, metric_label, &subtitle)?;

    let mut row = 5_u32;
    let (mut total_count, mut total_income, mut total_expenses, mut total_refunds) =
        (0_i64, 0_i64, 0_i64, 0_i64);
    let denominator = match data.metric {
        Some(CategoryMetric::Income) => data
            .categories
            .iter()
            .map(|item| item.income)
            .sum::<i64>(),
        _ => data
            .categories
            .iter()
            .map(|item| item.expenses)
            .sum::<i64>(),
    };

    for (index, item) in data.categories.iter().enumerate() {
        let alt = index % 2 == 1;
        total_count += item.count;
        total_income += item.income;
        total_expenses += item.expenses;
        total_refunds += item.refunds;
        let rgb = parse_color(item.color.as_deref()).unwrap_or(GRAY_SWATCH);
        let swatch = Format::new()
            .set_background_color(Color::RGB(rgb))
            .set_border(FormatBorder::Thin)
            .set_border_color(Color::RGB(HAIRLINE));
        ws.write_blank(row, 1, &swatch)?;
        ws.write_string_with_format(
            row,
            2,
            &clip(&item.name, 120),
            if alt { &cell_text_alt } else { &cell_text },
        )?;
        ws.write_string_with_format(
            row,
            3,
            &category_kind_label(item),
            if alt { &cell_center_alt } else { &cell_center },
        )?;
        ws.write_number_with_format(
            row,
            4,
            item.count as f64,
            if alt { &cell_count_alt } else { &cell_count },
        )?;
        ws.write_number_with_format(
            row,
            5,
            to_units(item.income, currency.exponent),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        ws.write_number_with_format(
            row,
            6,
            to_units(item.expenses, currency.exponent),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        ws.write_number_with_format(
            row,
            7,
            to_units(item.refunds, currency.exponent),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        ws.write_number_with_format(
            row,
            8,
            to_units(item.balance(), currency.exponent),
            if alt { &cell_number_alt } else { &cell_number },
        )?;
        let share = if denominator > 0 {
            let value = match data.metric {
                Some(CategoryMetric::Income) => item.income,
                _ => item.expenses,
            };
            value as f64 / denominator as f64
        } else {
            0.0
        };
        ws.write_number_with_format(
            row,
            9,
            share,
            if alt { &cell_percent_alt } else { &cell_percent },
        )?;
        row += 1;
    }

    if data.categories.is_empty() {
        ws.merge_range(
            5,
            1,
            5,
            9,
            "No hay movimientos con los filtros seleccionados.",
            &note,
        )?;
        row = 6;
    } else {
        ws.write_string_with_format(row, 1, "", &total_label)?;
        ws.write_string_with_format(row, 2, "Total", &total_label)?;
        ws.write_string_with_format(row, 3, "", &total_label)?;
        ws.write_number_with_format(row, 4, total_count as f64, &total_count_style)?;
        ws.write_number_with_format(
            row,
            5,
            to_units(total_income, currency.exponent),
            &total_money,
        )?;
        ws.write_number_with_format(
            row,
            6,
            to_units(total_expenses, currency.exponent),
            &total_money,
        )?;
        ws.write_number_with_format(
            row,
            7,
            to_units(total_refunds, currency.exponent),
            &total_money,
        )?;
        ws.write_number_with_format(
            row,
            8,
            to_units(total_income - total_expenses + total_refunds, currency.exponent),
            &total_money,
        )?;
        ws.write_number_with_format(row, 9, 1.0, &total_percent)?;
    }

    // El gráfico de anillo usa solo las categorías con valor positivo del
    // indicador elegido. El orden de la tabla deja esas filas al principio.
    let chart_count = match data.metric {
        Some(CategoryMetric::Expenses) => data
            .categories
            .iter()
            .take_while(|item| item.expenses > 0)
            .count(),
        Some(CategoryMetric::Income) => data
            .categories
            .iter()
            .take_while(|item| item.income > 0)
            .count(),
        None => 0,
    };
    if chart_count > 0 {
        let chart_last = 6 + chart_count as u32 - 1;
        let values_column = match data.metric {
            Some(CategoryMetric::Income) => "F",
            _ => "G",
        };
        let chart_title = match data.metric {
            Some(CategoryMetric::Income) => "Ingresos por categoría",
            _ => "Gastos por categoría",
        };
        let colors: Vec<Color> = data
            .categories
            .iter()
            .take(chart_count)
            .map(|item| Color::RGB(parse_color(item.color.as_deref()).unwrap_or(GRAY_SWATCH)))
            .collect();

        let mut chart = Chart::new_doughnut();
        chart
            .title()
            .set_name(chart_title);
        chart
            .set_hole_size(58)
            .set_width(600)
            .set_height(390)
            .set_style(10);
        {
            let mut chart_title_font = ChartFont::new();
            chart_title_font
                .set_size(12)
                .set_bold()
                .set_color(Color::RGB(INK));
            chart.title().set_font(&chart_title_font);
        }
        chart
            .legend()
            .set_position(ChartLegendPosition::Right)
            .set_font(&label_font());

        {
            let series = chart
                .add_series()
                .set_name(chart_title)
                .set_categories(&format!("Categorías!$C$6:$C${chart_last}"))
                .set_values(&format!(
                    "Categorías!${values_column}$6:${values_column}${chart_last}"
                ))
                .set_point_colors(&colors);

            // Con muchas categorías las etiquetas se enciman sobre el anillo.
            // La tabla ya muestra la participación exacta, así que se omiten.
            if chart_count <= 10 {
                let mut data_label = ChartDataLabel::new();
                data_label.show_percentage().set_font(&label_font());
                series.set_data_label(&data_label);
            }
        }

        ws.insert_chart(5, 11, &chart)?;
    }

    let note_row = if data.categories.is_empty() {
        8
    } else {
        row + 2
    };
    ws.merge_range(
        note_row,
        1,
        note_row,
        9,
        "Los colores de esta hoja y del gráfico son los que configuraste en Aureon.",
        &note,
    )?;

    Ok(())
}

fn write_movements_sheet(
    workbook: &mut Workbook,
    transactions: &[Transaction],
    categories: &HashMap<String, (String, String, Option<String>)>,
    currencies: &HashMap<String, Currency>,
) -> AppResult<()> {
    let ws = workbook.add_worksheet();
    let header = header_format(FormatAlign::Left);
    let cell_text = body_format(FormatAlign::Left, false);
    let cell_text_alt = body_format(FormatAlign::Left, true);
    let cell_center = body_format(FormatAlign::Center, false);
    let cell_center_alt = body_format(FormatAlign::Center, true);
    let date_format = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_font_color(Color::RGB(INK))
        .set_align(FormatAlign::Left)
        .set_num_format("dd/mm/yyyy")
        .set_border_bottom(FormatBorder::Thin)
        .set_border_bottom_color(Color::RGB(HAIRLINE));
    let date_format_alt = date_format
        .clone()
        .set_background_color(Color::RGB(PAPER_ALT));

    ws.set_name("Movimientos")?;
    ws.set_screen_gridlines(false);
    ws.set_zoom(90);
    ws.set_tab_color(Color::RGB(HAIRLINE));
    ws.set_column_width(0, 2)?;
    ws.set_column_width(1, 12)?;
    ws.set_column_width(2, 14)?;
    ws.set_column_width(3, 42)?;
    ws.set_column_width(4, 5)?;
    ws.set_column_width(5, 22)?;
    ws.set_column_width(6, 20)?;
    ws.set_column_width(7, 20)?;
    ws.set_column_width(8, 22)?;
    ws.set_column_width(9, 18)?;
    ws.set_column_width(10, 18)?;
    ws.set_column_width(11, 10)?;
    ws.set_column_width(12, 14)?;
    ws.set_column_width(13, 40)?;
    ws.set_row_height(0, 26)?;

    let headers = [
        "Fecha",
        "Tipo",
        "Descripción",
        "Color",
        "Categoría",
        "Cuenta",
        "Cuenta destino",
        "Comercio o persona",
        "Método de pago",
        "Monto",
        "Moneda",
        "Estado",
        "Notas",
    ];
    for (index, label) in headers.iter().enumerate() {
        let col = (index + 1) as u16;
        ws.write_string_with_format(0, col, *label, &header)?;
    }

    let mut money_cache: HashMap<String, (Format, Format)> = HashMap::new();
    for (index, item) in transactions.iter().enumerate() {
        let row = 1 + index as u32;
        let alt = index % 2 == 1;
        let (normal_money, alt_money) = money_cache
            .entry(item.currency_code.clone())
            .or_insert_with(|| {
                let currency = currencies
                    .get(&item.currency_code)
                    .cloned()
                    .unwrap_or_else(|| Currency {
                        code: item.currency_code.clone(),
                        name: item.currency_code.clone(),
                        symbol: item.currency_code.clone(),
                        exponent: 0,
                    });
                let format = money_format(&currency);
                (
                    body_number_format(&format, false, false),
                    body_number_format(&format, true, false),
                )
            });
        let money = if alt { alt_money } else { normal_money };
        let text = if alt { &cell_text_alt } else { &cell_text };
        let center = if alt { &cell_center_alt } else { &cell_center };
        let date_style = if alt { &date_format_alt } else { &date_format };

        let date = dates::parse_date(&item.date).ok();
        if let Some(date) = date {
            ws.write_datetime_with_format(row, 1, date, date_style)?;
        } else {
            ws.write_string_with_format(row, 1, &item.date, date_style)?;
        }

        ws.write_string_with_format(row, 2, type_label(&item.transaction_type), center)?;
        ws.write_string_with_format(row, 3, &clip(&item.description, 500), text)?;

        let rgb = item
            .category_id
            .as_ref()
            .and_then(|id| categories.get(id))
            .and_then(|(_, _, color)| parse_color(color.as_deref()))
            .unwrap_or(GRAY_SWATCH);
        let swatch = Format::new()
            .set_background_color(Color::RGB(rgb))
            .set_border(FormatBorder::Thin)
            .set_border_color(Color::RGB(HAIRLINE))
            .set_align(FormatAlign::Center);
        ws.write_blank(row, 4, &swatch)?;

        ws.write_string_with_format(
            row,
            5,
            &clip(
                item.category_name.as_deref().unwrap_or("Sin categoría"),
                120,
            ),
            text,
        )?;
        ws.write_string_with_format(
            row,
            6,
            &clip(item.account_name.as_deref().unwrap_or(""), 120),
            text,
        )?;
        ws.write_string_with_format(
            row,
            7,
            &clip(item.transfer_account_name.as_deref().unwrap_or(""), 120),
            text,
        )?;
        ws.write_string_with_format(
            row,
            8,
            &clip(item.contact_name.as_deref().unwrap_or(""), 120),
            text,
        )?;
        ws.write_string_with_format(
            row,
            9,
            &clip(item.payment_method.as_deref().unwrap_or(""), 80),
            text,
        )?;

        let signed = if item.transaction_type == "expense" {
            -(item.amount as f64)
        } else {
            item.amount as f64
        };
        ws.write_number_with_format(
            row,
            10,
            to_units_f64(signed, item_currency_exponent(item, currencies)),
            money,
        )?;
        ws.write_string_with_format(row, 11, &item.currency_code, center)?;
        ws.write_string_with_format(
            row,
            12,
            if item.voided { "Anulado" } else { "Vigente" },
            center,
        )?;
        ws.write_string_with_format(row, 13, &clip(item.notes.as_deref().unwrap_or(""), 2000), text)?;
    }

    if !transactions.is_empty() {
        ws.autofilter(0, 1, transactions.len() as u32, 13)?;
        ws.set_freeze_panes(1, 3)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Formatos y utilidades
// ---------------------------------------------------------------------------

impl ReportData {
    fn period_label_or_range(&self) -> String {
        if self.period_label.is_empty() {
            self.range_label.clone()
        } else {
            self.period_label.clone()
        }
    }
}

fn header_format(align: FormatAlign) -> Format {
    Format::new()
        .set_font_name("Calibri")
        .set_font_size(9)
        .set_bold()
        .set_font_color(Color::RGB(WHITE))
        .set_background_color(Color::RGB(INK))
        .set_align(align)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_bottom(FormatBorder::Thin)
        .set_border_bottom_color(Color::RGB(INK))
}

fn body_format(align: FormatAlign, alt: bool) -> Format {
    let format = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_font_color(Color::RGB(INK))
        .set_align(align)
        .set_align(FormatAlign::VerticalCenter)
        .set_border_bottom(FormatBorder::Thin)
        .set_border_bottom_color(Color::RGB(HAIRLINE));
    if alt {
        format.set_background_color(Color::RGB(PAPER_ALT))
    } else {
        format
    }
}

fn body_number_format(num_format: &str, alt: bool, integer: bool) -> Format {
    let format = Format::new()
        .set_font_name("Calibri")
        .set_font_size(10)
        .set_font_color(Color::RGB(INK))
        .set_align(FormatAlign::Right)
        .set_align(FormatAlign::VerticalCenter)
        .set_num_format(if integer { "#,##0" } else { num_format })
        .set_border_bottom(FormatBorder::Thin)
        .set_border_bottom_color(Color::RGB(HAIRLINE));
    if alt {
        format.set_background_color(Color::RGB(PAPER_ALT))
    } else {
        format
    }
}

fn label_font() -> ChartFont {
    let mut font = ChartFont::new();
    font.set_size(8).set_color(Color::RGB(MUTED));
    font
}

fn axis_font() -> ChartFont {
    let mut font = ChartFont::new();
    font.set_size(8).set_color(Color::RGB(MUTED));
    font
}

fn label_interval(count: usize) -> u16 {
    if count <= 8 {
        1
    } else if count <= 16 {
        2
    } else if count <= 32 {
        3
    } else {
        5
    }
}

fn money_format(currency: &Currency) -> String {
    let zeros = if currency.exponent > 0 {
        format!(".{}", "0".repeat(currency.exponent as usize))
    } else {
        String::new()
    };
    let symbol = currency.symbol.trim();
    if symbol.is_empty() {
        format!("#,##0{zeros}")
    } else {
        let escaped = symbol.replace('"', "\"\"");
        format!(
            "\"{escaped}\"#,##0{zeros};-\"{escaped}\"#,##0{zeros}"
        )
    }
}

fn axis_money_format(currency: &Currency) -> String {
    let zeros = if currency.exponent > 0 {
        format!(".{}", "0".repeat(currency.exponent as usize))
    } else {
        String::new()
    };
    let symbol = currency.symbol.trim();
    if symbol.is_empty() {
        format!("#,##0{zeros}")
    } else {
        format!("\"{}\"#,##0{zeros}", symbol.replace('"', "\"\""))
    }
}

fn to_units(amount: i64, exponent: u8) -> f64 {
    to_units_f64(amount as f64, exponent)
}

fn to_units_f64(amount: f64, exponent: u8) -> f64 {
    if exponent == 0 {
        amount
    } else {
        amount / 10_f64.powi(i32::from(exponent))
    }
}

fn item_currency_exponent(item: &Transaction, currencies: &HashMap<String, Currency>) -> u8 {
    currencies
        .get(&item.currency_code)
        .map(|currency| currency.exponent)
        .unwrap_or(0)
}

fn parse_color(raw: Option<&str>) -> Option<u32> {
    let value = raw?.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn display_date(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

fn type_label(value: &str) -> &'static str {
    match value {
        "income" => "Ingreso",
        "expense" => "Gasto",
        "transfer" => "Transferencia",
        "refund" => "Devolución",
        _ => "Movimiento",
    }
}

fn category_kind_label(item: &Usage) -> String {
    if item.category_id.is_none() {
        return "Sin categoría".to_string();
    }
    if item.income > 0 && (item.expenses > 0 || item.refunds > 0) {
        return "Mixta".to_string();
    }
    if item.income > 0 {
        return "Ingreso".to_string();
    }
    if item.expenses > 0 || item.refunds > 0 {
        return "Gasto".to_string();
    }
    if item.transfers > 0 {
        return "Transferencia".to_string();
    }
    match item.kind.as_str() {
        "income" => "Ingreso".to_string(),
        "expense" => "Gasto".to_string(),
        _ => "Sin categoría".to_string(),
    }
}

fn clip(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        value.chars().take(max_chars).collect::<String>() + "…"
    }
}

// ---------------------------------------------------------------------------
// Pruebas
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_money_with_symbol_and_decimals() {
        let currency = Currency {
            code: "CLP".to_string(),
            name: "Peso chileno".to_string(),
            symbol: "$".to_string(),
            exponent: 0,
        };
        assert_eq!(money_format(&currency), "\"$\"#,##0;-\"$\"#,##0");

        let dollars = Currency {
            code: "USD".to_string(),
            name: "Dólar".to_string(),
            symbol: "$".to_string(),
            exponent: 2,
        };
        assert_eq!(money_format(&dollars), "\"$\"#,##0.00;-\"$\"#,##0.00");
    }

    #[test]
    fn parses_only_valid_hex_colors() {
        assert_eq!(parse_color(Some("#1A2B3C")), Some(0x1A2B3C));
        assert_eq!(parse_color(Some("1a2b3c")), Some(0x1A2B3C));
        assert_eq!(parse_color(Some("#12345")), None);
        assert_eq!(parse_color(Some("rojo")), None);
        assert_eq!(parse_color(None), None);
    }

    #[test]
    fn groups_weekly_dates_from_monday() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();
        assert_eq!(bucket_key(&date.to_string(), Granularity::Week), "2026-10-19");
    }

    #[test]
    fn clips_long_text_without_breaking_utf8() {
        assert_eq!(clip("abcdef", 3), "abc…");
        assert_eq!(clip("ñandú", 10), "ñandú");
    }

    #[test]
    fn build_buckets_covers_each_granularity() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let daily = build_buckets(start, start, Granularity::Day);
        assert_eq!(daily.len(), 1);
        let weekly = build_buckets(start, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(), Granularity::Week);
        assert_eq!(weekly.len(), 3);
        let monthly = build_buckets(start, end, Granularity::Month);
        assert_eq!(monthly.len(), 3);
        let yearly = build_buckets(start, end, Granularity::Year);
        assert_eq!(yearly.len(), 1);
    }

    fn movement(index: usize, kind: &str, amount: i64) -> Transaction {
        Transaction {
            id: format!("t{index}"),
            transaction_type: kind.to_string(),
            description: format!("Movimiento {index}"),
            amount,
            currency_code: "CLP".to_string(),
            date: "2026-10-20".to_string(),
            category_id: None,
            category_name: None,
            account_id: None,
            account_name: None,
            transfer_account_id: None,
            transfer_account_name: None,
            contact_id: None,
            contact_name: None,
            payment_method: None,
            notes: None,
            tags: None,
            voided: false,
            voided_at: None,
            linked_type: None,
            linked_id: None,
            related_transaction_id: None,
            created_at: "2026-10-20 12:00:00".to_string(),
        }
    }

    #[test]
    fn monthly_buckets_count_every_movement_and_keep_refunds() {
        let transactions = vec![
            movement(1, "income", 100_000),
            movement(2, "expense", 30_000),
            movement(3, "refund", 12_000),
            movement(4, "transfer", 50_000),
        ];
        let filter = TransactionFilter {
            start_date: Some("2026-10-01".to_string()),
            end_date: Some("2026-10-31".to_string()),
            ..Default::default()
        };
        let data = collect_data(
            &transactions,
            &HashMap::new(),
            &filter,
            "CLP",
            "Octubre 2026",
        );

        assert_eq!(data.summary.transaction_count, 4);
        assert_eq!(data.summary.refunds, 12_000);
        assert_eq!(data.summary.net, 82_000);
        assert_eq!(data.buckets.iter().map(|bucket| bucket.count).sum::<i64>(), 4);
        assert_eq!(
            data.buckets
                .iter()
                .map(|bucket| bucket.refunds)
                .sum::<i64>(),
            12_000
        );
        assert_eq!(data.start, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert_eq!(data.end, NaiveDate::from_ymd_opt(2026, 10, 31).unwrap());
    }

}
