import { useEffect, useMemo, useState } from 'react';

import { Icon } from '../../components/Icons';
import { AmountInput, Money } from '../../components/Money';
import { ConfirmDialog, Modal } from '../../components/Modal';
import { EmptyState, Field, PageHeader, Stat } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import type { Frequency, Installment, Purchase, PurchaseInput } from '../../types';
import {
  FREQUENCY_LABELS,
  PAYMENT_METHODS,
  dueLabel,
  formatCurrency,
  formatDate,
  todayIso,
} from '../../utils/format';

const PURCHASE_STATUS_LABELS: Record<string, string> = {
  active: 'En curso',
  completed: 'Completada',
  cancelled: 'Cancelada',
  refunded: 'Devuelta',
};

const INSTALLMENT_STATUS_LABELS: Record<string, string> = {
  pending: 'Pendiente',
  paid: 'Pagada',
  overdue: 'Vencida',
  cancelled: 'Cancelada',
};

const PURCHASE_FILTERS = [
  { value: '', label: 'Todas' },
  { value: 'active', label: 'En curso' },
  { value: 'completed', label: 'Completadas' },
  { value: 'cancelled', label: 'Canceladas' },
  { value: 'refunded', label: 'Devueltas' },
];

const FREQUENCY_OPTIONS: { value: Frequency; label: string }[] = (
  Object.keys(FREQUENCY_LABELS) as Frequency[]
).map((value) => ({ value, label: FREQUENCY_LABELS[value] }));

function purchaseBadge(status: string): string {
  if (status === 'completed') return 'badge badge-income';
  if (status === 'cancelled') return 'badge';
  if (status === 'refunded') return 'badge badge-warning';
  return 'badge badge-info';
}

function installmentBadge(status: string): string {
  if (status === 'paid') return 'badge badge-income';
  if (status === 'overdue') return 'badge badge-expense';
  if (status === 'pending') return 'badge badge-info';
  return 'badge';
}

