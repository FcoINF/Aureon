import { useEffect, useMemo, useState } from 'react';

import { Icon } from '../../components/Icons';
import { AmountInput, Money } from '../../components/Money';
import { ConfirmDialog, Modal } from '../../components/Modal';
import { EmptyState, Field, PageHeader, Stat } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import type {
  Account,
  Contact,
  Currency,
  ObligationStatus,
  Receivable,
  ReceivableInput,
  ReceivableKind,
  ReceivablePayment,
} from '../../types';
import { dueLabel, formatCurrency, formatDate, statusLabel, todayIso } from '../../utils/format';

const STATUS_FILTERS: { value: string; label: string }[] = [
  { value: '', label: 'Todas' },
  { value: 'pending', label: 'Pendientes' },
  { value: 'partial', label: 'Parciales' },
  { value: 'paid', label: 'Pagadas' },
  { value: 'overdue', label: 'Vencidas' },
];

const STATUS_OPTIONS: { value: ObligationStatus; label: string }[] = [
  { value: 'pending', label: 'Pendiente' },
  { value: 'partial', label: 'Parcial' },
  { value: 'paid', label: 'Pagado' },
  { value: 'overdue', label: 'Vencido' },
  { value: 'cancelled', label: 'Cancelado' },
];

const KIND_LABELS: Record<ReceivableKind, string> = {
  debt: 'Deuda',
  advance: 'Adelanto',
  refund: 'Devolución',
};

function statusBadgeClass(status: string): string {
  switch (status) {
    case 'paid':
      return 'badge badge-income';
    case 'partial':
      return 'badge badge-info';
    case 'overdue':
      return 'badge badge-warning';
    case 'cancelled':
      return 'badge badge-expense';
    default:
      return 'badge';
  }
}

/** Suma los saldos pendientes agrupados por moneda. */
function pendingByCurrency(items: Receivable[]): Record<string, number> {
  const totals: Record<string, number> = {};
  for (const item of items) {
    if (item.status === 'paid' || item.status === 'cancelled') continue;
    totals[item.currencyCode] = (totals[item.currencyCode] ?? 0) + item.pendingAmount;
  }
  return totals;
}

interface ReceivableFormProps {
  open: boolean;
  currencies: Currency[];
  defaultCurrency: string;
  onClose: () => void;
  onSaved: () => void;
}

interface ReceivableFormState {
  title: string;
  kind: ReceivableKind;
  amount: number | null;
  currency: string;
  originDate: string;
  dueDate: string;
  contactId: string;
  contactName: string;
  notes: string;
}

function emptyFormState(currency: string): ReceivableFormState {
  return {
    title: '',
    kind: 'debt',
    amount: null,
    currency,
    originDate: todayIso(),
    dueDate: '',
    contactId: '',
    contactName: '',
    notes: '',
  };
}

