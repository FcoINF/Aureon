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
  Payable,
  PayableInput,
  PayablePayment,
} from '../../types';
import { dueLabel, formatCurrency, formatDate, statusLabel, todayIso } from '../../utils/format';

const STATUS_FILTERS: { value: string; label: string }[] = [
  { value: '', label: 'Todas' },
  { value: 'pending', label: 'Pendientes' },
  { value: 'partial', label: 'Parciales' },
  { value: 'paid', label: 'Pagadas' },
  { value: 'overdue', label: 'Vencidas' },
  { value: 'cancelled', label: 'Canceladas' },
];

const STATUS_OPTIONS: { value: ObligationStatus; label: string }[] = [
  { value: 'pending', label: 'Pendiente' },
  { value: 'partial', label: 'Parcial' },
  { value: 'paid', label: 'Pagado' },
  { value: 'overdue', label: 'Vencido' },
  { value: 'cancelled', label: 'Cancelado' },
];

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

/**
 * Aviso mostrado cuando no existe ninguna cuenta en la moneda del registro:
 * el backend rechaza mover dinero entre cuentas de distinta moneda.
 */
function NoAccountsNotice({ id, currency }: { id: string; currency: string }) {
  return (
    <p id={id} className="field-hint">
      No hay cuentas en {currency}. Crea una en Configuración → Cuentas.
    </p>
  );
}

/** Suma los saldos pendientes agrupados por moneda. */
function pendingByCurrency(items: Payable[]): Record<string, number> {
  const totals: Record<string, number> = {};
  for (const item of items) {
    if (item.status === 'paid' || item.status === 'cancelled') continue;
    totals[item.currencyCode] = (totals[item.currencyCode] ?? 0) + item.pendingAmount;
  }
  return totals;
}

interface PayableFormProps {
  open: boolean;
  currencies: Currency[];
  defaultCurrency: string;
  accounts: Account[];
  onClose: () => void;
  onSaved: () => void;
}

interface PayableFormState {
  title: string;
  amount: number | null;
  currency: string;
  createdDate: string;
  dueDate: string;
  accountId: string;
  contactId: string;
  contactName: string;
  notes: string;
}

function emptyFormState(currency: string): PayableFormState {
  return {
    title: '',
    amount: null,
    currency,
    createdDate: todayIso(),
    dueDate: '',
    accountId: '',
    contactId: '',
    contactName: '',
    notes: '',
  };
}

