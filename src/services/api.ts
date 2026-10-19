/**
 * Acceso a la aplicación nativa.
 *
 * Todas las consultas SQL y reglas financieras viven en Rust. La interfaz
 * sólo invoca comandos tipados.
 */
import { invoke } from '@tauri-apps/api/core';

import type {
  Account,
  AccountInput,
  AppInfo,
  AppSettings,
  Attachment,
  BackupResult,
  Budget,
  BudgetInput,
  CalendarEvent,
  Category,
  CategoryBreakdown,
  CategoryInput,
  Contact,
  ContactInput,
  Currency,
  Deposit,
  DepositCompleteInput,
  DepositInput,
  HistorySummary,
  Installment,
  InstallmentPaymentInput,
  Payable,
  PayableInput,
  PayablePayment,
  PayablePaymentInput,
  Purchase,
  PurchaseInput,
  Receivable,
  ReceivableInput,
  ReceivablePayment,
  ReceivablePaymentInput,
  SeriesPoint,
  Subscription,
  SubscriptionInput,
  SubscriptionPayment,
  SubscriptionPaymentInput,
  SubscriptionShare,
  SubscriptionShareInput,
  SummaryByCurrency,
  Transaction,
  TransactionFilter,
  TransactionInput,
  UpcomingCommitment,
  WishlistInput,
  WishlistItem,
} from '../types';

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new Error(readError(error));
  }
}

/** Convierte el error nativo en un mensaje comprensible. */
export function readError(error: unknown): string {
  const raw =
    typeof error === 'string'
      ? error
      : error && typeof error === 'object' && 'message' in error
        ? String((error as { message: unknown }).message)
        : 'Ocurrió un problema inesperado.';

  // Los errores de deserialización de Tauri son técnicos: se traducen.
  if (/invalid type|invalid value|expected/i.test(raw)) {
    return 'Alguno de los valores enviados no tiene un formato válido. Revisa los campos marcados.';
  }
  if (/missing required/i.test(raw)) {
    return 'Falta completar algún campo obligatorio.';
  }
  return raw;
}