/** Alta de una cuenta por cobrar. */
function ReceivableForm({
  open,
  currencies,
  defaultCurrency,
  onClose,
  onSaved,
}: ReceivableFormProps) {
  const toast = useToast();
  const [form, setForm] = useState<ReceivableFormState>(emptyFormState(defaultCurrency));
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setForm(emptyFormState(defaultCurrency));
    setError(null);
    void api.listContacts().then(setContacts).catch(() => setContacts([]));
  }, [open, defaultCurrency]);

  const update = <K extends keyof ReceivableFormState>(
    key: K,
    value: ReceivableFormState[K],
  ) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSave = async () => {
    if (!form.title.trim()) {
      setError('Escribe el motivo del cobro.');
      return;
    }
    if (!form.amount || form.amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (!form.contactId && !form.contactName.trim()) {
      setError('Indica a quién le debes cobrar.');
      return;
    }
    if (!form.originDate) {
      setError('Indica la fecha de origen.');
      return;
    }

    const input: ReceivableInput = {
      title: form.title.trim(),
      kind: form.kind,
      totalAmount: form.amount,
      currencyCode: form.currency,
      originDate: form.originDate,
      dueDate: form.dueDate || null,
      contactId: form.contactId || null,
      contactName: form.contactId ? null : form.contactName.trim(),
      notes: form.notes.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      await api.createReceivable(input);
      toast.success('Cuenta por cobrar creada.');
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo crear la cuenta por cobrar.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Nueva cuenta por cobrar"
      subtitle="Registra el dinero que te deben. No afecta tus saldos hasta que lo recibas."
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn" onClick={onClose} disabled={saving}>
            Cancelar
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? 'Guardando…' : 'Crear cuenta'}
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
          <Field label="Motivo" required className="span-2">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.title}
                placeholder="Ej.: préstamo a Marco"
                onChange={(event) => update('title', event.target.value)}
              />
            )}
          </Field>

          <Field label="Tipo">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.kind}
                onChange={(event) => update('kind', event.target.value as ReceivableKind)}
              >
                {Object.entries(KIND_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Monto total" required>
            {(id) => (
              <AmountInput
                id={id}
                value={form.amount}
                currency={form.currency}
                onChange={(value) => update('amount', value)}
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
                {currencies.map((currency) => (
                  <option key={currency.code} value={currency.code}>
                    {currency.code} · {currency.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Fecha de origen" required>
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.originDate}
                onChange={(event) => update('originDate', event.target.value)}
              />
            )}
          </Field>

          <Field label="Fecha de vencimiento" hint="Opcional">
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.dueDate}
                onChange={(event) => update('dueDate', event.target.value)}
              />
            )}
          </Field>
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Persona</p>
        <div className="form-grid">
          <Field label="Contacto registrado" className="span-2">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.contactId}
                onChange={(event) => {
                  update('contactId', event.target.value);
                  if (event.target.value) update('contactName', '');
                }}
              >
                <option value="">Escribir un nombre libre</option>
                {contacts.map((contact) => (
                  <option key={contact.id} value={contact.id}>
                    {contact.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          {!form.contactId ? (
            <Field label="Nombre de la persona" required className="span-2">
              {(id) => (
                <input
                  id={id}
                  className="input"
                  value={form.contactName}
                  placeholder="Ej.: Marco Pérez"
                  onChange={(event) => update('contactName', event.target.value)}
                />
              )}
            </Field>
          ) : null}
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Otros datos</p>
        <div className="form-grid">
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

interface ReceiveModalProps {
  receivable: Receivable | null;
  accounts: Account[];
  onClose: () => void;
  onSaved: () => void;
}

/** Registro de un cobro (total o parcial). */
function ReceiveModal({ receivable, accounts, onClose, onSaved }: ReceiveModalProps) {
  const toast = useToast();
  const [amount, setAmount] = useState<number | null>(null);
  const [date, setDate] = useState(todayIso());
  const [accountId, setAccountId] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!receivable) return;
    setAmount(receivable.pendingAmount);
    setDate(todayIso());
    setAccountId('');
    setError(null);
  }, [receivable]);

  const handleSave = async () => {
    if (!receivable) return;
    if (!amount || amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (amount > receivable.pendingAmount) {
      setError('El cobro supera el saldo pendiente.');
      return;
    }
    if (!accountId) {
      setError('Indica la cuenta donde se recibe el dinero.');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await api.receiveMoney(receivable.id, {
        amount,
        receivedAt: date,
        accountId,
      });
      toast.success('Cobro registrado.');
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo registrar el cobro.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={receivable !== null}
      title="Registrar cobro"
      subtitle={
        receivable
          ? `Saldo pendiente: ${formatCurrency(receivable.pendingAmount, receivable.currencyCode)}`
          : undefined
      }
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn" onClick={onClose} disabled={saving}>
            Cancelar
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? 'Guardando…' : 'Registrar cobro'}
          </button>
        </>
      }
    >
      {error ? (
        <div className="alert alert-error" style={{ marginBottom: 'var(--space-4)' }}>
          {error}
        </div>
      ) : null}

      <div className="alert alert-info" style={{ marginBottom: 'var(--space-4)' }}>
        <Icon name="info" size={15} />
        <span>
          El dinero se suma a la cuenta sólo cuando registras el cobro. Puedes hacer
          pagos parciales: el saldo pendiente se actualiza con cada registro.
        </span>
      </div>

      <div className="form-grid">
        <Field label="Monto recibido" required>
          {(id) => (
            <AmountInput
              id={id}
              value={amount}
              currency={receivable?.currencyCode ?? 'CLP'}
              onChange={setAmount}
            />
          )}
        </Field>

        <Field label="Fecha del cobro" required>
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

        <Field label="Cuenta donde entra el dinero" required className="span-2">
          {(id) => (
            <select
              id={id}
              className="select"
              value={accountId}
              onChange={(event) => setAccountId(event.target.value)}
            >
              <option value="">Selecciona una cuenta</option>
              {accounts.map((account) => (
                <option key={account.id} value={account.id}>
                  {account.name} · {account.currencyCode}
                </option>
              ))}
            </select>
          )}
        </Field>
      </div>
    </Modal>
  );
}

interface PaymentsModalProps {
  receivable: Receivable | null;
  onClose: () => void;
}

/** Historial de cobros de una cuenta por cobrar. */
function PaymentsModal({ receivable, onClose }: PaymentsModalProps) {
  const payments = useAsync(
    () => (receivable ? api.listReceivablePayments(receivable.id) : Promise.resolve([])),
    [receivable?.id],
  );

  return (
    <Modal
      open={receivable !== null}
      title="Historial de cobros"
      subtitle={receivable ? receivable.title : undefined}
      onClose={onClose}
    >
      {payments.loading ? (
        <div className="loading">
          <span className="spinner" />
        </div>
      ) : payments.error ? (
        <div className="alert alert-error">{payments.error}</div>
      ) : (payments.data ?? []).length === 0 ? (
        <EmptyState
          icon="receipt"
          title="Sin cobros registrados"
          text="Cuando registres un cobro aparecerá aquí con su fecha y la cuenta donde entró el dinero."
        />
      ) : (
        <div className="detail-list">
          {(payments.data ?? []).map((payment: ReceivablePayment) => (
            <div className="detail-row" key={payment.id}>
              <span className="detail-key">
                {formatDate(payment.receivedAt)}
                <span className="text-soft"> · {payment.accountName ?? 'Sin cuenta'}</span>
              </span>
              <span className="detail-value">
                <Money
                  value={payment.amount}
                  currency={receivable?.currencyCode ?? 'CLP'}
                  tone="income"
                />
              </span>
            </div>
          ))}
        </div>
      )}
    </Modal>
  );
}

interface StatusModalProps {
  receivable: Receivable | null;
  onClose: () => void;
  onSaved: () => void;
}

/** Cambio manual del estado de una cuenta por cobrar. */
function StatusModal({ receivable, onClose, onSaved }: StatusModalProps) {
  const toast = useToast();
  const [status, setStatus] = useState<ObligationStatus>('pending');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!receivable) return;
    setStatus(receivable.status);
    setError(null);
  }, [receivable]);

  const handleSave = async () => {
    if (!receivable) return;
    setSaving(true);
    setError(null);
    try {
      await api.setReceivableStatus(receivable.id, status);
      toast.success('Estado actualizado.');
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo cambiar el estado.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={receivable !== null}
      title="Cambiar estado"
      subtitle={receivable ? receivable.title : undefined}
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn" onClick={onClose} disabled={saving}>
            Cancelar
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? 'Guardando…' : 'Guardar estado'}
          </button>
        </>
      }
    >
      {error ? (
        <div className="alert alert-error" style={{ marginBottom: 'var(--space-4)' }}>
          {error}
        </div>
      ) : null}

      <div className="alert alert-info" style={{ marginBottom: 'var(--space-4)' }}>
        <Icon name="info" size={15} />
        <span>
          Marcar como pagado no registra el cobro: el dinero sigue sin estar en tus cuentas.
          Usa «Registrar cobro» para que el ingreso se sume al saldo.
        </span>
      </div>

      <Field label="Estado" required>
        {(id) => (
          <select
            id={id}
            className="select"
            value={status}
            onChange={(event) => setStatus(event.target.value as ObligationStatus)}
          >
            {STATUS_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        )}
      </Field>
    </Modal>
  );
}