export function PurchasesPage() {
  const { settings, refresh } = useAppData();
  const toast = useToast();

  const [status, setStatus] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState<Purchase | null>(null);
  const [busy, setBusy] = useState(false);

  const purchases = useAsync(() => api.listPurchases(status || undefined), [status]);
  const items = purchases.data ?? [];

  const stats = useMemo(() => {
    const list = purchases.data ?? [];
    const currency = settings.defaultCurrency;
    const pending = list
      .filter((item) => item.currencyCode === currency)
      .reduce((acc, item) => acc + item.pendingAmount, 0);
    const paid = list.reduce((acc, item) => acc + item.installmentsPaid, 0);
    const total = list.reduce((acc, item) => acc + item.installmentsCount, 0);
    const overdue = list.reduce((acc, item) => acc + item.installmentsOverdue, 0);
    return { currency, pending, paid, total, overdue };
  }, [purchases.data, settings.defaultCurrency]);

  const handleCancel = async () => {
    if (!cancelling) return;
    setBusy(true);
    try {
      await api.cancelPurchase(cancelling.id);
      toast.success('Compra cancelada.');
      setCancelling(null);
      setDetailId(null);
      purchases.reload();
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo cancelar la compra.');
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="Compras"
        subtitle="Controla tus compras en cuotas y el calendario de vencimientos de cada una."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setFormOpen(true)}
          >
            <Icon name="plus" size={14} />
            Nueva compra
          </button>
        }
      />

      <div className="stat-grid">
        <Stat
          label="Compras"
          value={String(items.length)}
          meta="En el filtro actual"
        />
        <Stat
          label="Cuotas pagadas"
          value={`${stats.paid} de ${stats.total}`}
          meta="Cuotas saldadas de todas las compras"
        />
        <Stat
          label="Cuotas vencidas"
          value={String(stats.overdue)}
          tone={stats.overdue > 0 ? 'warning' : 'default'}
          meta={stats.overdue > 0 ? 'Revisa los próximos vencimientos' : 'Todo al día'}
        />
        <Stat
          label="Saldo pendiente"
          value={formatCurrency(stats.pending, stats.currency)}
          meta={`Sólo compras en ${stats.currency}`}
          tone="expense"
        />
      </div>

      <section className="card">
        <div className="filter-bar">
          <div className="segmented" role="group" aria-label="Estado de las compras">
            {PURCHASE_FILTERS.map((filter) => (
              <button
                key={filter.value}
                type="button"
                aria-pressed={status === filter.value}
                onClick={() => setStatus(filter.value)}
              >
                {filter.label}
              </button>
            ))}
          </div>
        </div>

        <div className="card-body">
          {purchases.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : purchases.error ? (
            <div className="alert alert-error">{purchases.error}</div>
          ) : items.length === 0 ? (
            <EmptyState
              icon="bag"
              title={
                status === ''
                  ? 'Sin compras registradas'
                  : 'Sin compras en este estado'
              }
              text="Registra una compra para llevar el control de sus cuotas, vencimientos y saldos."
              action={
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => setFormOpen(true)}
                >
                  <Icon name="plus" size={14} />
                  Nueva compra
                </button>
              }
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Compra</th>
                    <th>Fecha</th>
                    <th className="num">Monto total</th>
                    <th>Cuotas</th>
                    <th className="num">Saldo pendiente</th>
                    <th>Próximo vencimiento</th>
                    <th>Estado</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {items.map((item) => {
                    const percent =
                      item.installmentsCount > 0
                        ? Math.round(
                            (item.installmentsPaid / item.installmentsCount) * 100,
                          )
                        : 0;
                    return (
                      <tr key={item.id}>
                        <td>
                          <button
                            type="button"
                            className="btn btn-ghost"
                            style={{ height: 'auto', padding: 0 }}
                            onClick={() => setDetailId(item.id)}
                          >
                            {item.name}
                          </button>
                          <div className="text-sm text-soft">
                            {item.contactName ?? 'Sin comercio'}
                          </div>
                        </td>
                        <td className="text-sm">{formatDate(item.purchaseDate)}</td>
                        <td className="num">
                          <Money value={item.totalAmount} currency={item.currencyCode} />
                        </td>
                        <td style={{ minWidth: 170 }}>
                          <div className="text-sm">
                            {item.installmentsPaid} de {item.installmentsCount} cuotas pagadas
                          </div>
                          <div className="progress">
                            <div
                              className={`progress-bar${
                                item.installmentsOverdue > 0 ? ' is-over' : ''
                              }`}
                              style={{ width: `${percent}%` }}
                            />
                          </div>
                        </td>
                        <td className="num">
                          {item.pendingAmount > 0 ? (
                            <Money
                              value={item.pendingAmount}
                              currency={item.currencyCode}
                            />
                          ) : (
                            <span className="text-soft">—</span>
                          )}
                        </td>
                        <td className="text-sm">
                          {item.nextDueDate ? (
                            <>
                              {formatDate(item.nextDueDate)}
                              <div className="text-sm text-soft">
                                {dueLabel(item.nextDueDate)}
                              </div>
                            </>
                          ) : (
                            <span className="text-soft">Sin vencimientos</span>
                          )}
                        </td>
                        <td>
                          <span className={purchaseBadge(item.status)}>
                            {PURCHASE_STATUS_LABELS[item.status] ?? item.status}
                          </span>
                        </td>
                        <td>
                          <div className="table-actions">
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Ver detalle"
                              aria-label={`Ver detalle de ${item.name}`}
                              onClick={() => setDetailId(item.id)}
                            >
                              <Icon name="chevron-right" size={14} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </section>

      <PurchaseFormModal
        open={formOpen}
        onClose={() => setFormOpen(false)}
        onSaved={() => {
          purchases.reload();
          void refresh();
        }}
      />

      {detailId ? (
        <PurchaseDetailModal
          purchaseId={detailId}
          onClose={() => setDetailId(null)}
          onChanged={() => {
            purchases.reload();
            void refresh();
          }}
          onRequestCancel={(purchase) => setCancelling(purchase)}
        />
      ) : null}

      <ConfirmDialog
        open={cancelling !== null}
        title="Cancelar compra"
        message={
          cancelling
            ? `Se cancelarán las cuotas pendientes de «${cancelling.name}». Los pagos ya registrados se conservan, pero no podrás registrar nuevos pagos ni devoluciones.`
            : ''
        }
        confirmLabel="Cancelar compra"
        danger
        busy={busy}
        onConfirm={handleCancel}
        onCancel={() => setCancelling(null)}
      />
    </>
  );
}

interface PurchaseDetailProps {
  purchaseId: string;
  onClose: () => void;
  onChanged: () => void;
  onRequestCancel: (purchase: Purchase) => void;
}

function PurchaseDetailModal({
  purchaseId,
  onClose,
  onChanged,
  onRequestCancel,
}: PurchaseDetailProps) {
  const { accounts } = useAppData();
  const toast = useToast();

  const purchase = useAsync(() => api.getPurchase(purchaseId), [purchaseId]);
  const installments = useAsync(() => api.listInstallments(purchaseId), [purchaseId]);

  const [mode, setMode] = useState<'none' | 'pay' | 'refund'>('none');
  const [target, setTarget] = useState<Installment | null>(null);
  const [amount, setAmount] = useState<number | null>(null);
  const [date, setDate] = useState(todayIso());
  const [accountId, setAccountId] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const data = purchase.data;
  const currency = data?.currencyCode ?? 'CLP';
  const purchaseUrl = data?.url ?? null;
  const remaining = target ? target.amount - target.paidAmount : 0;
  const percent =
    data && data.installmentsCount > 0
      ? Math.round((data.installmentsPaid / data.installmentsCount) * 100)
      : 0;

  const startPay = (installment: Installment) => {
    setMode('pay');
    setTarget(installment);
    setAmount(installment.amount - installment.paidAmount);
    setDate(todayIso());
    setAccountId(installment.accountId ?? data?.accountId ?? '');
    setError(null);
  };

  const startRefund = () => {
    setMode('refund');
    setTarget(null);
    setAmount(data ? data.paidAmount : null);
    setDate(todayIso());
    setAccountId(data?.accountId ?? '');
    setError(null);
  };

  const cancelForm = () => {
    setMode('none');
    setTarget(null);
    setError(null);
  };

  const submitPay = async () => {
    if (!target) return;
    if (!amount || amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (amount > remaining) {
      setError('El pago supera el saldo pendiente de la cuota.');
      return;
    }
    if (!date) {
      setError('Indica la fecha del pago.');
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await api.payInstallment(target.id, {
        amount,
        paidAt: date,
        accountId: accountId || null,
      });
      toast.success(
        amount === remaining ? 'Cuota pagada.' : 'Pago parcial registrado.',
      );
      setMode('none');
      setTarget(null);
      installments.reload();
      purchase.reload();
      onChanged();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo registrar el pago.');
    } finally {
      setSaving(false);
    }
  };

  const submitRefund = async () => {
    if (!data) return;
    if (!amount || amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (amount > data.paidAmount) {
      setError('La devolución supera el monto ya pagado de la compra.');
      return;
    }
    if (!date) {
      setError('Indica la fecha de la devolución.');
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await api.refundPurchase(data.id, amount, date, accountId || null);
      toast.success('Devolución registrada.');
      setMode('none');
      installments.reload();
      purchase.reload();
      onChanged();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'No se pudo registrar la devolución.',
      );
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open
      wide
      title={data?.name ?? 'Detalle de la compra'}
      subtitle={data?.contactName ?? undefined}
      onClose={onClose}
      footer={
        <button type="button" className="btn" onClick={onClose}>
          Cerrar
        </button>
      }
    >
      {purchase.loading ? (
        <div className="loading">
          <span className="spinner" />
        </div>
      ) : purchase.error ? (
        <div className="alert alert-error">{purchase.error}</div>
      ) : !data ? null : (
        <>
          <div className="grid-2">
            <div className="detail-list">
              <div className="detail-row">
                <span className="detail-key">Fecha de compra</span>
                <span className="detail-value">
                  {formatDate(data.purchaseDate)}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Comercio</span>
                <span className="detail-value">
                  {data.contactName ?? 'Sin comercio'}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Monto total</span>
                <span className="detail-value">
                  <Money value={data.totalAmount} currency={currency} />
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Pagado</span>
                <span className="detail-value">
                  <Money value={data.paidAmount} currency={currency} />
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Saldo pendiente</span>
                <span className="detail-value">
                  <Money value={data.pendingAmount} currency={currency} />
                </span>
              </div>
            </div>

            <div className="detail-list">
              <div className="detail-row">
                <span className="detail-key">Próximo vencimiento</span>
                <span className="detail-value">
                  {data.nextDueDate
                    ? `${formatDate(data.nextDueDate)} · ${dueLabel(data.nextDueDate)}`
                    : 'Sin vencimientos'}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Frecuencia</span>
                <span className="detail-value">
                  {FREQUENCY_LABELS[data.frequency] ?? data.frequency}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Método de pago</span>
                <span className="detail-value">
                  {data.paymentMethod ?? '—'}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Cuenta</span>
                <span className="detail-value">{data.accountName ?? '—'}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Categoría</span>
                <span className="detail-value">{data.categoryName ?? '—'}</span>
              </div>
            </div>
          </div>

          <div className="form-section">
            <p className="form-section-title">
              {data.installmentsPaid} de {data.installmentsCount} cuotas pagadas
            </p>
            <div className="progress">
              <div
                className={`progress-bar${
                  data.installmentsOverdue > 0 ? ' is-over' : ''
                }`}
                style={{ width: `${percent}%` }}
              />
            </div>
            <div className="row">
              <span className={purchaseBadge(data.status)}>
                {PURCHASE_STATUS_LABELS[data.status] ?? data.status}
              </span>
              <button
                type="button"
                className="btn btn-sm"
                onClick={startRefund}
                disabled={data.status === 'cancelled' || data.paidAmount <= 0}
              >
                <Icon name="outgoing" size={13} />
                Devolver dinero
              </button>
              <button
                type="button"
                className="btn btn-sm btn-danger"
                onClick={() => onRequestCancel(data)}
                disabled={data.status === 'cancelled'}
              >
                <Icon name="close" size={13} />
                Cancelar compra
              </button>
            </div>
          </div>

          {purchaseUrl ? (
            <div className="form-section">
              <p className="form-section-title">Enlace del producto</p>
              <button
                type="button"
                className="btn btn-sm"
                onClick={() => {
                  void api
                    .openExternalUrl(purchaseUrl)
                    .catch(() => toast.error('No se pudo abrir el enlace.'));
                }}
              >
                <Icon name="external" size={13} />
                Abrir enlace
              </button>
            </div>
          ) : null}

          {data.notes ? (
            <div className="form-section">
              <p className="form-section-title">Notas</p>
              <p className="text-sm text-muted">{data.notes}</p>
            </div>
          ) : null}

          {mode === 'pay' && target ? (
            <div className="form-section">
              <p className="form-section-title">
                Registrar pago de la cuota {target.number}
              </p>
              {error ? <div className="alert alert-error">{error}</div> : null}
              <div className="form-grid">
                <Field
                  label="Monto a pagar"
                  required
                  hint={`Saldo pendiente: ${formatCurrency(remaining, currency)}`}
                >
                  {(id) => (
                    <AmountInput
                      id={id}
                      value={amount}
                      currency={currency}
                      onChange={setAmount}
                    />
                  )}
                </Field>
                <Field label="Fecha del pago" required>
                  {(id) => (
                    <input
                      id={id}
                      className="input"
                      type="date"
                      value={date}
                      onChange={(event) => setDate(event.target.value)}
                    />
                  )}
                </Field>
                <Field
                  label="Cuenta"
                  className="span-2"
                  hint="Si la dejas vacía se usa la cuenta de la compra."
                >
                  {(id) => (
                    <select
                      id={id}
                      className="select"
                      value={accountId}
                      onChange={(event) => setAccountId(event.target.value)}
                    >
                      <option value="">Sin especificar</option>
                      {accounts.map((account) => (
                        <option key={account.id} value={account.id}>
                          {account.name} · {account.currencyCode}
                        </option>
                      ))}
                    </select>
                  )}
                </Field>
              </div>
              <div className="row">
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => void submitPay()}
                  disabled={saving}
                >
                  {saving ? 'Registrando…' : 'Registrar pago'}
                </button>
                <button
                  type="button"
                  className="btn"
                  onClick={cancelForm}
                  disabled={saving}
                >
                  Cancelar
                </button>
              </div>
            </div>
          ) : null}

          {mode === 'refund' ? (
            <div className="form-section">
              <p className="form-section-title">Registrar devolución</p>
              {error ? <div className="alert alert-error">{error}</div> : null}
              <div className="form-grid">
                <Field
                  label="Monto devuelto"
                  required
                  hint={`Máximo devuelto: ${formatCurrency(
                    data?.paidAmount ?? 0,
                    currency,
                  )}`}
                >
                  {(id) => (
                    <AmountInput
                      id={id}
                      value={amount}
                      currency={currency}
                      onChange={setAmount}
                    />
                  )}
                </Field>
                <Field label="Fecha de la devolución" required>
                  {(id) => (
                    <input
                      id={id}
                      className="input"
                      type="date"
                      value={date}
                      onChange={(event) => setDate(event.target.value)}
                    />
                  )}
                </Field>
                <Field
                  label="Cuenta"
                  className="span-2"
                  hint="Si la dejas vacía se usa la cuenta de la compra."
                >
                  {(id) => (
                    <select
                      id={id}
                      className="select"
                      value={accountId}
                      onChange={(event) => setAccountId(event.target.value)}
                    >
                      <option value="">Sin especificar</option>
                      {accounts.map((account) => (
                        <option key={account.id} value={account.id}>
                          {account.name} · {account.currencyCode}
                        </option>
                      ))}
                    </select>
                  )}
                </Field>
              </div>
              <div className="row">
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => void submitRefund()}
                  disabled={saving}
                >
                  {saving ? 'Registrando…' : 'Registrar devolución'}
                </button>
                <button
                  type="button"
                  className="btn"
                  onClick={cancelForm}
                  disabled={saving}
                >
                  Cancelar
                </button>
              </div>
            </div>
          ) : null}

          <div className="form-section">
            <p className="form-section-title">Calendario de cuotas</p>
            {installments.loading ? (
              <div className="loading">
                <span className="spinner" />
              </div>
            ) : installments.error ? (
              <div className="alert alert-error">{installments.error}</div>
            ) : (installments.data ?? []).length === 0 ? (
              <EmptyState
                icon="calendar"
                title="Esta compra no tiene cuotas"
                text="El calendario de vencimientos aparecerá en cuanto se generen las cuotas."
              />
            ) : (
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th>Cuota</th>
                      <th>Vencimiento</th>
                      <th className="num">Monto</th>
                      <th className="num">Pagado</th>
                      <th>Estado</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {(installments.data ?? []).map((installment) => {
                      const saldo = installment.amount - installment.paidAmount;
                      return (
                        <tr key={installment.id}>
                          <td className="text-sm">
                            {installment.number} de {data.installmentsCount}
                          </td>
                          <td className="text-sm">
                            {formatDate(installment.dueDate)}
                            <div className="text-sm text-soft">
                              {dueLabel(installment.dueDate)}
                            </div>
                          </td>
                          <td className="num">
                            <Money
                              value={installment.amount}
                              currency={currency}
                            />
                          </td>
                          <td className="num">
                            {installment.paidAmount > 0 ? (
                              <Money
                                value={installment.paidAmount}
                                currency={currency}
                              />
                            ) : (
                              <span className="text-soft">—</span>
                            )}
                          </td>
                          <td>
                            <span className={installmentBadge(installment.status)}>
                              {INSTALLMENT_STATUS_LABELS[installment.status] ??
                                installment.status}
                            </span>
                            {installment.paidAt ? (
                              <div className="text-sm text-soft">
                                Pagada el {formatDate(installment.paidAt)}
                                {installment.accountName
                                  ? ` · ${installment.accountName}`
                                  : ''}
                              </div>
                            ) : null}
                          </td>
                          <td>
                            {saldo > 0 &&
                            installment.status !== 'cancelled' ? (
                              <button
                                type="button"
                                className="btn btn-sm"
                                onClick={() => startPay(installment)}
                              >
                                <Icon name="coins" size={13} />
                                Pagar
                              </button>
                            ) : null}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </>
      )}
    </Modal>
  );
}

interface PurchaseFormProps {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}

interface PurchaseFormState {
  name: string;
  contactName: string;
  url: string;
  purchaseDate: string;
  totalAmount: number | null;
  currency: string;
  installmentsCount: number;
  installmentAmount: number | null;
  firstDueDate: string;
  frequency: Frequency;
  frequencyDays: number;
  paymentMethod: string;
  accountId: string;
  categoryId: string;
  notes: string;
  paidImmediately: boolean;
}

function emptyPurchaseForm(currency: string): PurchaseFormState {
  return {
    name: '',
    contactName: '',
    url: '',
    purchaseDate: todayIso(),
    totalAmount: null,
    currency,
    installmentsCount: 1,
    installmentAmount: null,
    firstDueDate: '',
    frequency: 'monthly',
    frequencyDays: 30,
    paymentMethod: '',
    accountId: '',
    categoryId: '',
    notes: '',
    paidImmediately: false,
  };
}

function PurchaseFormModal({ open, onClose, onSaved }: PurchaseFormProps) {
  const { settings, currencies, accounts, expenseCategories, refresh } = useAppData();
  const toast = useToast();
  const [form, setForm] = useState<PurchaseFormState>(
    emptyPurchaseForm(settings.defaultCurrency),
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setError(null);
    setForm(emptyPurchaseForm(settings.defaultCurrency));
  }, [open, settings.defaultCurrency]);

  const currencyOptions =
    currencies.length > 0
      ? currencies
      : [
          {
            code: settings.defaultCurrency,
            name: settings.defaultCurrency,
            symbol: '$',
            exponent: 0,
          },
        ];

  const update = <K extends keyof PurchaseFormState>(
    key: K,
    value: PurchaseFormState[K],
  ) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSave = async () => {
    if (!form.name.trim()) {
      setError('Escribe el nombre de la compra.');
      return;
    }
    if (!form.totalAmount || form.totalAmount <= 0) {
      setError('Ingresa el monto total de la compra.');
      return;
    }
    if (form.installmentsCount < 1) {
      setError('La compra debe tener al menos una cuota.');
      return;
    }
    if (form.installmentsCount > 1 && !form.firstDueDate) {
      setError('Indica la fecha de vencimiento de la primera cuota.');
      return;
    }
    if (form.frequency === 'custom' && form.frequencyDays <= 0) {
      setError('Indica cada cuántos días se repite el pago.');
      return;
    }
    if (
      form.installmentAmount &&
      form.installmentAmount > 0 &&
      form.installmentAmount * form.installmentsCount > form.totalAmount
    ) {
      setError('El importe de cada cuota no puede ser mayor que el total de la compra.');
      return;
    }
    if (form.installmentsCount > 1 && form.paidImmediately) {
      setError('Una compra en cuotas no se paga al contado.');
      return;
    }

    const input: PurchaseInput = {
      name: form.name.trim(),
      contactName: form.contactName.trim() || null,
      url: form.url.trim() || null,
      purchaseDate: form.purchaseDate,
      totalAmount: form.totalAmount,
      currencyCode: form.currency,
      installmentsCount: form.installmentsCount,
      installmentAmount:
        form.installmentAmount && form.installmentAmount > 0
          ? form.installmentAmount
          : null,
      firstDueDate:
        form.installmentsCount > 1 ? form.firstDueDate : form.purchaseDate,
      frequency: form.frequency,
      frequencyDays: form.frequency === 'custom' ? form.frequencyDays : null,
      paymentMethod: form.paymentMethod || null,
      accountId: form.accountId || null,
      categoryId: form.categoryId || null,
      paidImmediately: form.installmentsCount === 1 && form.paidImmediately,
      notes: form.notes.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      await api.createPurchase(input);
      toast.success('Compra registrada.');
      await refresh();
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo registrar la compra.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      wide
      title="Nueva compra"
      subtitle="Crear una compra no registra gastos: sólo genera el calendario de cuotas."
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn" onClick={onClose} disabled={saving}>
            Cancelar
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void handleSave()}
            disabled={saving}
          >
            {saving ? 'Guardando…' : 'Registrar compra'}
          </button>
        </>
      }
    >
      {error ? (
        <div className="alert alert-error" style={{ marginBottom: 'var(--space-4)' }}>
          {error}
        </div>
      ) : null}

      <div className="form-section">
        <p className="form-section-title">Detalle</p>
        <div className="form-grid">
          <Field label="Nombre de la compra" required className="span-2">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.name}
                placeholder="Ej.: notebook Lenovo"
                onChange={(event) => update('name', event.target.value)}
              />
            )}
          </Field>

          <Field
            label="Comercio"
            hint="Si no existe, se creará automáticamente como comercio."
          >
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.contactName}
                placeholder="Ej.: Tienda Online"
                onChange={(event) => update('contactName', event.target.value)}
              />
            )}
          </Field>

          <Field label="URL del producto">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.url}
                placeholder="https://…"
                onChange={(event) => update('url', event.target.value)}
              />
            )}
          </Field>

          <Field label="Monto total" required>
            {(id) => (
              <AmountInput
                id={id}
                value={form.totalAmount}
                currency={form.currency}
                onChange={(value) => update('totalAmount', value)}
              />
            )}
          </Field>

          <Field label="Moneda">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.currency}
                onChange={(event) => update('currency', event.target.value)}
              >
                {currencyOptions.map((currency) => (
                  <option key={currency.code} value={currency.code}>
                    {currency.code} · {currency.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Fecha de compra" required>
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.purchaseDate}
                onChange={(event) => update('purchaseDate', event.target.value)}
              />
            )}
          </Field>

          <Field label="Categoría de gasto">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.categoryId}
                onChange={(event) => update('categoryId', event.target.value)}
              >
                <option value="">Sin categoría</option>
                {expenseCategories.map((category) => (
                  <option key={category.id} value={category.id}>
                    {category.name}
                  </option>
                ))}
              </select>
            )}
          </Field>
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Cuotas</p>
        <div className="form-grid">
          <Field label="Cantidad de cuotas" required>
            {(id) => (
              <input
                id={id}
                className="input"
                type="number"
                min={1}
                value={form.installmentsCount}
                onChange={(event) =>
                  update('installmentsCount', Math.max(1, Number(event.target.value) || 1))
                }
              />
            )}
          </Field>

          <Field
            label="Importe de cada cuota"
            hint="Si lo dejas vacío se reparte el total entre las cuotas."
          >
            {(id) => (
              <AmountInput
                id={id}
                value={form.installmentAmount}
                currency={form.currency}
                onChange={(value) => update('installmentAmount', value)}
              />
            )}
          </Field>

          {form.installmentsCount > 1 ? (
            <Field label="Vencimiento de la primera cuota" required>
              {(id) => (
                <input
                  id={id}
                  className="input"
                  type="date"
                  value={form.firstDueDate}
                  onChange={(event) => update('firstDueDate', event.target.value)}
                />
              )}
            </Field>
          ) : null}

          <Field label="Frecuencia">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.frequency}
                onChange={(event) =>
                  update('frequency', event.target.value as Frequency)
                }
              >
                {FREQUENCY_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            )}
          </Field>

          {form.frequency === 'custom' ? (
            <Field label="Cada cuántos días" required>
              {(id) => (
                <input
                  id={id}
                  className="input"
                  type="number"
                  min={1}
                  value={form.frequencyDays}
                  onChange={(event) =>
                    update('frequencyDays', Math.max(0, Number(event.target.value) || 0))
                  }
                />
              )}
            </Field>
          ) : null}
        </div>

        {form.installmentsCount === 1 ? (
          <div className="alert alert-info">
            <Icon name="info" size={15} />
            <span>
              Con una sola cuota no se registra ningún gasto al crear la compra.
              Marca la casilla de abajo sólo si la pagaste al contado.
            </span>
          </div>
        ) : null}

        {form.installmentsCount === 1 ? (
          <label className="checkbox">
            <input
              type="checkbox"
              checked={form.paidImmediately}
              onChange={(event) => update('paidImmediately', event.target.checked)}
            />
            Pagada al contado (registra el gasto y deja la compra completada)
          </label>
        ) : null}
      </div>

      <div className="form-section">
        <p className="form-section-title">Pago y notas</p>
        <div className="form-grid">
          <Field label="Cuenta">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.accountId}
                onChange={(event) => update('accountId', event.target.value)}
              >
                <option value="">Sin especificar</option>
                {accounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name} · {account.currencyCode}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Método de pago">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.paymentMethod}
                onChange={(event) => update('paymentMethod', event.target.value)}
              >
                <option value="">Sin especificar</option>
                {PAYMENT_METHODS.map((method) => (
                  <option key={method} value={method}>
                    {method}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Notas" className="span-2">
            {(id) => (
              <textarea
                id={id}
                className="textarea"
                value={form.notes}
                onChange={(event) => update('notes', event.target.value)}
              />
            )}
          </Field>
        </div>
      </div>
    </Modal>
  );
}
