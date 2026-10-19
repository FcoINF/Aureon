/**
 * Formato de dinero y fechas (convención chilena).
 *
 * Los importes viajan entre la interfaz y Rust como enteros en la unidad
 * mínima de la moneda: pesos enteros para CLP, centavos para USD/EUR.
 */

export const CURRENCY_EXPONENTS: Record<string, number> = {
  CLP: 0,
  USD: 2,
  EUR: 2,
};

export const CURRENCY_SYMBOLS: Record<string, string> = {
  CLP: '$',
  USD: 'US$',
  EUR: '€',
};

export function exponentOf(currency: string): number {
  return CURRENCY_EXPONENTS[currency] ?? 0;
}

export function symbolOf(currency: string): string {
  return CURRENCY_SYMBOLS[currency] ?? '$';
}

/** Convierte lo que el usuario escribe a unidades mínimas. */
export function parseAmountToMinor(raw: string, exponent: number): number | null {
  const cleaned = raw.replace(/[^\d.,-]/g, '');
  if (!cleaned) return null;

  const negative = cleaned.startsWith('-');
  const body = cleaned.replace('-', '');
  if (!body) return null;

  let head = body;
  let fraction: string | null = null;

  const lastComma = body.lastIndexOf(',');
  const lastDot = body.lastIndexOf('.');

  if (lastComma >= 0 && lastDot >= 0) {
    if (lastComma > lastDot) {
      head = body.slice(0, lastComma).replace(/\./g, '');
      fraction = body.slice(lastComma + 1);
    } else {
      head = body.slice(0, lastDot).replace(/,/g, '');
      fraction = body.slice(lastDot + 1);
    }
  } else if (lastComma >= 0) {
    const tail = body.slice(lastComma + 1);
    if (exponent === 0) {
      if (tail.length === 2 && /[1-9]/.test(tail)) {
        head = body.slice(0, lastComma);
        fraction = tail;
      } else {
        head = body.slice(0, lastComma);
      }
    } else {
      head = body.slice(0, lastComma);
      fraction = tail;
    }
  } else if (lastDot >= 0) {
    const tail = body.slice(lastDot + 1);
    const isDecimal =
      exponent === 0
        ? tail.length === 2 && /[1-9]/.test(tail)
        : tail.length === exponent;
    if (isDecimal) {
      head = body.slice(0, lastDot).replace(/,/g, '');
      fraction = tail;
    } else {
      // El punto separa miles: se elimina por completo.
      head = body.replace(/\./g, '');
    }
  }

  if (!/^\d*$/.test(head)) return null;
  if (fraction !== null && !/^\d*$/.test(fraction)) return null;

  const base = head === '' ? 0 : Number.parseInt(head, 10);
  if (!Number.isFinite(base)) return null;

  let minor = base * 10 ** exponent;
  if (fraction !== null) {
    const digits = fraction.slice(0, exponent);
    minor += digits === '' ? 0 : Number.parseInt(digits, 10);
    if (fraction.length > exponent) {
      const next = Number.parseInt(fraction[exponent], 10);
      if (!Number.isNaN(next) && next >= 5) minor += 1;
    }
  }

  const result = negative ? -minor : minor;
  return Number.isSafeInteger(result) ? result : null;
}

/** Formatea unidades mínimas en convención chilena: 1.234,56 */
export function formatAmount(minor: number, exponent = 0): string {
  const negative = minor < 0;
  const abs = Math.abs(minor);
  const factor = 10 ** exponent;
  const integer = Math.floor(abs / factor);
  const fraction = abs % factor;

  const digits = String(integer);
  let grouped = '';
  for (let i = 0; i < digits.length; i += 1) {
    if (i > 0 && (digits.length - i) % 3 === 0) grouped += '.';
    grouped += digits[i];
  }

  let out = negative ? '-' : '';
  out += grouped;
  if (exponent > 0) {
    out += ',';
    out += String(fraction).padStart(exponent, '0');
  }
  return out;
}

/** Formatea con el símbolo de la moneda: $1.234 */
export function formatCurrency(minor: number, currency = 'CLP', withSymbol = true): string {
  const exponent = exponentOf(currency);
  const amount = formatAmount(minor, exponent);
  return withSymbol ? `${symbolOf(currency)}${amount}` : amount;
}

/** Convierte a texto editable para los formularios. */
export function amountToInput(minor: number, currency = 'CLP'): string {
  const exponent = exponentOf(currency);
  if (exponent === 0) return String(minor);
  return formatAmount(minor, exponent).replace('.', '').replace(',', '.');
}