export function ReceivablesPage() {
  const { settings, currencies, accounts, refresh } = useAppData();
  const toast = useToast();

  const [statusFilter, setStatusFilter] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [receiving, setReceiving] = useState<Receivable | null>(null);
  const [history, setHistory] = useState<Receivable | null>(null);
  const [statusTarget, setStatusTarget] = useState<Receivable | null>(null);
  const [deleting, setDeleting] = useState<Receivable | null>(null);
  const [busy, setBusy] = useState(false);

  const list = useAsync(() => api.listReceivables(statusFilter || undefined), [statusFilter]);
  const all = useAsync(() => api.listReceivables(), []);

  useEffect(() => {
    const sync = async () => {
      try {
        await api.refreshOverdueStatus();
      } catch {
        // Si no se pueden actualizar los vencimientos, se muestran los estados guardados.
      }
      list.reload();
      all.reload();
    };
    void sync();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const currencyOptions = useMemo<Currency[]>(
    () =>
      currencies.length > 0
        ? currencies
        : [
            {
              code: settings.defaultCurrency,
              name: settings.defaultCurrency,
              symbol: '$',
              exponent: 0,
            },
          ],
    [currencies, settings.defaultCurrency],
  );

  const totals = useMemo(() => pendingByCurrency(all.data ?? []), [all.data]);
  const mainCurrency = settings.defaultCurrency;
  const mainTotal = totals[mainCurrency] ?? 0;
  const otherTotals = Object.entries(totals).filter(([code]) => code !== mainCurrency);
  const openCount = (all.data ?? []).filter(
    (item) => item.status !== 'paid' && item.status !== 'cancelled',
  ).length;
  const overdueCount = (all.data ?? []).filter((item) => item.status === 'overdue').length;

  const reloadAll = () => {
    list.reload();
    all.reload();
    void refresh();
  };

  const handleDelete = async () => {
    if (!deleting) return;
    setBusy(true);
    try {
      await api.deleteReceivable(deleting.id);
      toast.success('Cuenta por cobrar eliminada.');
      setDeleting(null);
      reloadAll();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo eliminar la cuenta.');
    } finally {
      setBusy(false);
    }
  };

  const items = list.data ?? [];

  return (
    <>
      <PageHeader
        title="Cuentas por cobrar"
        subtitle="Dinero que otras personas te deben y que todavía no has recibido."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setFormOpen(true)}
          >
            <Icon name="plus" size={14} />
            Nueva cuenta por cobrar
          </button>
        }
      />

      <div className="stat-grid">
        <Stat
          label="Total pendiente de cobro"
          value={formatCurrency(mainTotal, mainCurrency)}
          meta={
            otherTotals.length > 0
              ? `Además ${otherTotals
                  .map(([code, value]) => `${formatCurrency(value, code)} en ${code}`)
                  .join(' · ')}`
              : `${openCount} cuenta${openCount === 1 ? '' : 's'} abierta${
                  openCount === 1 ? '' : 's'
                }`
          }
          tone="accent"
        />
        <Stat
          label="Cuentas vencidas"
          value={String(overdueCount)}
          meta={
            overdueCount > 0
              ? 'Cuentas con el vencimiento pasado y saldo pendiente'
              : 'No hay cuentas vencidas'
          }
          tone={overdueCount > 0 ? 'warning' : 'default'}
        />
        <Stat
          label="Cuentas registradas"
          value={String((all.data ?? []).length)}
          meta="Incluye las pagadas y canceladas"
        />
      </div>

      <div className="alert alert-info">
        <Icon name="info" size={15} />
        <span>
          El dinero por cobrar no es dinero disponible: no forma parte de tus saldos hasta
          que registras el cobro. Al hacerlo, el importe se suma a la cuenta que elijas.
        </span>
      </div>

      <section className="card">
        <div className="filter-bar">
          <div className="field">
            <span className="field-label">Estado</span>
            <div className="segmented" role="group" aria-label="Filtrar por estado">
              {STATUS_FILTERS.map((option) => (
                <button
                  key={option.value || 'all'}
                  type="button"
                  aria-pressed={statusFilter === option.value}
                  onClick={() => setStatusFilter(option.value)}
                >
                  {option.label}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="card-body">
          {list.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : list.error ? (
            <div className="alert alert-error">{list.error}</div>
          ) : items.length === 0 ? (
            <EmptyState
              icon="incoming"
              title="Sin cuentas por cobrar"
              text="Registra el dinero que te deben para seguirlo y recordarlo."
              action={
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => setFormOpen(true)}
                >
                  <Icon name="plus" size={14} />
                  Nueva cuenta por cobrar
                </button>
              }
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Motivo</th>
                    <th>Persona</th>
                    <th>Origen</th>
                    <th>Vencimiento</th>
                    <th className="num">Total</th>
                    <th className="num">Recibido</th>
                    <th className="num">Saldo</th>
                    <th>Estado</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {items.map((item) => {
                    const progress =
                      item.totalAmount > 0
                        ? Math.min(100, Math.round((item.receivedAmount / item.totalAmount) * 100))
                        : 0;
                    return (
                      <tr key={item.id}>
                        <td>
                          <div className="truncate">{item.title}</div>
                          <span className="text-sm text-soft">
                            {KIND_LABELS[item.kind] ?? item.kind}
                          </span>
                        </td>
                        <td className="text-sm">{item.contactName}</td>
                        <td className="text-sm text-muted">{formatDate(item.originDate)}</td>
                        <td className="text-sm">
                          {formatDate(item.dueDate)}
                          {item.dueDate ? (
                            <div className="text-sm text-soft">{dueLabel(item.dueDate)}</div>
                          ) : null}
                        </td>
                        <td className="num">
                          <Money value={item.totalAmount} currency={item.currencyCode} />
                        </td>
                        <td className="num">
                          <Money
                            value={item.receivedAmount}
                            currency={item.currencyCode}
                            tone="income"
                          />
                        </td>
                        <td className="num">
                          <Money value={item.pendingAmount} currency={item.currencyCode} />
                          <div className="progress" style={{ marginTop: 4 }}>
                            <div
                              className={`progress-bar${
                                item.status === 'overdue' ? ' is-over' : ''
                              }`}
                              style={{ width: `${progress}%` }}
                            />
                          </div>
                        </td>
                        <td>
                          <span className={statusBadgeClass(item.status)}>
                            {statusLabel(item.status)}
                          </span>
                        </td>
                        <td>
                          <div className="table-actions">
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Registrar cobro"
                              aria-label={`Registrar cobro de ${item.title}`}
                              disabled={item.pendingAmount <= 0 || item.status === 'cancelled'}
                              onClick={() => setReceiving(item)}
                            >
                              <Icon name="incoming" size={14} />
                            </button>
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Historial de cobros"
                              aria-label={`Historial de cobros de ${item.title}`}
                              onClick={() => setHistory(item)}
                            >
                              <Icon name="receipt" size={14} />
                            </button>
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Cambiar estado"
                              aria-label={`Cambiar estado de ${item.title}`}
                              onClick={() => setStatusTarget(item)}
                            >
                              <Icon name="edit" size={14} />
                            </button>
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Eliminar"
                              aria-label={`Eliminar ${item.title}`}
                              onClick={() => setDeleting(item)}
                            >
                              <Icon name="trash" size={14} />
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

      <ReceivableForm
        open={formOpen}
        currencies={currencyOptions}
        defaultCurrency={settings.defaultCurrency}
        onClose={() => setFormOpen(false)}
        onSaved={reloadAll}
      />

      <ReceiveModal
        receivable={receiving}
        accounts={accounts}
        onClose={() => setReceiving(null)}
        onSaved={reloadAll}
      />

      <PaymentsModal receivable={history} onClose={() => setHistory(null)} />

      <StatusModal
        receivable={statusTarget}
        onClose={() => setStatusTarget(null)}
        onSaved={reloadAll}
      />

      <ConfirmDialog
        open={deleting !== null}
        title="Eliminar cuenta por cobrar"
        message={
          deleting
            ? `Se eliminará «${deleting.title}» y su historial de cobros. Esta acción no se puede deshacer.`
            : ''
        }
        confirmLabel="Eliminar"
        danger
        busy={busy}
        onConfirm={handleDelete}
        onCancel={() => setDeleting(null)}
      />
    </>
  );
}
