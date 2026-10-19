/**
 * Tipos de dominio de Aureon.
 * Reflejan exactamente los modelos expuestos por los comandos nativos.
 */

export interface Currency {
  code: string;
  name: string;
  symbol: string;
  exponent: number;
}

export type AccountType =
  | 'cash'
  | 'checking'
  | 'savings'
  | 'rut'
  | 'credit_card'
  | 'other';

export interface Account {
  id: string;
  name: string;
  accountType: AccountType;
  currencyCode: string;
  initialBalance: number;
  initialBalanceDate: string;
  archived: boolean;
  description: string | null;
  createdAt: string;
  balance: number;
}

export interface AccountInput {
  name: string;
  accountType: AccountType;
  currencyCode: string;
  initialBalance: number;
  initialBalanceDate: string;
  archived?: boolean;
  description?: string | null;
}

export type CategoryKind = 'income' | 'expense';

export interface Category {
  id: string;
  name: string;
  kind: CategoryKind;
  color: string | null;
  archived: boolean;
  createdAt: string;
  spent: number | null;
  budget: number | null;
}

export interface CategoryInput {
  name: string;
  kind: CategoryKind;
  color?: string | null;
}

export type ContactKind = 'person' | 'merchant';

export interface Contact {
  id: string;
  name: string;
  kind: ContactKind;
  notes: string | null;
  createdAt: string;
}

export interface ContactInput {
  name: string;
  kind: ContactKind;
  notes?: string | null;
}

export type TransactionType = 'income' | 'expense' | 'transfer' | 'refund';

export interface Transaction {
  id: string;
  transactionType: TransactionType;
  description: string;
  amount: number;
  currencyCode: string;
  date: string;
  categoryId: string | null;
  categoryName: string | null;
  accountId: string | null;
  accountName: string | null;
  transferAccountId: string | null;
  transferAccountName: string | null;
  contactId: string | null;
  contactName: string | null;
  paymentMethod: string | null;
  notes: string | null;
  tags: string | null;
  voided: boolean;
  voidedAt: string | null;
  linkedType: string | null;
  linkedId: string | null;
  relatedTransactionId: string | null;
  createdAt: string;
}

export interface TransactionInput {
  transactionType: TransactionType;
  description: string;
  amount: number;
  currencyCode: string;
  date: string;
  categoryId?: string | null;
  accountId?: string | null;
  transferAccountId?: string | null;
  contactId?: string | null;
  paymentMethod?: string | null;
  notes?: string | null;
  tags?: string | null;
  relatedTransactionId?: string | null;
}

export interface TransactionFilter {
  startDate?: string | null;
  endDate?: string | null;
  period?: string | null;
  types?: string[] | null;
  categoryIds?: string[] | null;
  accountIds?: string[] | null;
  contactIds?: string[] | null;
  paymentMethod?: string | null;
  minAmount?: number | null;
  maxAmount?: number | null;
  search?: string | null;
  includeVoided?: boolean | null;
  limit?: number | null;
  offset?: number | null;
  sort?: string | null;
  direction?: string | null;
  linkedType?: string | null;
  linkedId?: string | null;
}

export interface Budget {
  id: string;
  categoryId: string;
  categoryName: string;
  currencyCode: string;
  amount: number;
  startDate: string;
  endDate: string | null;
  spent: number;
  periodSpent: number;
}

export interface BudgetInput {
  categoryId: string;
  currencyCode: string;
  amount: number;
  startDate?: string | null;
  endDate?: string | null;
}

export type Frequency =
  | 'weekly'
  | 'biweekly'
  | 'monthly'
  | 'bimonthly'
  | 'quarterly'
  | 'semiannual'
  | 'yearly'
  | 'custom';

export type PurchaseStatus = 'active' | 'completed' | 'cancelled' | 'refunded';
export type InstallmentStatus = 'pending' | 'paid' | 'overdue' | 'cancelled';

export interface Purchase {
  id: string;
  name: string;
  contactId: string | null;
  contactName: string | null;
  url: string | null;
  purchaseDate: string;
  subtotalAmount: number;
  interestAmount: number;
  feesAmount: number;
  totalAmount: number;
  currencyCode: string;
  installmentsCount: number;
  installmentAmount: number;
  firstDueDate: string | null;
  frequency: Frequency;
  frequencyDays: number | null;
  paymentMethod: string | null;
  accountId: string | null;
  accountName: string | null;
  status: PurchaseStatus;
  notes: string | null;
  createdAt: string;
  categoryId: string | null;
  categoryName: string | null;
  paidAmount: number;
  pendingAmount: number;
  installmentsPaid: number;
  installmentsPending: number;
  installmentsOverdue: number;
  nextDueDate: string | null;
  lastDueDate: string | null;
}

