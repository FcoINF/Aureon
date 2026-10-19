//! Modelos de dominio.
//!
//! Todas las structs se serializan en `camelCase` para la interfaz.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Monedas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Currency {
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub exponent: u8,
}

// ---------------------------------------------------------------------------
// Cuentas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub currency_code: String,
    pub initial_balance: i64,
    pub initial_balance_date: String,
    pub archived: bool,
    pub description: Option<String>,
    pub created_at: String,
    pub balance: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub name: String,
    pub account_type: String,
    pub currency_code: String,
    pub initial_balance: i64,
    pub initial_balance_date: String,
    pub archived: Option<bool>,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Categorías
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub color: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub spent: Option<i64>,
    pub budget: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInput {
    pub name: String,
    pub kind: String,
    pub color: Option<String>,
}

// ---------------------------------------------------------------------------
// Contactos (personas y comercios)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    pub name: String,
    pub kind: String,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Movimientos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: String,
    pub transaction_type: String,
    pub description: String,
    pub amount: i64,
    pub currency_code: String,
    pub date: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub transfer_account_id: Option<String>,
    pub transfer_account_name: Option<String>,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub voided: bool,
    pub voided_at: Option<String>,
    pub linked_type: Option<String>,
    pub linked_id: Option<String>,
    pub related_transaction_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInput {
    pub transaction_type: String,
    pub description: String,
    pub amount: i64,
    pub currency_code: String,
    pub date: String,
    pub category_id: Option<String>,
    pub account_id: Option<String>,
    pub transfer_account_id: Option<String>,
    pub contact_id: Option<String>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<String>,
    pub related_transaction_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFilter {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub period: Option<String>,
    pub types: Option<Vec<String>>,
    pub category_ids: Option<Vec<String>>,
    pub account_ids: Option<Vec<String>>,
    pub contact_ids: Option<Vec<String>>,
    pub payment_method: Option<String>,
    pub min_amount: Option<i64>,
    pub max_amount: Option<i64>,
    pub search: Option<String>,
    pub include_voided: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort: Option<String>,
    pub direction: Option<String>,
    pub linked_type: Option<String>,
    pub linked_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Presupuestos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Budget {
    pub id: String,
    pub category_id: String,
    pub category_name: String,
    pub currency_code: String,
    pub amount: i64,
    pub start_date: String,
    pub end_date: Option<String>,
    pub spent: i64,
    pub period_spent: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetInput {
    pub category_id: String,
    pub currency_code: String,
    pub amount: i64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

// ---------------------------------------------------------------------------
// Compras y cuotas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Purchase {
    pub id: String,
    pub name: String,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub url: Option<String>,
    pub purchase_date: String,
    pub subtotal_amount: i64,
    pub interest_amount: i64,
    pub fees_amount: i64,
    pub total_amount: i64,
    pub currency_code: String,
    pub installments_count: i64,
    pub installment_amount: i64,
    pub first_due_date: Option<String>,
    pub frequency: String,
    pub frequency_days: Option<i64>,
    pub payment_method: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub paid_amount: i64,
    pub pending_amount: i64,
    pub installments_paid: i64,
    pub installments_pending: i64,
    pub installments_overdue: i64,
    pub next_due_date: Option<String>,
    pub last_due_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseInput {
    pub name: String,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub url: Option<String>,
    pub purchase_date: String,
    pub total_amount: i64,
    pub currency_code: String,
    pub installments_count: i64,
    pub installment_amount: Option<i64>,
    pub first_due_date: Option<String>,
    pub frequency: Option<String>,
    pub frequency_days: Option<i64>,
    pub payment_method: Option<String>,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub status: Option<String>,
    pub paid_immediately: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installment {
    pub id: String,
    pub purchase_id: String,
    pub purchase_name: String,
    pub number: i64,
    pub amount: i64,
    pub due_date: String,
    pub status: String,
    pub paid_at: Option<String>,
    pub paid_amount: i64,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallmentPaymentInput {
    pub amount: i64,
    pub paid_at: String,
    pub account_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Suscripciones
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub url: Option<String>,
    pub amount: i64,
    pub currency_code: String,
    pub frequency: String,
    pub frequency_days: Option<i64>,
    pub start_date: String,
    pub next_charge_date: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub payment_method: Option<String>,
    pub responsible_contact_id: Option<String>,
    pub responsible_name: Option<String>,
    pub status: String,
    pub cancelled_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub paid_total: i64,
    pub monthly_estimate: i64,
    pub yearly_estimate: i64,
    pub payment_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionInput {
    pub name: String,
    pub category_id: Option<String>,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub url: Option<String>,
    pub amount: i64,
    pub currency_code: String,
    pub frequency: String,
    pub frequency_days: Option<i64>,
    pub start_date: String,
    pub next_charge_date: Option<String>,
    pub account_id: Option<String>,
    pub payment_method: Option<String>,
    pub responsible_contact_id: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPayment {
    pub id: String,
    pub subscription_id: String,
    pub subscription_name: String,
    pub amount: i64,
    pub charged_at: String,
    pub next_charge_date: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub transaction_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionPaymentInput {
    pub amount: Option<i64>,
    pub charged_at: String,
    pub account_id: Option<String>,
    pub next_charge_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionShare {
    pub id: String,
    pub subscription_id: String,
    pub contact_id: String,
    pub contact_name: String,
    pub share_type: String,
    pub share_value: i64,
    pub receivable_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionShareInput {
    pub contact_id: String,
    pub share_type: String,
    pub share_value: i64,
    pub receivable_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Cuentas por cobrar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receivable {
    pub id: String,
    pub contact_id: String,
    pub contact_name: String,
    pub title: String,
    pub kind: String,
    pub total_amount: i64,
    pub received_amount: i64,
    pub pending_amount: i64,
    pub currency_code: String,
    pub origin_date: String,
    pub due_date: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub payment_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivableInput {
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub title: String,
    pub kind: Option<String>,
    pub total_amount: i64,
    pub currency_code: String,
    pub origin_date: String,
    pub due_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivablePayment {
    pub id: String,
    pub receivable_id: String,
    pub amount: i64,
    pub received_at: String,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub transaction_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivablePaymentInput {
    pub amount: i64,
    pub received_at: String,
    pub account_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Cuentas por pagar
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payable {
    pub id: String,
    pub contact_id: String,
    pub contact_name: String,
    pub title: String,
    pub total_amount: i64,
    pub paid_amount: i64,
    pub pending_amount: i64,
    pub currency_code: String,
    pub created_date: String,
    pub due_date: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub payment_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayableInput {
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub title: String,
    pub total_amount: i64,
    pub currency_code: String,
    pub created_date: String,
    pub due_date: Option<String>,
    pub account_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayablePayment {
    pub id: String,
    pub payable_id: String,
    pub amount: i64,
    pub paid_at: String,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub transaction_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayablePaymentInput {
    pub amount: i64,
    pub paid_at: String,
    pub account_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Depósitos pendientes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deposit {
    pub id: String,
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub title: String,
    pub amount: i64,
    pub currency_code: String,
    pub created_date: String,
    pub due_date: Option<String>,
    pub account_id: Option<String>,
    pub account_name: Option<String>,
    pub destination_institution: Option<String>,
    pub destination_details: Option<String>,
    pub payment_method: Option<String>,
    pub status: String,
    pub completed_at: Option<String>,
    pub transaction_id: Option<String>,
    pub receivable_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositInput {
    pub contact_id: Option<String>,
    pub contact_name: Option<String>,
    pub title: String,
    pub amount: i64,
    pub currency_code: String,
    pub created_date: String,
    pub due_date: Option<String>,
    pub account_id: Option<String>,
    pub destination_institution: Option<String>,
    pub destination_details: Option<String>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositCompleteInput {
    pub completed_at: Option<String>,
    pub account_id: Option<String>,
    pub transfer_account_id: Option<String>,
    pub register_transaction: Option<bool>,
    pub transaction_type: Option<String>,
    pub contact_id: Option<String>,
    pub category_id: Option<String>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub receivable_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Lista de deseos
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WishlistItem {
    pub id: String,
    pub name: Option<String>,
    pub url: Option<String>,
    pub store: Option<String>,
    pub estimated_price: Option<i64>,
    pub currency_code: String,
    pub added_date: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub priority: i64,
    pub savings_goal: Option<i64>,
    pub status: String,
    pub notes: Option<String>,
    pub purchase_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WishlistInput {
    pub name: Option<String>,
    pub url: Option<String>,
    pub store: Option<String>,
    pub estimated_price: Option<i64>,
    pub currency_code: Option<String>,
    pub added_date: Option<String>,
    pub category_id: Option<String>,
    pub priority: Option<i64>,
    pub savings_goal: Option<i64>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Reportes y panel principal
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyAmount {
    pub currency_code: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryByCurrency {
    pub currency_code: String,
    pub balance: i64,
    pub income: i64,
    pub expenses: i64,
    pub net: i64,
    pub receivable_pending: i64,
    pub payable_pending: i64,
    pub installments_pending: i64,
    pub installments_pending_amount: i64,
    pub subscriptions_active: i64,
    pub subscriptions_monthly: i64,
    pub deposits_pending: i64,
    pub deposits_pending_amount: i64,
    pub overdue_amount: i64,
    pub overdue_count: i64,
    pub upcoming_amount: i64,
    pub upcoming_count: i64,
    pub refunds: i64,
    pub transfers: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPoint {
    pub label: String,
    pub value: i64,
    pub secondary: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdown {
    pub id: Option<String>,
    pub label: String,
    pub amount: i64,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingCommitment {
    pub id: String,
    pub source: String,
    pub title: String,
    pub amount: i64,
    pub currency_code: String,
    pub due_date: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummary {
    pub currency_code: String,
    pub income: i64,
    pub expenses: i64,
    pub net: i64,
    pub refunds: i64,
    pub transfers: i64,
    pub purchases_total: i64,
    pub installments_paid: i64,
    pub subscriptions_paid: i64,
    pub money_received: i64,
    pub money_paid: i64,
    pub pending_commitments: i64,
    pub overdue_commitments: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: String,
    pub source: String,
    pub title: String,
    pub amount: i64,
    pub currency_code: String,
    pub date: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub file_name: String,
    pub stored_name: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub database_path: String,
    pub database_exists: bool,
    pub schema_version: i64,
    pub has_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub path: String,
    pub schema_version: i64,
    pub created_at: String,
}