/** Alta de una cuenta por pagar. */
function PayableForm({
  open,
  currencies,
  defaultCurrency,
  accounts,
  onClose,
  onSaved,
}: PayableFormProps) {
  const toast = useToast();
  const [form, setForm] = useState<PayableFormState>(emptyFormState(defaultCurrency));
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // La cuenta prevista debe tener la misma moneda que la obligación.
  const currencyAccounts = useMemo(
    () => accounts.filter((account) => account.currencyCode === form.currency),
    [accounts, form.currency],
  );

  useEffect(() => {
    if (!open) return;
    setForm(emptyFormState(defaultCurrency));
    setError(null);
    void api.listContacts().then(setContacts).catch(() => setContacts([]));
  }, [open, defaultCurrency]);

  const update = <K extends keyof PayableFormState>(key: K, value: PayableFormState[K]) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSave = async () => {
    if (!form.title.trim()) {
      setError('Escribe la descripción de la obligación.');
      return;
    }
    if (!form.amount || form.amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (!form.contactId && !form.contactName.trim()) {
      setError('Indica a quién debes pagar.');
      return;
    }
    if (!form.createdDate) {
      setError('Indica la fecha de creación.');
      return;
    }

    const input: PayableInput = {
      title: form.title.trim(),
      totalAmount: form.amount,
      currencyCode: form.currency,
      createdDate: form.createdDate,
      dueDate: form.dueDate || null,
      accountId: form.accountId || null,
      contactId: form.contactId || null,
      contactName: form.contactId ? null : form.contactName.trim(),
      notes: form.notes.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      await api.createPayable(input);
      toast.success('Cuenta por pagar creada.');
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo crear la cuenta por pagar.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Nueva cuenta por pagar"
      subtitle="Registra una obligación de pago. No descuenta dinero de tus cuentas hasta que la pagas."
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
          <Field label="Descripción" required className="span-2">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.title}
                placeholder="Ej.: mensualidad del arriendo"
                onChange={(event) => update('title', event.target.value)}
              />
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
                onChange={(event) => {
                  update('currency', event.target.value);
                  update('accountId', '');
                }}
              >
                {currencies.map((currency) => (
                  <option key={currency.code} value={currency.code}>
                    {currency.code} · {currency.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Fecha de creación" required>
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.createdDate}
                onChange={(event) => update('createdDate', event.target.value)}
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

          <Field label="Cuenta prevista" hint="De dónde saldrá el dinero al pagar">
            {(id) =>
              currencyAccounts.length === 0 ? (
                <NoAccountsNotice id={id} currency={form.currency} />
              ) : (
                <select
                  id={id}
                  className="select"
                  value={form.accountId}
                  onChange={(event) => update('accountId', event.target.value)}
                >
                  <option value="">Sin definir</option>
                  {currencyAccounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {account.name} · {account.currencyCode}
                    </option>
                  ))}
                </select>
              )
            }
          </Field>
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Acreedor</p>
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
            <Field label="Nombre del acreedor" required className="span-2">
              {(id) => (
                <input
                  id={id}
                  className="input"
                  value={form.contactName}
                  placeholder="Ej.: Banco del Estado"
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

interface PayModalProps {
  payable: Payable | null;
  accounts: Account[];
  onClose: () => void;
  onSaved: () => void;
}

/** Registro de un pago (total o parcial). */
function PayModal({ payable, accounts, onClose, onSaved }: PayModalProps) {
  const toast = useToast();
  const [amount, setAmount] = useState<number | null>(null);
  const [date, setDate] = useState(todayIso());
  const [accountId, setAccountId] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const currency = payable?.currencyCode ?? '';
  // El backend sólo admite cuentas de la misma moneda del pago.
  const currencyAccounts = useMemo(
    () => accounts.filter((account) => account.currencyCode === currency),
    [accounts, currency],
  );

  useEffect(() => {
    if (!payable) return;
    setAmount(payable.pendingAmount);
    setDate(todayIso());
    const planned = accounts.find(
      (account) => account.id === payable.accountId && account.currencyCode === payable.currencyCode,
    );
    setAccountId(planned ? planned.id : '');
    setError(null);
  }, [payable, accounts]);

  const handleSave = async () => {
    if (!payable) return;
    if (!amount || amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (amount > payable.pendingAmount) {
      setError('El pago supera el saldo pendiente.');
      return;
    }
    if (currencyAccounts.length === 0) {
      setError(`No hay cuentas en ${currency}. Crea una en Configuración → Cuentas.`);
      return;
    }
    if (!accountId) {
      setError('Indica la cuenta desde donde se paga.');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await api.payObligation(payable.id, {
        amount,
        paidAt: date,
        accountId,
      });
      toast.success('Pago registrado.');
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo registrar el pago.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={payable !== null}
      title="Registrar pago"
      subtitle={
        payable
          ? `Saldo pendiente: ${formatCurrency(payable.pendingAmount, payable.currencyCode)}`
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
            {saving ? 'Guardando…' : 'Registrar pago'}
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
          El dinero se descuenta de la cuenta sólo cuando registras el pago. Puedes pagar
          en partes: el saldo pendiente se actualiza con cada registro.
        </span>
      </div>

      <div className="form-grid">
        <Field label="Monto pagado" required>
          {(id) => (
            <AmountInput
              id={id}
              value={amount}
              currency={payable?.currencyCode ?? 'CLP'}
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

        <Field label="Cuenta desde donde se paga" required className="span-2">
          {(id) =>
            currencyAccounts.length === 0 ? (
              <NoAccountsNotice id={id} currency={currency} />
            ) : (
              <select
                id={id}
                className="select"
                value={accountId}
                onChange={(event) => setAccountId(event.target.value)}
              >
                <option value="">Selecciona una cuenta</option>
                {currencyAccounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name} · {account.currencyCode}
                  </option>
                ))}
              </select>
            )
          }
        </Field>
      </div>
    </Modal>
  );
}

interface PaymentsModalProps {
  payable: Payable | null;
  onClose: () => void;
}

/** Historial de pagos de una cuenta por pagar. */
function PaymentsModal({ payable, onClose }: PaymentsModalProps) {
  const payments = useAsync(
    () => (payable ? api.listPayablePayments(payable.id) : Promise.resolve([])),
    [payable?.id],
  );

  return (
    <Modal
      open={payable !== null}
      title="Historial de pagos"
      subtitle={payable ? payable.title : undefined}
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
          title="Sin pagos registrados"
          text="Cuando registres un pago aparecerá aquí con su fecha y la cuenta desde donde salió el dinero."
        />
      ) : (
        <div className="detail-list">
          {(payments.data ?? []).map((payment: PayablePayment) => (
            <div className="detail-row" key={payment.id}>
              <span className="detail-key">
                {formatDate(payment.paidAt)}
                <span className="text-soft"> · {payment.accountName ?? 'Sin cuenta'}</span>
              </span>
              <span className="detail-value">
                <Money
                  value={payment.amount}
                  currency={payable?.currencyCode ?? 'CLP'}
                  tone="expense"
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
  payable: Payable | null;
  onClose: () => void;
  onSaved: () => void;
}

/** Cambio manual del estado de una cuenta por pagar. */
function StatusModal({ payable, onClose, onSaved }: StatusModalProps) {
  const toast = useToast();
  const [status, setStatus] = useState<ObligationStatus>('pending');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!payable) return;
    setStatus(payable.status);
    setError(null);
  }, [payable]);

  const handleSave = async () => {
    if (!payable) return;
    setSaving(true);
    setError(null);
    try {
      await api.setPayableStatus(payable.id, status);
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
      open={payable !== null}
      title="Cambiar estado"
      subtitle={payable ? payable.title : undefined}
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
          Marcar como pagado no descuenta dinero de tus cuentas. Usa «Registrar pago» para
          que el gasto se descuente realmente del saldo.
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

export function PayablesPage() {
  const { settings, currencies, accounts, refresh } = useAppData();
  const toast = useToast();

  const [statusFilter, setStatusFilter] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [paying, setPaying] = useState<Payable | null>(null);
  const [history, setHistory] = useState<Payable | null>(null);
  const [statusTarget, setStatusTarget] = useState<Payable | null>(null);
  const [deleting, setDeleting] = useState<Payable | null>(null);
  const [busy, setBusy] = useState(false);

  const list = useAsync(() => api.listPayables(statusFilter || undefined), [statusFilter]);
  const all = useAsync(() => api.listPayables(), []);

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
    void (async () => {
      try {
        await api.refreshOverdueStatus();
      } catch {
        // Si no se pueden actualizar los vencimientos, se muestran los estados guardados.
      }
      list.reload();
      all.reload();
      void refresh();
    })();
  };

  const handleDelete = async () => {
    if (!deleting) return;
    setBusy(true);
    try {
      await api.deletePayable(deleting.id);
      toast.success('Cuenta por pagar eliminada.');
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
        title="Cuentas por pagar"
        subtitle="Obligaciones de dinero que tú debes pagar."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setFormOpen(true)}
          >
            <Icon name="plus" size={14} />
            Nueva cuenta por pagar
          </button>
        }
      />

      <div className="stat-grid">
        <Stat
          label="Total pendiente de pago"
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
          tone="expense"
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
          Crear una cuenta por pagar no descuenta dinero de tus cuentas. El saldo sólo baja
          cuando registras el pago.
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
              icon="outgoing"
              title="Sin cuentas por pagar"
              text="Registra tus obligaciones para saber qué y cuándo debes pagar."
              action={
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => setFormOpen(true)}
                >
                  <Icon name="plus" size={14} />
                  Nueva cuenta por pagar
                </button>
              }
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Descripción</th>
                    <th>Acreedor</th>
                    <th>Creación</th>
                    <th>Vencimiento</th>
                    <th className="num">Total</th>
                    <th className="num">Pagado</th>
                    <th className="num">Saldo</th>
                    <th>Estado</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {items.map((item) => {
                    const progress =
                      item.totalAmount > 0
                        ? Math.min(100, Math.round((item.paidAmount / item.totalAmount) * 100))
                        : 0;
                    return (
                      <tr key={item.id}>
                        <td>
                          <div className="truncate">{item.title}</div>
                          {item.accountName ? (
                            <span className="text-sm text-soft">{item.accountName}</span>
                          ) : null}
                        </td>
                        <td className="text-sm">{item.contactName}</td>
                        <td className="text-sm text-muted">{formatDate(item.createdDate)}</td>
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
                          <Money value={item.paidAmount} currency={item.currencyCode} tone="expense" />
                        </td>
                        <td className="num">
                          <Money value={item.pendingAmount} currency={item.currencyCode} />
                          <div
                            className="progress"
                            style={{ marginTop: 4 }}
                            role="progressbar"
                            aria-valuenow={progress}
                            aria-valuemin={0}
                            aria-valuemax={100}
                            aria-label={`Pagado: ${progress}% de ${formatCurrency(
                              item.totalAmount,
                              item.currencyCode,
                            )}`}
                          >
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
                              title="Registrar pago"
                              aria-label={`Registrar pago de ${item.title}`}
                              disabled={item.pendingAmount <= 0 || item.status === 'cancelled'}
                              onClick={() => setPaying(item)}
                            >
                              <Icon name="outgoing" size={14} />
                            </button>
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Historial de pagos"
                              aria-label={`Historial de pagos de ${item.title}`}
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

      <PayableForm
        open={formOpen}
        currencies={currencyOptions}
        defaultCurrency={settings.defaultCurrency}
        accounts={accounts}
        onClose={() => setFormOpen(false)}
        onSaved={reloadAll}
      />

      <PayModal
        payable={paying}
        accounts={accounts}
        onClose={() => setPaying(null)}
        onSaved={reloadAll}
      />

      <PaymentsModal payable={history} onClose={() => setHistory(null)} />

      <StatusModal
        payable={statusTarget}
        onClose={() => setStatusTarget(null)}
        onSaved={reloadAll}
      />

      <ConfirmDialog
        open={deleting !== null}
        title="Eliminar cuenta por pagar"
        message={
          deleting
            ? `Se eliminará «${deleting.title}» y su historial de pagos. El dinero ya pagado no se revierte: permanece descontado de la cuenta desde donde salió y en Movimientos. Esta acción no se puede deshacer.`
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