export interface PurchaseInput {
  name: string;
  contactId?: string | null;
  contactName?: string | null;
  url?: string | null;
  purchaseDate: string;
  totalAmount: number;
  currencyCode: string;
  installmentsCount: number;
  installmentAmount?: number | null;
  firstDueDate?: string | null;
  frequency?: Frequency | null;
  frequencyDays?: number | null;
  paymentMethod?: string | null;
  accountId?: string | null;
  categoryId?: string | null;
  status?: PurchaseStatus | null;
  paidImmediately?: boolean | null;
  notes?: string | null;
}

export interface Installment {
  id: string;
  purchaseId: string;
  purchaseName: string;
  number: number;
  amount: number;
  dueDate: string;
  status: InstallmentStatus;
  paidAt: string | null;
  paidAmount: number;
  accountId: string | null;
  accountName: string | null;
  transactionId: string | null;
}

export interface InstallmentPaymentInput {
  amount: number;
  paidAt: string;
  accountId?: string | null;
}

export type SubscriptionStatus = 'active' | 'paused' | 'cancelled' | 'expired';

export interface Subscription {
  id: string;
  name: string;
  categoryId: string | null;
  categoryName: string | null;
  contactId: string | null;
  contactName: string | null;
  url: string | null;
  amount: number;
  currencyCode: string;
  frequency: Frequency;
  frequencyDays: number | null;
  startDate: string;
  nextChargeDate: string | null;
  accountId: string | null;
  accountName: string | null;
  paymentMethod: string | null;
  responsibleContactId: string | null;
  responsibleName: string | null;
  status: SubscriptionStatus;
  cancelledAt: string | null;
  notes: string | null;
  createdAt: string;
  paidTotal: number;
  monthlyEstimate: number;
  yearlyEstimate: number;
  paymentCount: number;
}

export interface SubscriptionInput {
  name: string;
  categoryId?: string | null;
  contactId?: string | null;
  contactName?: string | null;
  url?: string | null;
  amount: number;
  currencyCode: string;
  frequency: Frequency;
  frequencyDays?: number | null;
  startDate: string;
  nextChargeDate?: string | null;
  accountId?: string | null;
  paymentMethod?: string | null;
  responsibleContactId?: string | null;
  status?: SubscriptionStatus | null;
  notes?: string | null;
}

export interface SubscriptionPayment {
  id: string;
  subscriptionId: string;
  subscriptionName: string;
  amount: number;
  chargedAt: string;
  nextChargeDate: string | null;
  accountId: string | null;
  accountName: string | null;
  transactionId: string | null;
  createdAt: string;
}

export interface SubscriptionPaymentInput {
  amount?: number | null;
  chargedAt: string;
  accountId?: string | null;
  nextChargeDate?: string | null;
}

export interface SubscriptionShare {
  id: string;
  subscriptionId: string;
  contactId: string;
  contactName: string;
  shareType: 'percent' | 'amount';
  shareValue: number;
  receivableId: string | null;
}

export interface SubscriptionShareInput {
  contactId: string;
  shareType: 'percent' | 'amount';
  shareValue: number;
  receivableId?: string | null;
}

export type ReceivableKind = 'debt' | 'advance' | 'refund';
export type ObligationStatus = 'pending' | 'partial' | 'paid' | 'cancelled' | 'overdue';

export interface Receivable {
  id: string;
  contactId: string;
  contactName: string;
  title: string;
  kind: ReceivableKind;
  totalAmount: number;
  receivedAmount: number;
  pendingAmount: number;
  currencyCode: string;
  originDate: string;
  dueDate: string | null;
  status: ObligationStatus;
  notes: string | null;
  createdAt: string;
  paymentCount: number;
}

export interface ReceivableInput {
  contactId?: string | null;
  contactName?: string | null;
  title: string;
  kind?: ReceivableKind | null;
  totalAmount: number;
  currencyCode: string;
  originDate: string;
  dueDate?: string | null;
  notes?: string | null;
}

export interface ReceivablePayment {
  id: string;
  receivableId: string;
  amount: number;
  receivedAt: string;
  accountId: string | null;
  accountName: string | null;
  transactionId: string | null;
  createdAt: string;
}

export interface ReceivablePaymentInput {
  amount: number;
  receivedAt: string;
  accountId?: string | null;
}

export interface Payable {
  id: string;
  contactId: string;
  contactName: string;
  title: string;
  totalAmount: number;
  paidAmount: number;
  pendingAmount: number;
  currencyCode: string;
  createdDate: string;
  dueDate: string | null;
  accountId: string | null;
  accountName: string | null;
  status: ObligationStatus;
  notes: string | null;
  createdAt: string;
  paymentCount: number;
}