export const api = {
  // Cuentas -----------------------------------------------------------
  listCurrencies: () => call<Currency[]>('list_currencies'),
  listAccounts: (includeArchived = false) =>
    call<Account[]>('list_accounts', { includeArchived }),
  createAccount: (input: AccountInput) => call<Account>('create_account', { input }),
  updateAccount: (id: string, input: AccountInput) =>
    call<Account>('update_account', { id, input }),
  setAccountArchived: (id: string, archived: boolean) =>
    call<Account>('set_account_archived', { id, archived }),
  deleteAccount: (id: string) => call<void>('delete_account', { id }),

  // Categorías --------------------------------------------------------
  listCategories: (kind?: string, includeArchived = false, period = 'month', currency?: string) =>
    call<Category[]>('list_categories', { kind, includeArchived, period, currency }),
  createCategory: (input: CategoryInput) => call<Category>('create_category', { input }),
  updateCategory: (id: string, input: CategoryInput) =>
    call<Category>('update_category', { id, input }),
  setCategoryArchived: (id: string, archived: boolean) =>
    call<Category>('set_category_archived', { id, archived }),

  // Contactos ---------------------------------------------------------
  listContacts: (kind?: string, search?: string) =>
    call<Contact[]>('list_contacts', { kind, search }),
  createContact: (input: ContactInput) => call<Contact>('create_contact', { input }),
  updateContact: (id: string, input: ContactInput) =>
    call<Contact>('update_contact', { id, input }),
  deleteContact: (id: string) => call<void>('delete_contact', { id }),

  // Movimientos --------------------------------------------------------
  listTransactions: (filter?: TransactionFilter) =>
    call<Transaction[]>('list_transactions', { filter }),
  countTransactions: (filter?: TransactionFilter) =>
    call<number>('count_transactions', { filter }),
  createTransaction: (input: TransactionInput) =>
    call<Transaction>('create_transaction', { input }),
  updateTransaction: (id: string, input: TransactionInput) =>
    call<Transaction>('update_transaction', { id, input }),
  voidTransaction: (id: string) => call<Transaction>('void_transaction', { id }),
  unvoidTransaction: (id: string) => call<Transaction>('unvoid_transaction', { id }),
  deleteTransaction: (id: string) => call<void>('delete_transaction', { id }),

  // Compras y cuotas ---------------------------------------------------
  listPurchases: (status?: string) => call<Purchase[]>('list_purchases', { status }),
  getPurchase: (id: string) => call<Purchase>('get_purchase', { id }),
  listInstallments: (purchaseId: string) =>
    call<Installment[]>('list_installments', { purchaseId }),
  createPurchase: (input: PurchaseInput) => call<Purchase>('create_purchase', { input }),
  payInstallment: (installmentId: string, input: InstallmentPaymentInput) =>
    call<Installment>('pay_installment', { installmentId, input }),
  updateInstallment: (installmentId: string, amount: number, dueDate: string) =>
    call<Installment>('update_installment', { installmentId, amount, dueDate }),
  cancelPurchase: (id: string) => call<Purchase>('cancel_purchase', { id }),
  refundPurchase: (id: string, amount: number, date: string, accountId?: string | null) =>
    call<Purchase>('refund_purchase', { id, amount, date, accountId }),

  // Presupuestos -------------------------------------------------------
  listBudgets: () => call<Budget[]>('list_budgets'),
  saveBudget: (input: BudgetInput) => call<Budget>('save_budget', { input }),
  updateBudget: (id: string, input: BudgetInput) =>
    call<Budget>('update_budget', { id, input }),
  deleteBudget: (id: string) => call<void>('delete_budget', { id }),

  // Suscripciones ------------------------------------------------------
  listSubscriptions: (status?: string) =>
    call<Subscription[]>('list_subscriptions', { status }),
  getSubscription: (id: string) => call<Subscription>('get_subscription', { id }),
  listSubscriptionPayments: (subscriptionId: string) =>
    call<SubscriptionPayment[]>('list_subscription_payments', { subscriptionId }),
  createSubscription: (input: SubscriptionInput) =>
    call<Subscription>('create_subscription', { input }),
  updateSubscription: (id: string, input: SubscriptionInput) =>
    call<Subscription>('update_subscription', { id, input }),
  chargeSubscription: (id: string, input: SubscriptionPaymentInput) =>
    call<SubscriptionPayment>('charge_subscription', { id, input }),
  setSubscriptionStatus: (id: string, status: string) =>
    call<Subscription>('set_subscription_status', { id, status }),
  deleteSubscription: (id: string) => call<void>('delete_subscription', { id }),
  listSubscriptionShares: (subscriptionId: string) =>
    call<SubscriptionShare[]>('list_subscription_shares', { subscriptionId }),
  addSubscriptionShare: (subscriptionId: string, input: SubscriptionShareInput) =>
    call<SubscriptionShare>('add_subscription_share', { subscriptionId, input }),
  removeSubscriptionShare: (id: string) =>
    call<void>('remove_subscription_share', { id }),

  // Cuentas por cobrar -------------------------------------------------
  listReceivables: (status?: string) => call<Receivable[]>('list_receivables', { status }),
  getReceivable: (id: string) => call<Receivable>('get_receivable', { id }),
  listReceivablePayments: (receivableId: string) =>
    call<ReceivablePayment[]>('list_receivable_payments', { receivableId }),
  createReceivable: (input: ReceivableInput) =>
    call<Receivable>('create_receivable', { input }),
  updateReceivable: (id: string, input: ReceivableInput) =>
    call<Receivable>('update_receivable', { id, input }),
  receiveMoney: (id: string, input: ReceivablePaymentInput) =>
    call<Receivable>('receive_money', { id, input }),
  setReceivableStatus: (id: string, status: string) =>
    call<Receivable>('set_receivable_status', { id, status }),
  deleteReceivable: (id: string) => call<void>('delete_receivable', { id }),

  // Cuentas por pagar --------------------------------------------------
  listPayables: (status?: string) => call<Payable[]>('list_payables', { status }),
  getPayable: (id: string) => call<Payable>('get_payable', { id }),
  listPayablePayments: (payableId: string) =>
    call<PayablePayment[]>('list_payable_payments', { payableId }),
  createPayable: (input: PayableInput) => call<Payable>('create_payable', { input }),
  updatePayable: (id: string, input: PayableInput) =>
    call<Payable>('update_payable', { id, input }),
  payObligation: (id: string, input: PayablePaymentInput) =>
    call<Payable>('pay_obligation', { id, input }),
  setPayableStatus: (id: string, status: string) =>
    call<Payable>('set_payable_status', { id, status }),
  deletePayable: (id: string) => call<void>('delete_payable', { id }),

  // Depósitos pendientes ------------------------------------------------
  listDeposits: (status?: string) => call<Deposit[]>('list_deposits', { status }),
  getDeposit: (id: string) => call<Deposit>('get_deposit', { id }),
  createDeposit: (input: DepositInput) => call<Deposit>('create_deposit', { input }),
  updateDeposit: (id: string, input: DepositInput) =>
    call<Deposit>('update_deposit', { id, input }),
  completeDeposit: (id: string, input: DepositCompleteInput) =>
    call<Deposit>('complete_deposit', { id, input }),
  setDepositStatus: (id: string, status: string) =>
    call<Deposit>('set_deposit_status', { id, status }),
  deleteDeposit: (id: string) => call<void>('delete_deposit', { id }),

  // Lista de deseos -----------------------------------------------------
  listWishlist: (status?: string) => call<WishlistItem[]>('list_wishlist', { status }),
  createWishlistItem: (input: WishlistInput) =>
    call<WishlistItem>('create_wishlist_item', { input }),
  updateWishlistItem: (id: string, input: WishlistInput) =>
    call<WishlistItem>('update_wishlist_item', { id, input }),
  setWishlistStatus: (id: string, status: string) =>
    call<WishlistItem>('set_wishlist_status', { id, status }),
  convertWishlistItem: (id: string, input: PurchaseInput) =>
    call<Purchase>('convert_wishlist_item', { id, input }),
  deleteWishlistItem: (id: string) => call<void>('delete_wishlist_item', { id }),

  // Reportes ------------------------------------------------------------
  dashboardSummary: (period: string, currency: string) =>
    call<SummaryByCurrency | null>('dashboard_summary', { period, currency }),
  summaryByCurrency: (period: string) =>
    call<SummaryByCurrency[]>('summary_by_currency', { period }),
  incomeExpenseSeries: (period: string, currency: string, granularity: string) =>
    call<SeriesPoint[]>('income_expense_series', { period, currency, granularity }),
  expensesByCategory: (period: string, currency: string) =>
    call<CategoryBreakdown[]>('expenses_by_category', { period, currency }),
  expensesByContact: (period: string, currency: string) =>
    call<CategoryBreakdown[]>('expenses_by_contact', { period, currency }),
  balanceEvolution: (period: string, currency: string) =>
    call<SeriesPoint[]>('balance_evolution', { period, currency }),
  historySummary: (period: string) => call<HistorySummary[]>('history_summary', { period }),
  recentTransactions: (limit = 8) => call<Transaction[]>('recent_transactions', { limit }),
  upcomingCommitments: (limit = 8) =>
    call<UpcomingCommitment[]>('upcoming_commitments', { limit }),
  calendarEvents: (start: string, end: string) =>
    call<CalendarEvent[]>('calendar_events', { start, end }),

  // Sistema -------------------------------------------------------------
  getSettings: () => call<AppSettings>('get_settings'),
  saveSettings: (settings: AppSettings) => call<AppSettings>('save_settings', { settings }),
  appInfo: () => call<AppInfo>('app_info'),
  refreshOverdueStatus: () => call<void>('refresh_overdue_status'),
  exportDataset: (dataset: string, filter?: TransactionFilter) =>
    call<string>('export_dataset', { dataset, filter }),
  writeTextFile: (path: string, contents: string) =>
    call<string>('write_text_file', { path, contents }),
  writeBase64File: (path: string, contents: string) =>
    call<string>('write_base64_file', { path, contents }),
  createBackup: (path: string) => call<BackupResult>('create_backup', { path }),
  validateBackupFile: (path: string) => call<number>('validate_backup_file', { path }),
  restoreBackup: (path: string) => call<number>('restore_backup', { path }),
  importAttachment: (sourcePath: string) =>
    call<Attachment>('import_attachment', { sourcePath }),
  openAttachment: (storedName: string) => call<void>('open_attachment', { storedName }),
  openExternalUrl: (url: string) => call<void>('open_external_url', { url }),
  defaultBackupPath: () => call<string>('default_backup_path'),
};