// ---------------------------------------------------------------------------
// Fechas
// ---------------------------------------------------------------------------

/** Fecha de hoy en formato ISO (AAAA-MM-DD). */
export function todayIso(): string {
  const now = new Date();
  return isoFromDate(now);
}

export function isoFromDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/** Muestra las fechas como DD/MM/AAAA. */
export function formatDate(iso: string | null | undefined): string {
  if (!iso) return '—';
  const base = iso.split('T')[0];
  if (!/^\d{4}-\d{2}-\d{2}$/.test(base)) return iso;
  return `${base.slice(8, 10)}/${base.slice(5, 7)}/${base.slice(0, 4)}`;
}

export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return '—';
  const [date, time] = iso.split('T');
  return `${formatDate(date)}${time ? ` ${time.slice(0, 5)}` : ''}`;
}

/** Días que faltan para una fecha (negativo si ya pasó). */
export function daysUntil(iso: string): number {
  const target = new Date(`${iso.split('T')[0]}T00:00:00`);
  const today = new Date(`${todayIso()}T00:00:00`);
  return Math.round((target.getTime() - today.getTime()) / 86_400_000);
}

export function dueLabel(iso: string | null | undefined): string {
  if (!iso) return 'Sin fecha';
  const days = daysUntil(iso);
  if (days === 0) return 'Vence hoy';
  if (days === 1) return 'Vence mañana';
  if (days === -1) return 'Venció ayer';
  if (days < 0) return `Vencida hace ${Math.abs(days)} días`;
  if (days <= 30) return `Vence en ${days} días`;
  return `Vence el ${formatDate(iso)}`;
}

export const MONTH_NAMES = [
  'Enero',
  'Febrero',
  'Marzo',
  'Abril',
  'Mayo',
  'Junio',
  'Julio',
  'Agosto',
  'Septiembre',
  'Octubre',
  'Noviembre',
  'Diciembre',
];

export const WEEKDAY_NAMES = [
  'Lunes',
  'Martes',
  'Miércoles',
  'Jueves',
  'Viernes',
  'Sábado',
  'Domingo',
];

/** Etiquetas de los períodos disponibles en el historial. */
export const PERIOD_OPTIONS: { value: string; label: string }[] = [
  { value: 'today', label: 'Hoy' },
  { value: 'yesterday', label: 'Ayer' },
  { value: 'week', label: 'Semana actual' },
  { value: 'last_week', label: 'Semana anterior' },
  { value: 'month', label: 'Mes actual' },
  { value: 'last_month', label: 'Mes anterior' },
  { value: 'year', label: 'Año actual' },
  { value: 'last_year', label: 'Año anterior' },
];

export const FREQUENCY_LABELS: Record<string, string> = {
  weekly: 'Semanal',
  biweekly: 'Quincenal',
  monthly: 'Mensual',
  bimonthly: 'Cada dos meses',
  quarterly: 'Trimestral',
  semiannual: 'Semestral',
  yearly: 'Anual',
  custom: 'Personalizada',
};

export const PAYMENT_METHODS = [
  'Débito',
  'Crédito',
  'Transferencia',
  'Efectivo',
  'Tarjeta de crédito',
  'Vale',
  'Otro',
];

export const ACCOUNT_TYPE_LABELS: Record<string, string> = {
  cash: 'Efectivo',
  checking: 'Cuenta corriente',
  savings: 'Cuenta de ahorro',
  rut: 'Cuenta RUT',
  credit_card: 'Tarjeta de crédito',
  other: 'Otro',
};

export const TRANSACTION_TYPE_LABELS: Record<string, string> = {
  income: 'Ingreso',
  expense: 'Gasto',
  transfer: 'Transferencia',
  refund: 'Devolución',
};

export const STATUS_LABELS: Record<string, string> = {
  pending: 'Pendiente',
  partial: 'Parcial',
  paid: 'Pagado',
  cancelled: 'Cancelado',
  overdue: 'Vencido',
  scheduled: 'Programado',
  completed: 'Completado',
  active: 'Activa',
  paused: 'Pausada',
  expired: 'Vencida',
  refunded: 'Devuelta',
  wanted: 'Deseado',
  saving: 'Ahorrando',
  purchased: 'Comprado',
  discarded: 'Descartado',
};

export const SOURCE_LABELS: Record<string, string> = {
  installment: 'Cuota',
  subscription: 'Suscripción',
  payable: 'Cuenta por pagar',
  receivable: 'Cuenta por cobrar',
  deposit: 'Depósito',
};

export function statusLabel(status: string): string {
  return STATUS_LABELS[status] ?? status;
}