export interface PayableInput {
  contactId?: string | null;
  contactName?: string | null;
  title: string;
  totalAmount: number;
  currencyCode: string;
  createdDate: string;
  dueDate?: string | null;
  accountId?: string | null;
  notes?: string | null;
}

export interface PayablePayment {
  id: string;
  payableId: string;
  amount: number;
  paidAt: string;
  accountId: string | null;
  accountName: string | null;
  transactionId: string | null;
  createdAt: string;
}

export interface PayablePaymentInput {
  amount: number;
  paidAt: string;
  accountId?: string | null;
}

export type DepositStatus = 'pending' | 'scheduled' | 'completed' | 'overdue' | 'cancelled';

export interface Deposit {
  id: string;
  contactId: string | null;
  contactName: string | null;
  title: string;
  amount: number;
  currencyCode: string;
  createdDate: string;
  dueDate: string | null;
  accountId: string | null;
  accountName: string | null;
  destinationInstitution: string | null;
  destinationDetails: string | null;
  paymentMethod: string | null;
  status: DepositStatus;
  completedAt: string | null;
  transactionId: string | null;
  receivableId: string | null;
  notes: string | null;
  createdAt: string;
}

export interface DepositInput {
  contactId?: string | null;
  contactName?: string | null;
  title: string;
  amount: number;
  currencyCode: string;
  createdDate: string;
  dueDate?: string | null;
  accountId?: string | null;
  destinationInstitution?: string | null;
  destinationDetails?: string | null;
  paymentMethod?: string | null;
  notes?: string | null;
}

export interface DepositCompleteInput {
  completedAt?: string | null;
  accountId?: string | null;
  transferAccountId?: string | null;
  registerTransaction?: boolean | null;
  transactionType?: 'expense' | 'transfer' | null;
  contactId?: string | null;
  categoryId?: string | null;
  paymentMethod?: string | null;
  notes?: string | null;
  receivableId?: string | null;
}

export type WishlistStatus = 'wanted' | 'saving' | 'purchased' | 'discarded';

export interface WishlistItem {
  id: string;
  name: string | null;
  url: string | null;
  store: string | null;
  estimatedPrice: number | null;
  currencyCode: string;
  addedDate: string;
  categoryId: string | null;
  categoryName: string | null;
  priority: number;
  savingsGoal: number | null;
  status: WishlistStatus;
  notes: string | null;
  purchaseId: string | null;
  createdAt: string;
}

export interface WishlistInput {
  name?: string | null;
  url?: string | null;
  store?: string | null;
  estimatedPrice?: number | null;
  currencyCode?: string | null;
  addedDate?: string | null;
  categoryId?: string | null;
  priority?: number | null;
  savingsGoal?: number | null;
  status?: WishlistStatus | null;
  notes?: string | null;
}

export interface SummaryByCurrency {
  currencyCode: string;
  balance: number;
  income: number;
  expenses: number;
  net: number;
  receivablePending: number;
  payablePending: number;
  installmentsPending: number;
  installmentsPendingAmount: number;
  subscriptionsActive: number;
  subscriptionsMonthly: number;
  depositsPending: number;
  depositsPendingAmount: number;
  overdueAmount: number;
  overdueCount: number;
  upcomingAmount: number;
  upcomingCount: number;
  refunds: number;
  transfers: number;
}

export interface SeriesPoint {
  label: string;
  value: number;
  secondary: number | null;
}

export interface CategoryBreakdown {
  id: string | null;
  label: string;
  amount: number;
  color: string | null;
}

export interface UpcomingCommitment {
  id: string;
  source: string;
  title: string;
  amount: number;
  currencyCode: string;
  dueDate: string;
  status: string;
  detail: string | null;
}

export interface HistorySummary {
  currencyCode: string;
  income: number;
  expenses: number;
  net: number;
  refunds: number;
  transfers: number;
  purchasesTotal: number;
  installmentsPaid: number;
  subscriptionsPaid: number;
  moneyReceived: number;
  moneyPaid: number;
  pendingCommitments: number;
  overdueCommitments: number;
  transactionCount: number;
}

export interface CalendarEvent {
  id: string;
  source: string;
  title: string;
  amount: number;
  currencyCode: string;
  date: string;
  status: string;
  detail: string | null;
}

export interface Attachment {
  id: string;
  fileName: string;
  storedName: string;
  mimeType: string | null;
  sizeBytes: number;
  createdAt: string;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  databasePath: string;
  databaseExists: boolean;
  schemaVersion: number;
  hasData: boolean;
}

export interface BackupResult {
  path: string;
  schemaVersion: number;
  createdAt: string;
}

export interface AppSettings {
  defaultCurrency: string;
  dateFormat: string;
  theme: 'light' | 'dark';
  exportSeparator: string;
  backupFolder: string | null;
  weekStartsOnMonday: boolean;
}
