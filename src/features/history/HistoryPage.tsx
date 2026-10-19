import { useEffect, useMemo, useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';

import { Icon } from '../../components/Icons';
import { AmountInput, Money } from '../../components/Money';
import { EmptyState, Field, PageHeader, Stat } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { CategoryDonut, IncomeExpenseChart } from '../../components/charts/Charts';
import { useAsync } from '../../hooks/useAsync';
import { api, readError } from '../../services/api';
import { useAppData } from '../../services/store';
import { buildHistoryPdf } from '../../utils/pdf';
import type {
  CategoryBreakdown,
  HistorySummary,
  SeriesPoint,
  Transaction,
  TransactionFilter,
} from '../../types';
import {
  PAYMENT_METHODS,
  PERIOD_OPTIONS,
  TRANSACTION_TYPE_LABELS,
  formatCurrency,
  formatDate,
  todayIso,
} from '../../utils/format';

// ---------------------------------------------------------------------------
// PerÃ­odos
// ---------------------------------------------------------------------------

interface DateRange {
  start: string;
  end: string;
}

function toIso(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function addDays(date: Date, days: number): Date {
  const copy = new Date(date.getTime());
  copy.setDate(copy.getDate() + days);
  return copy;
}

function addMonths(date: Date, months: number): Date {
  const copy = new Date(date.getTime());
  const day = copy.getDate();
  copy.setDate(1);
  copy.setMonth(copy.getMonth() + months);
  const last = new Date(copy.getFullYear(), copy.getMonth() + 1, 0).getDate();
  copy.setDate(Math.min(day, last));
  return copy;
}

function parseIso(iso: string): Date {
  return new Date(`${iso}T00:00:00`);
}

/** Rango de un perÃ­odo predefinido (misma regla que usa el motor de reportes). */
function periodRange(period: string): DateRange {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const mondayOffset = (today.getDay() + 6) % 7;

  switch (period) {
    case 'today':
      return { start: toIso(today), end: toIso(today) };
    case 'yesterday': {
      const day = addDays(today, -1);
      return { start: toIso(day), end: toIso(day) };
    }
    case 'week': {
      const start = addDays(today, -mondayOffset);
      return { start: toIso(start), end: toIso(addDays(start, 6)) };
    }
    case 'last_week': {
      const start = addDays(today, -mondayOffset - 7);
      return { start: toIso(start), end: toIso(addDays(start, 6)) };
    }
    case 'month': {
      const start = new Date(today.getFullYear(), today.getMonth(), 1);
      const end = new Date(today.getFullYear(), today.getMonth() + 1, 0);
      return { start: toIso(start), end: toIso(end) };
    }
    case 'last_month': {
      const start = new Date(today.getFullYear(), today.getMonth() - 1, 1);
      const end = new Date(today.getFullYear(), today.getMonth(), 0);
      return { start: toIso(start), end: toIso(end) };
    }
    case 'year': {
      const start = new Date(today.getFullYear(), 0, 1);
      const end = new Date(today.getFullYear(), 11, 31);
      return { start: toIso(start), end: toIso(end) };
    }
    case 'last_year': {
      const start = new Date(today.getFullYear() - 1, 0, 1);
      const end = new Date(today.getFullYear() - 1, 11, 31);
      return { start: toIso(start), end: toIso(end) };
    }
    default: {
      const start = new Date(today.getFullYear(), today.getMonth(), 1);
      const end = new Date(today.getFullYear(), today.getMonth() + 1, 0);
      return { start: toIso(start), end: toIso(end) };
    }
  }
}

function shiftRange(range: DateRange, days: number): DateRange {
  return {
    start: toIso(addDays(parseIso(range.start), days)),
    end: toIso(addDays(parseIso(range.end), days)),
  };
}

function shiftRangeMonths(range: DateRange, months: number): DateRange {
  return {
    start: toIso(addMonths(parseIso(range.start), months)),
    end: toIso(addMonths(parseIso(range.end), months)),
  };
}

/** PerÃ­odo inmediatamente anterior al elegido. */
function previousRange(period: string, current: DateRange): DateRange {
  switch (period) {
    case 'today':
      return periodRange('yesterday');
    case 'yesterday':
      return shiftRange(current, -1);
    case 'week':
      return periodRange('last_week');
    case 'last_week':
      return shiftRange(current, -7);
    case 'month':
      return periodRange('last_month');
    case 'last_month':
      return shiftRangeMonths(current, -1);
    case 'year':
      return periodRange('last_year');
    case 'last_year':
      return shiftRangeMonths(current, -12);
    default: {
      const start = parseIso(current.start);
      const end = parseIso(current.end);
      const length = Math.round((end.getTime() - start.getTime()) / 86_400_000) + 1;
      return {
        start: toIso(addDays(start, -length)),
        end: toIso(addDays(start, -1)),
      };
    }
  }
}

/** PerÃ­odo anterior que tambiÃ©n sabe resolver el motor de reportes. */
const PREVIOUS_PERIOD_KEY: Record<string, string> = {
  today: 'yesterday',
  week: 'last_week',
  month: 'last_month',
  year: 'last_year',
};

// ---------------------------------------------------------------------------
// CÃ¡lculos de apoyo
// ---------------------------------------------------------------------------

function emptySummary(currencyCode: string): HistorySummary {
  return {
    currencyCode,
    income: 0,
    expenses: 0,
    net: 0,
    refunds: 0,
    transfers: 0,
    purchasesTotal: 0,
    installmentsPaid: 0,
    subscriptionsPaid: 0,
    moneyReceived: 0,
    moneyPaid: 0,
    pendingCommitments: 0,
    overdueCommitments: 0,
    transactionCount: 0,
  };
}

/** Resume los movimientos por moneda cuando el motor no entrega el resumen. */
function summarize(transactions: Transaction[]): HistorySummary[] {
  const map = new Map<string, HistorySummary>();
  for (const item of transactions) {
    if (item.voided) continue;
    const summary = map.get(item.currencyCode) ?? emptySummary(item.currencyCode);
    if (item.transactionType === 'income') {
      summary.income += item.amount;
      if (item.linkedType === 'receivable') summary.moneyReceived += item.amount;
    } else if (item.transactionType === 'expense') {
      summary.expenses += item.amount;
      if (item.linkedType === 'purchase') summary.purchasesTotal += item.amount;
      else if (item.linkedType === 'installment') summary.installmentsPaid += item.amount;
      else if (item.linkedType === 'subscription') summary.subscriptionsPaid += item.amount;
      else if (item.linkedType === 'payable') summary.moneyPaid += item.amount;
    } else if (item.transactionType === 'refund') {
      summary.refunds += item.amount;
    } else if (item.transactionType === 'transfer') {
      summary.transfers += item.amount;
    }
    summary.transactionCount += 1;
    map.set(item.currencyCode, summary);
  }
  // El balance neto se calcula al final: sin Ã©l, la tarjeta mostraba $0.
  for (const summary of map.values()) {
    summary.net = summary.income - summary.expenses + summary.refunds;
  }
  return [...map.values()];
}

/** Serie diaria de ingresos y gastos para rangos personalizados. */
function buildSeries(transactions: Transaction[], currency: string): SeriesPoint[] {
  const points = new Map<string, SeriesPoint>();
  for (const item of transactions) {
    if (item.voided) continue;
    if (item.transactionType !== 'income' && item.transactionType !== 'expense') continue;
    // Nunca se suman importes de monedas distintas.
    if (item.currencyCode !== currency) continue;
    const point = points.get(item.date) ?? { label: item.date, value: 0, secondary: 0 };
    if (item.transactionType === 'income') point.value += item.amount;
    else point.secondary = (point.secondary ?? 0) + item.amount;
    points.set(item.date, point);
  }
  return [...points.values()].sort((a, b) => a.label.localeCompare(b.label));
}

/** Gastos por categorÃ­a para rangos personalizados. */
function buildCategories(transactions: Transaction[], currency: string): CategoryBreakdown[] {
  const map = new Map<string, CategoryBreakdown>();
  for (const item of transactions) {
    if (item.voided) continue;
    if (item.transactionType !== 'expense') continue;
    if (item.currencyCode !== currency) continue;
    const label = item.categoryName ?? 'Sin categorÃ­a';
    const current = map.get(label) ?? { id: item.categoryId, label, amount: 0, color: null };
    current.amount += item.amount;
    map.set(label, current);
  }
  return [...map.values()].sort((a, b) => b.amount - a.amount);
}

async function loadSummary(
  periodKey: string | null,
  range: DateRange,
): Promise<HistorySummary[]> {
  if (!range.start || !range.end) return [];
  if (periodKey) {
    try {
      const result = await api.historySummary(periodKey);
      if (result.length > 0) return result;
    } catch {
      // El motor no pudo entregar el resumen: se calcula con los movimientos.
    }
  }
  const transactions = await api.listTransactions({
    startDate: range.start,
    endDate: range.end,
    limit: 5000,
  });
  return summarize(transactions);
}

function percentChange(current: number, previous: number): string {
  if (previous === 0) return 'Sin base de comparaciÃ³n';
  const diff = ((current - previous) / Math.abs(previous)) * 100;
  const sign = diff > 0 ? '+' : '';
  return `${sign}${diff.toFixed(1).replace('.', ',')} %`;
}

// ---------------------------------------------------------------------------
// PÃ¡gina
// ---------------------------------------------------------------------------

export function HistoryPage() {
  const { settings, accounts, expenseCategories, incomeCategories } = useAppData();
  const toast = useToast();

  const [period, setPeriod] = useState('month');
  const [customStart, setCustomStart] = useState('');
  const [customEnd, setCustomEnd] = useState('');

  const [types, setTypes] = useState<string[]>([]);
  const [accountId, setAccountId] = useState('');
  const [categoryId, setCategoryId] = useState('');
  const [paymentMethod, setPaymentMethod] = useState('');
  const [minAmount, setMinAmount] = useState<number | null>(null);
  const [maxAmount, setMaxAmount] = useState<number | null>(null);
  const [includeVoided, setIncludeVoided] = useState(false);

  useEffect(() => {
    if (period === 'custom' && !customStart && !customEnd) {
      const now = new Date();
      setCustomStart(toIso(addDays(now, -30)));
      setCustomEnd(toIso(now));
    }
  }, [period, customStart, customEnd]);

  const range = useMemo<DateRange>(
    () =>
      period === 'custom'
        ? { start: customStart, end: customEnd }
        : periodRange(period),
    [period, customStart, customEnd],
  );

  /**
   * Un rango personalizado necesita las dos fechas. Con una sola, la consulta
   * se quedarÃ­a sin lÃ­mites y devolverÃ­a todo el historial.
   */
  const rangoIncompleto = period === 'custom' && (!customStart || !customEnd);

  const previous = useMemo<DateRange>(
    () => previousRange(period, range),
    [period, range],
  );

  const currency = settings.defaultCurrency;

  const report = useAsync(
    async () => {
      const [current, before] = await Promise.all([
        loadSummary(period === 'custom' ? null : period, range),
        loadSummary(PREVIOUS_PERIOD_KEY[period] ?? null, previous),
      ]);
      return { current, before };
    },
    [period, customStart, customEnd],
  );

  const series = useAsync(async (): Promise<SeriesPoint[]> => {
    if (period === 'custom') {
      if (!customStart || !customEnd) return [];
      const transactions = await api.listTransactions({
        startDate: customStart,
        endDate: customEnd,
        includeVoided,
        limit: 5000,
      });
      return buildSeries(transactions, currency);
    }
    return api.incomeExpenseSeries(period, currency, 'day');
  }, [period, currency, customStart, customEnd, includeVoided]);

  const categories = useAsync(async (): Promise<CategoryBreakdown[]> => {
    if (period === 'custom') {
      if (!customStart || !customEnd) return [];
      const transactions = await api.listTransactions({
        startDate: customStart,
        endDate: customEnd,
        includeVoided,
        limit: 5000,
      });
      return buildCategories(transactions, currency);
    }
    return api.expensesByCategory(period, currency);
  }, [period, currency, customStart, customEnd, includeVoided]);

  const filter = useMemo<TransactionFilter>(
    () => ({
      // El rango se envía siempre explícito: el backend lo usa como límite.
      startDate: range.start || null,
      endDate: range.end || null,
      types: types.length > 0 ? types : null,
      accountIds: accountId ? [accountId] : null,
      categoryIds: categoryId ? [categoryId] : null,
      paymentMethod: paymentMethod || null,
      minAmount,
      maxAmount,
      includeVoided,
      sort: 'date',
      direction: 'desc',
      limit: 500,
    }),
    [
      period,
      range.start,
      range.end,
      types,
      accountId,
      categoryId,
      paymentMethod,
      minAmount,
      maxAmount,
      includeVoided,
    ],
  );

  const movements = useAsync(
    () =>
      rangoIncompleto
        ? Promise.resolve([] as Transaction[])
        : api.listTransactions(filter),
    [filter, rangoIncompleto],
  );

  /** Total real de movimientos que cumplen los filtros (la tabla muestra 500). */
  const totalMovimientos = useAsync(
    () =>
      rangoIncompleto
        ? Promise.resolve(0)
        : api.countTransactions({ ...filter, limit: null }),
    [filter, rangoIncompleto],
  );

  const current = useMemo(
    () => (report.data?.current ?? []).find((item) => item.currencyCode === currency) ?? null,
    [report.data, currency],
  );
  const before = useMemo(
    () => (report.data?.before ?? []).find((item) => item.currencyCode === currency) ?? null,
    [report.data, currency],
  );
  const otherCurrencies = useMemo(
    () => (report.data?.current ?? []).filter((item) => item.currencyCode !== currency),
    [report.data, currency],
  );

  const chartLabels = useMemo(
    () => (series.data ?? []).map((point) => point.label),
    [series.data],
  );
  const chartIncome = useMemo(
    () => (series.data ?? []).map((point) => point.value),
    [series.data],
  );
  const chartExpenses = useMemo(
    () => (series.data ?? []).map((point) => point.secondary ?? 0),
    [series.data],
  );

  const allCategories = [...expenseCategories, ...incomeCategories];

  const toggleType = (type: string) => {
    setTypes((current) =>
      current.includes(type)
        ? current.filter((item) => item !== type)
        : [...current, type],
    );
  };

  const clearFilters = () => {
    setTypes([]);
    setAccountId('');
    setCategoryId('');
    setPaymentMethod('');
    setMinAmount(null);
    setMaxAmount(null);
  };

  const handleExport = async () => {
    try {
      const path = await save({
        defaultPath: `aureon-movimientos-${todayIso()}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!path) return;
      // Sin el lÃ­mite de la tabla: el archivo sale completo.
      const contents = await api.exportDataset('transactions', {
        ...filter,
        limit: null,
      });
      await api.writeTextFile(path, contents);
      toast.success(`Movimientos exportados en ${path}.`);
    } catch (err) {
      toast.error(readError(err));
    }
  };

  const handleExportPdf = async () => {
    try {
      const path = await save({
        defaultPath: `aureon-reporte-${todayIso()}.pdf`,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
      if (!path) return;
      const transactions = await api.listTransactions({ ...filter, limit: 5000 });
      // El resumen del PDF se calcula con los movimientos exportados, para que
      // los totales cuadren con las filas de la tabla.
      const resumen = summarize(transactions).find(
        (item) => item.currencyCode === currency,
      ) ?? null;
      const base64 = await buildHistoryPdf({
        periodLabel: periodLabel,
        currency,
        summary: resumen,
        transactions,
        generatedAt: todayIso(),
      });
      await api.writeBase64File(path, base64);
      toast.success(`Reporte guardado en ${path}.`);
    } catch (err) {
      toast.error(readError(err));
    }
  };

  const periodLabel =
    period === 'custom'
      ? `${formatDate(range.start)} â€“ ${formatDate(range.end)}`
      : (PERIOD_OPTIONS.find((option) => option.value === period)?.label ?? 'PerÃ­odo');

  return (
    <>
      <PageHeader
        title="Historial y reportes"
        subtitle="Consulta tus ingresos, gastos y compromisos por perÃ­odo."
        actions={
          <>
            <button type="button" className="btn" onClick={() => void handleExportPdf()}>
              <Icon name="download" size={14} />
              Exportar PDF
            </button>
            <button type="button" className="btn" onClick={() => void handleExport()}>
              <Icon name="download" size={14} />
              Exportar CSV
            </button>
          </>
        }
      />

      <section className="card">
        <div className="filter-bar">
          <Field label="PerÃ­odo">
            {(id) => (
              <select
                id={id}
                className="select"
                value={period}
                onChange={(event) => setPeriod(event.target.value)}
              >
                {PERIOD_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
                <option value="custom">Rango personalizado</option>
              </select>
            )}
          </Field>

          {period === 'custom' ? (
            <>
              <Field label="Desde">
                {(id) => (
                  <input
                    id={id}
                    className="input"
                    type="date"
                    value={customStart}
                    onChange={(event) => setCustomStart(event.target.value)}
                  />
                )}
              </Field>
              <Field label="Hasta">
                {(id) => (
                  <input
                    id={id}
                    className="input"
                    type="date"
                    value={customEnd}
                    onChange={(event) => setCustomEnd(event.target.value)}
                  />
                )}
              </Field>
            </>
          ) : null}

          <span className="text-sm text-muted">
            {formatDate(range.start)} â€“ {formatDate(range.end)}
          </span>

          <label className="checkbox" style={{ marginBottom: 6 }}>
            <input
              type="checkbox"
              checked={includeVoided}
              onChange={(event) => setIncludeVoided(event.target.checked)}
            />
            Mostrar anulados
          </label>
        </div>
      </section>

      {rangoIncompleto ? (
        <div className="alert alert-warning">
          <Icon name="alert" size={15} />
          <span>
            Elige las dos fechas del rango para consultar los movimientos. Con una sola
            fecha no se puede acotar la bÃºsqueda.
          </span>
        </div>
      ) : null}

      {report.error ? <div className="alert alert-error">{report.error}</div> : null}

      <div className="stat-grid">
        <Stat
          label="Ingresos"
          value={formatCurrency(current?.income ?? 0, currency)}
          meta="Entradas del perÃ­odo"
          tone="income"
        />
        <Stat
          label="Gastos"
          value={formatCurrency(current?.expenses ?? 0, currency)}
          meta="Salidas del perÃ­odo"
          tone="expense"
        />
        <Stat
          label="Balance neto"
          value={formatCurrency(current?.net ?? 0, currency)}
          meta="Ingresos menos gastos"
          tone="accent"
        />
        <Stat
          label="Movimientos"
          value={String(current?.transactionCount ?? 0)}
          meta="Operaciones registradas en el perÃ­odo"
        />
      </div>

      <div className="stat-grid">
        <Stat
          label="Devoluciones"
          value={formatCurrency(current?.refunds ?? 0, currency)}
          meta="Dinero que volviÃ³ a tus cuentas"
        />
        <Stat
          label="Transferencias entre cuentas"
          value={formatCurrency(current?.transfers ?? 0, currency)}
          meta="Movimientos internos, no son ingresos ni gastos"
        />
        <Stat
          label="Compras realizadas"
          value={formatCurrency(current?.purchasesTotal ?? 0, currency)}
          meta="Compras pagadas al contado"
        />
        <Stat
          label="Pagos de cuotas"
          value={formatCurrency(current?.installmentsPaid ?? 0, currency)}
          meta="Cuotas de compras pagadas"
        />
        <Stat
          label="Pagos de suscripciones"
          value={formatCurrency(current?.subscriptionsPaid ?? 0, currency)}
          meta="Cobros de servicios recurrentes"
        />
        <Stat
          label="Dinero cobrado"
          value={formatCurrency(current?.moneyReceived ?? 0, currency)}
          meta="Recibido de cuentas por cobrar"
        />
        <Stat
          label="Dinero pagado"
          value={formatCurrency(current?.moneyPaid ?? 0, currency)}
          meta="Pagado por cuentas por pagar"
        />
      </div>

      <section className="card">
        <header className="card-header">
          <h2 className="card-title">ComparaciÃ³n con el perÃ­odo anterior</h2>
          <span className="text-sm text-muted">{currency}</span>
        </header>
        <div className="card-body">
          <div className="detail-list">
            <div className="detail-row">
              <span className="detail-key">Ingresos</span>
              <span className="detail-value">
                {formatCurrency(current?.income ?? 0, currency)}
                <span className="text-sm text-muted">
                  {' Â· '}
                  {percentChange(current?.income ?? 0, before?.income ?? 0)}
                </span>
              </span>
            </div>
            <div className="detail-row">
              <span className="detail-key">Gastos</span>
              <span className="detail-value">
                {formatCurrency(current?.expenses ?? 0, currency)}
                <span className="text-sm text-muted">
                  {' Â· '}
                  {percentChange(current?.expenses ?? 0, before?.expenses ?? 0)}
                </span>
              </span>
            </div>
            <div className="detail-row">
              <span className="detail-key">Movimientos</span>
              <span className="detail-value">
                {current?.transactionCount ?? 0}
                <span className="text-sm text-muted">
                  {' Â· '}
                  {percentChange(
                    current?.transactionCount ?? 0,
                    before?.transactionCount ?? 0,
                  )}
                </span>
              </span>
            </div>
          </div>
          <p className="text-sm text-soft" style={{ marginTop: 'var(--space-4)' }}>
            La comparaciÃ³n usa el perÃ­odo equivalente mÃ¡s cercano: {periodLabel} contra el
            perÃ­odo anterior.
          </p>
        </div>
      </section>

      <div className="grid-main">
        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Ingresos frente a gastos</h2>
            <span className="text-sm text-muted">{periodLabel}</span>
          </header>
          <div className="card-body">
            {series.loading ? (
              <div className="loading">
                <span className="spinner" />
              </div>
            ) : series.error ? (
              <div className="alert alert-error">{series.error}</div>
            ) : chartLabels.length === 0 ? (
              <EmptyState
                icon="chart"
                title="Sin movimientos en este perÃ­odo"
                text="Registra ingresos y gastos para ver su evoluciÃ³n."
              />
            ) : (
              <IncomeExpenseChart
                labels={chartLabels}
                income={chartIncome}
                expenses={chartExpenses}
                currency={currency}
              />
            )}
          </div>
        </section>

        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Gastos por categorÃ­a</h2>
          </header>
          <div className="card-body">
            {categories.loading ? (
              <div className="loading">
                <span className="spinner" />
              </div>
            ) : categories.error ? (
              <div className="alert alert-error">{categories.error}</div>
            ) : (categories.data ?? []).length === 0 ? (
              <EmptyState icon="target" title="Sin gastos registrados" />
            ) : (
              <CategoryDonut slices={categories.data ?? []} currency={currency} />
            )}
          </div>
        </section>
      </div>

      {otherCurrencies.length > 0 ? (
        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Otras monedas del perÃ­odo</h2>
          </header>
          <div className="card-body">
            <div className="grid-2">
              {otherCurrencies.map((item) => (
                <div key={item.currencyCode} className="detail-list">
                  <div className="detail-row">
                    <span className="detail-key">Moneda</span>
                    <span className="detail-value">{item.currencyCode}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Ingresos</span>
                    <span className="detail-value">
                      {formatCurrency(item.income, item.currencyCode)}
                    </span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Gastos</span>
                    <span className="detail-value">
                      {formatCurrency(item.expenses, item.currencyCode)}
                    </span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Balance neto</span>
                    <span className="detail-value">
                      {formatCurrency(item.net, item.currencyCode)}
                    </span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Movimientos</span>
                    <span className="detail-value">{item.transactionCount}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>
      ) : null}

      <section className="card">
        <div className="filter-bar">
          <Field label="Cuenta">
            {(id) => (
              <select
                id={id}
                className="select"
                value={accountId}
                onChange={(event) => setAccountId(event.target.value)}
              >
                <option value="">Todas</option>
                {accounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="CategorÃ­a">
            {(id) => (
              <select
                id={id}
                className="select"
                value={categoryId}
                onChange={(event) => setCategoryId(event.target.value)}
              >
                <option value="">Todas</option>
                {allCategories.map((category) => (
                  <option key={category.id} value={category.id}>
                    {category.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="MÃ©todo de pago">
            {(id) => (
              <select
                id={id}
                className="select"
                value={paymentMethod}
                onChange={(event) => setPaymentMethod(event.target.value)}
              >
                <option value="">Todos</option>
                {PAYMENT_METHODS.map((method) => (
                  <option key={method} value={method}>
                    {method}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Monto mÃ­nimo">
            {(id) => (
              <AmountInput
                id={id}
                value={minAmount}
                currency={currency}
                onChange={setMinAmount}
              />
            )}
          </Field>

          <Field label="Monto mÃ¡ximo">
            {(id) => (
              <AmountInput
                id={id}
                value={maxAmount}
                currency={currency}
                onChange={setMaxAmount}
              />
            )}
          </Field>

          <div className="field">
            <span className="field-label">Tipo</span>
            <div className="row" style={{ gap: 6 }}>
              {Object.entries(TRANSACTION_TYPE_LABELS).map(([value, label]) => (
                <button
                  key={value}
                  type="button"
                  className="btn btn-sm"
                  aria-pressed={types.includes(value)}
                  style={
                    types.includes(value)
                      ? {
                          background: 'var(--accent-soft)',
                          borderColor: 'var(--accent)',
                          color: 'var(--accent-strong)',
                        }
                      : undefined
                  }
                  onClick={() => toggleType(value)}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          <button type="button" className="btn" onClick={clearFilters}>
            <Icon name="close" size={13} />
            Limpiar filtros
          </button>
        </div>

        <div className="card-body">
          <div className="row-between" style={{ marginBottom: 'var(--space-4)' }}>
            <span className="text-sm text-muted">
              {totalMovimientos.data ?? (movements.data ?? []).length} movimiento
              {(totalMovimientos.data ?? (movements.data ?? []).length) === 1 ? '' : 's'} con
              los filtros aplicados
              {(totalMovimientos.data ?? 0) > (movements.data ?? []).length
                ? ` (se muestran los ${(movements.data ?? []).length} mÃ¡s recientes; el CSV y el PDF incluyen todos)`
                : ''}
            </span>
            <button type="button" className="btn btn-sm" onClick={() => void handleExport()}>
              <Icon name="download" size={13} />
              Exportar CSV
            </button>
          </div>

          {movements.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : movements.error ? (
            <div className="alert alert-error">{movements.error}</div>
          ) : (movements.data ?? []).length === 0 ? (
            <EmptyState
              icon="arrows"
              title="Sin movimientos con estos filtros"
              text="Cambia el perÃ­odo o quita algÃºn filtro para ver mÃ¡s resultados."
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Fecha</th>
                    <th>Concepto</th>
                    <th>CategorÃ­a</th>
                    <th>Cuenta</th>
                    <th className="num">Monto</th>
                  </tr>
                </thead>
                <tbody>
                  {(movements.data ?? []).map((item) => (
                    <tr key={item.id}>
                      <td className="text-sm">{formatDate(item.date)}</td>
                      <td>
                        <div className="row" style={{ gap: 8 }}>
                          <span
                            className={`badge ${
                              item.transactionType === 'income'
                                ? 'badge-income'
                                : item.transactionType === 'expense'
                                  ? 'badge-expense'
                                  : item.transactionType === 'refund'
                                    ? 'badge-info'
                                    : 'badge-accent'
                            }`}
                          >
                            {TRANSACTION_TYPE_LABELS[item.transactionType]}
                          </span>
                          <span className={item.voided ? 'text-soft' : ''}>
                            {item.description}
                          </span>
                        </div>
                        {item.voided ? (
                          <span className="text-xs text-soft">Anulado</span>
                        ) : null}
                      </td>
                      <td className="text-sm text-muted">{item.categoryName ?? 'â€”'}</td>
                      <td className="text-sm text-muted">
                        {item.transactionType === 'transfer'
                          ? `${item.accountName} â†’ ${item.transferAccountName}`
                          : (item.accountName ?? 'â€”')}
                      </td>
                      <td className="num">
                        <Money
                          value={
                            item.transactionType === 'income' ||
                            item.transactionType === 'refund'
                              ? item.amount
                              : -item.amount
                          }
                          currency={item.currencyCode}
                          tone={
                            item.transactionType === 'income' ||
                            item.transactionType === 'refund'
                              ? 'income'
                              : 'expense'
                          }
                        />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </section>
    </>
  );
}
