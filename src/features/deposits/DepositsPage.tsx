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
  Category,
  Contact,
  Currency,
  Deposit,
  DepositCompleteInput,
  DepositInput,
} from '../../types';
import {
  PAYMENT_METHODS,
  dueLabel,
  formatCurrency,
  formatDate,
  statusLabel,
  todayIso,
} from '../../utils/format';

const STATUS_FILTERS: { value: string; label: string }[] = [
  { value: '', label: 'Todos' },
  { value: 'pending', label: 'Pendientes' },
  { value: 'scheduled', label: 'Programados' },
  { value: 'overdue', label: 'Vencidos' },
  { value: 'completed', label: 'Completados' },
  { value: 'cancelled', label: 'Cancelados' },
];

function statusBadgeClass(status: string): string {
  switch (status) {
    case 'completed':
      return 'badge badge-income';
    case 'scheduled':
      return 'badge badge-info';
    case 'overdue':
      return 'badge badge-warning';
    case 'cancelled':
      return 'badge badge-expense';
    default:
      return 'badge';
  }
}

interface DepositFormProps {
  open: boolean;
  currencies: Currency[];
  defaultCurrency: string;
  accounts: Account[];
  onClose: () => void;
  onSaved: () => void;
}

interface DepositFormState {
  title: string;
  amount: number | null;
  currency: string;
  createdDate: string;
  dueDate: string;
  accountId: string;
  contactId: string;
  contactName: string;
  destinationInstitution: string;
  destinationDetails: string;
  paymentMethod: string;
  notes: string;
}

function emptyFormState(currency: string): DepositFormState {
  return {
    title: '',
    amount: null,
    currency,
    createdDate: todayIso(),
    dueDate: '',
    accountId: '',
    contactId: '',
    contactName: '',
    destinationInstitution: '',
    destinationDetails: '',
    paymentMethod: '',
    notes: '',
  };
}

/** Alta de un depósito pendiente. */
function DepositForm({
  open,
  currencies,
  defaultCurrency,
  accounts,
  onClose,
  onSaved,
}: DepositFormProps) {
  const toast = useToast();
  const [form, setForm] = useState<DepositFormState>(emptyFormState(defaultCurrency));
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setForm(emptyFormState(defaultCurrency));
    setError(null);
    void api.listContacts().then(setContacts).catch(() => setContacts([]));
  }, [open, defaultCurrency]);

  const update = <K extends keyof DepositFormState>(key: K, value: DepositFormState[K]) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSave = async () => {
    if (!form.title.trim()) {
      setError('Escribe el motivo del depósito.');
      return;
    }
    if (!form.amount || form.amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (!form.createdDate) {
      setError('Indica la fecha de creación.');
      return;
    }

    const input: DepositInput = {
      title: form.title.trim(),
      amount: form.amount,
      currencyCode: form.currency,
      createdDate: form.createdDate,
      dueDate: form.dueDate || null,
      accountId: form.accountId || null,
      contactId: form.contactId || null,
      contactName: form.contactId ? null : form.contactName.trim() || null,
      destinationInstitution: form.destinationInstitution.trim() || null,
      destinationDetails: form.destinationDetails.trim() || null,
      paymentMethod: form.paymentMethod || null,
      notes: form.notes.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      await api.createDeposit(input);
      toast.success('Depósito creado.');
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo crear el depósito.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Nuevo depósito pendiente"
      subtitle="Anota una transferencia o depósito que aún no realizas. No altera tus saldos."
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
            {saving ? 'Guardando…' : 'Crear depósito'}
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
                placeholder="Ej.: transferencia al fondo de inversión"
                onChange={(event) => update('title', event.target.value)}
              />
            )}
          </Field>

          <Field label="Monto" required>
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

          <Field label="Fecha límite" hint="Opcional">
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

          <Field label="Cuenta de origen" hint="De dónde saldrá el dinero">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.accountId}
                onChange={(event) => update('accountId', event.target.value)}
              >
                <option value="">Sin definir</option>
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
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Destinatario</p>
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
            <Field label="Nombre del destinatario" className="span-2">
              {(id) => (
                <input
                  id={id}
                  className="input"
                  value={form.contactName}
                  placeholder="Ej.: inversionista particular"
                  onChange={(event) => update('contactName', event.target.value)}
                />
              )}
            </Field>
          ) : null}

          <Field label="Banco o institución de destino">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.destinationInstitution}
                placeholder="Ej.: BancoEstado"
                onChange={(event) => update('destinationInstitution', event.target.value)}
              />
            )}
          </Field>

          <Field label="Datos de destino" hint="Cuenta, alias o número de operación">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.destinationDetails}
                placeholder="Ej.: 123456789"
                onChange={(event) => update('destinationDetails', event.target.value)}
              />
            )}
          </Field>
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

type CompleteMode = 'none' | 'transfer' | 'expense';

interface CompleteModalProps {
  deposit: Deposit | null;
  accounts: Account[];
  expenseCategories: Category[];
  onClose: () => void;
  onSaved: () => void;
}

/**
 * Al completar un depósito se pregunta si el movimiento real debe quedar
 * registrado en la contabilidad de la app.
 */
function CompleteModal({
  deposit,
  accounts,
  expenseCategories,
  onClose,
  onSaved,
}: CompleteModalProps) {
  const toast = useToast();
  const [mode, setMode] = useState<CompleteMode>('none');
  const [date, setDate] = useState(todayIso());
  const [accountId, setAccountId] = useState('');
  const [transferAccountId, setTransferAccountId] = useState('');
  const [categoryId, setCategoryId] = useState('');
  const [contactId, setContactId] = useState('');
  const [paymentMethod, setPaymentMethod] = useState('');
  const [notes, setNotes] = useState('');
  const [contacts, setContacts] = useState<Contact[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!deposit) return;
    setMode('none');
    setDate(todayIso());
    setAccountId(deposit.accountId ?? '');
    setTransferAccountId('');
    setCategoryId('');
    setContactId(deposit.contactId ?? '');
    setPaymentMethod(deposit.paymentMethod ?? '');
    setNotes('');
    setError(null);
    void api.listContacts().then(setContacts).catch(() => setContacts([]));
  }, [deposit]);

  const handleSave = async () => {
    if (!deposit) return;
    if (mode !== 'none' && !accountId) {
      setError('Indica la cuenta de origen del dinero.');
      return;
    }
    if (mode === 'transfer') {
      if (!transferAccountId) {
        setError('Indica la cuenta de destino de la transferencia.');
        return;
      }
      if (transferAccountId === accountId) {
        setError('El origen y el destino de la transferencia deben ser distintas.');
        return;
      }
    }
    if (mode === 'expense') {
      if (!categoryId) {
        setError('Elige la categoría de gasto del movimiento.');
        return;
      }
      if (!contactId) {
        setError('Elige el contacto al que se le paga.');
        return;
      }
    }

    const input: DepositCompleteInput = {
      completedAt: date,
      accountId: accountId || null,
      registerTransaction: mode !== 'none',
      transactionType: mode === 'none' ? null : mode,
      transferAccountId: mode === 'transfer' ? transferAccountId : null,
      categoryId: mode === 'expense' ? categoryId : null,
      contactId: mode === 'expense' ? contactId : null,
      paymentMethod: paymentMethod || null,
      notes: notes.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      await api.completeDeposit(deposit.id, input);
      toast.success('Depósito completado.');
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo completar el depósito.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={deposit !== null}
      title="Completar depósito"
      subtitle={
        deposit
          ? `${deposit.title} · ${formatCurrency(deposit.amount, deposit.currencyCode)}`
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
            {saving ? 'Guardando…' : 'Completar depósito'}
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
        <p className="form-section-title">¿Registro el movimiento real?</p>
        <div className="segmented" role="group" aria-label="Registro del movimiento">
          <button
            type="button"
            aria-pressed={mode === 'none'}
            onClick={() => setMode('none')}
          >
            Sin movimiento
          </button>
          <button
            type="button"
            aria-pressed={mode === 'transfer'}
            onClick={() => setMode('transfer')}
          >
            Transferencia
          </button>
          <button
            type="button"
            aria-pressed={mode === 'expense'}
            onClick={() => setMode('expense')}
          >
            Pago a un tercero
          </button>
        </div>
      </div>

      <div className="alert alert-info" style={{ margin: 'var(--space-4) 0' }}>
        <Icon name="info" size={15} />
        <span>
          {mode === 'none'
            ? 'El depósito se marcará como completado sin tocar tus saldos. Podrás registrar el movimiento más tarde desde Movimientos.'
            : mode === 'transfer'
              ? 'Se registrará una transferencia entre tus cuentas: sale dinero de la cuenta de origen y entra en la de destino.'
              : 'Se registrará un gasto a nombre del destinatario, descontando el monto de la cuenta de origen.'}
          {deposit?.contactId
            ? ' Además, como el depósito tiene destinatario, Aureon registrará una cuenta por cobrar para que no pierdas el seguimiento de ese dinero.'
            : ''}
        </span>
      </div>

      <div className="form-grid">
        <Field label="Fecha de envío" required>
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
          label="Cuenta de origen"
          required={mode !== 'none'}
          hint={mode === 'none' ? 'Se usará la cuenta del depósito' : undefined}
        >
          {(id) => (
            <select
              id={id}
              className="select"
              value={accountId}
              onChange={(event) => setAccountId(event.target.value)}
            >
              <option value="">
                {mode === 'none' ? 'La del depósito' : 'Selecciona una cuenta'}
              </option>
              {accounts.map((account) => (
                <option key={account.id} value={account.id}>
                  {account.name} · {account.currencyCode}
                </option>
              ))}
            </select>
          )}
        </Field>

        {mode === 'transfer' ? (
          <Field label="Cuenta de destino" required className="span-2">
            {(id) => (
              <select
                id={id}
                className="select"
                value={transferAccountId}
                onChange={(event) => setTransferAccountId(event.target.value)}
              >
                <option value="">Selecciona una cuenta</option>
                {accounts
                  .filter((account) => account.id !== accountId)
                  .map((account) => (
                    <option key={account.id} value={account.id}>
                      {account.name} · {account.currencyCode}
                    </option>
                  ))}
              </select>
            )}
          </Field>
        ) : null}

        {mode === 'expense' ? (
          <>
            <Field label="Categoría de gasto" required>
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={categoryId}
                  onChange={(event) => setCategoryId(event.target.value)}
                >
                  <option value="">Selecciona una categoría</option>
                  {expenseCategories.map((category) => (
                    <option key={category.id} value={category.id}>
                      {category.name}
                    </option>
                  ))}
                </select>
              )}
            </Field>

            <Field label="Contacto" required>
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={contactId}
                  onChange={(event) => setContactId(event.target.value)}
                >
                  <option value="">Selecciona un contacto</option>
                  {contacts.map((contact) => (
                    <option key={contact.id} value={contact.id}>
                      {contact.name}
                    </option>
                  ))}
                </select>
              )}
            </Field>
          </>
        ) : null}

        <Field label="Método de pago">
          {(id) => (
            <select
              id={id}
              className="select"
              value={paymentMethod}
              onChange={(event) => setPaymentMethod(event.target.value)}
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

        <Field label="Notas">
          {(id) => (
            <input
              id={id}
              className="input"
              value={notes}
              onChange={(event) => setNotes(event.target.value)}
            />
          )}
        </Field>
      </div>
    </Modal>
  );
}

export function DepositsPage() {
  const { settings, currencies, accounts, expenseCategories, refresh } = useAppData();
  const toast = useToast();

  const [statusFilter, setStatusFilter] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [completing, setCompleting] = useState<Deposit | null>(null);
  const [cancelling, setCancelling] = useState<Deposit | null>(null);
  const [deleting, setDeleting] = useState<Deposit | null>(null);
  const [busy, setBusy] = useState(false);

  const list = useAsync(() => api.listDeposits(statusFilter || undefined), [statusFilter]);
  const all = useAsync(() => api.listDeposits(), []);

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

  const items = all.data ?? [];
  const pendingCount = items.filter(
    (item) => item.status === 'pending' || item.status === 'scheduled',
  ).length;
  const overdueCount = items.filter((item) => item.status === 'overdue').length;
  const completedCount = items.filter((item) => item.status === 'completed').length;
  const cancelledCount = items.filter((item) => item.status === 'cancelled').length;

  const pendingTotals = useMemo(() => {
    const totals: Record<string, number> = {};
    for (const item of items) {
      if (item.status !== 'pending' && item.status !== 'scheduled') continue;
      totals[item.currencyCode] = (totals[item.currencyCode] ?? 0) + item.amount;
    }
    return totals;
  }, [items]);

  const mainCurrency = settings.defaultCurrency;
  const mainTotal = pendingTotals[mainCurrency] ?? 0;
  const otherTotals = Object.entries(pendingTotals).filter(([code]) => code !== mainCurrency);

  const reloadAll = () => {
    list.reload();
    all.reload();
    void refresh();
  };

  const handleCancel = async () => {
    if (!cancelling) return;
    setBusy(true);
    try {
      await api.setDepositStatus(cancelling.id, 'cancelled');
      toast.success('Depósito cancelado.');
      setCancelling(null);
      reloadAll();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo cancelar el depósito.');
    } finally {
      setBusy(false);
    }
  };

  const handleDelete = async () => {
    if (!deleting) return;
    setBusy(true);
    try {
      await api.deleteDeposit(deleting.id);
      toast.success('Depósito eliminado.');
      setDeleting(null);
      reloadAll();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo eliminar el depósito.');
    } finally {
      setBusy(false);
    }
  };

  const rows = list.data ?? [];

  return (
    <>
      <PageHeader
        title="Depósitos pendientes"
        subtitle="Transferencias y depósitos que tienes pendientes de realizar."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setFormOpen(true)}
          >
            <Icon name="plus" size={14} />
            Nuevo depósito
          </button>
        }
      />

      <div className="stat-grid">
        <Stat
          label="Depósitos pendientes"
          value={String(pendingCount)}
          meta="Sin realizar y programados"
          tone="accent"
        />
        <Stat
          label="Depósitos vencidos"
          value={String(overdueCount)}
          meta={
            overdueCount > 0 ? 'Pasó la fecha límite y siguen sin realizarse' : 'Nada vencido'
          }
          tone={overdueCount > 0 ? 'warning' : 'default'}
        />
        <Stat
          label="Depósitos completados"
          value={String(completedCount)}
          meta={`${cancelledCount} cancelados`}
        />
        <Stat
          label="Monto por depositar"
          value={formatCurrency(mainTotal, mainCurrency)}
          meta={
            otherTotals.length > 0
              ? `Además ${otherTotals
                  .map(([code, value]) => `${formatCurrency(value, code)} en ${code}`)
                  .join(' · ')}`
              : 'Suma de los depósitos aún no realizados'
          }
        />
      </div>

      <div className="alert alert-info">
        <Icon name="info" size={15} />
        <span>
          Crear un depósito no altera tus saldos. El dinero sólo se mueve cuando completas
          el depósito y eliges registrar el movimiento real.
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
          ) : rows.length === 0 ? (
            <EmptyState
              icon="send"
              title="Sin depósitos pendientes"
              text="Anota las transferencias que tienes que hacer para no olvidarlas."
              action={
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => setFormOpen(true)}
                >
                  <Icon name="plus" size={14} />
                  Nuevo depósito
                </button>
              }
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Motivo</th>
                    <th>Destinatario</th>
                    <th className="num">Monto</th>
                    <th>Fecha límite</th>
                    <th>Cuenta de origen</th>
                    <th>Institución de destino</th>
                    <th>Estado</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {rows.map((item) => (
                    <tr key={item.id}>
                      <td>
                        <div className="truncate">{item.title}</div>
                        {item.destinationDetails ? (
                          <span className="text-sm text-soft">{item.destinationDetails}</span>
                        ) : null}
                      </td>
                      <td className="text-sm">{item.contactName ?? '—'}</td>
                      <td className="num">
                        <Money value={item.amount} currency={item.currencyCode} />
                      </td>
                      <td className="text-sm">
                        {formatDate(item.dueDate)}
                        {item.dueDate ? (
                          <div className="text-sm text-soft">{dueLabel(item.dueDate)}</div>
                        ) : null}
                      </td>
                      <td className="text-sm text-muted">{item.accountName ?? '—'}</td>
                      <td className="text-sm text-muted">
                        {item.destinationInstitution ?? '—'}
                      </td>
                      <td>
                        <span className={statusBadgeClass(item.status)}>
                          {statusLabel(item.status)}
                        </span>
                      </td>
                      <td>
                        <div className="table-actions">
                          {item.status === 'pending' || item.status === 'scheduled' ? (
                            <>
                              <button
                                type="button"
                                className="btn btn-icon"
                                title="Completar depósito"
                                aria-label={`Completar ${item.title}`}
                                onClick={() => setCompleting(item)}
                              >
                                <Icon name="check" size={14} />
                              </button>
                              <button
                                type="button"
                                className="btn btn-icon"
                                title="Cancelar depósito"
                                aria-label={`Cancelar ${item.title}`}
                                onClick={() => setCancelling(item)}
                              >
                                <Icon name="close" size={14} />
                              </button>
                            </>
                          ) : null}
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Eliminar"
                            aria-label={`Eliminar ${item.title}`}
                            disabled={item.status === 'completed'}
                            onClick={() => setDeleting(item)}
                          >
                            <Icon name="trash" size={14} />
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </section>

      <DepositForm
        open={formOpen}
        currencies={currencyOptions}
        defaultCurrency={settings.defaultCurrency}
        accounts={accounts}
        onClose={() => setFormOpen(false)}
        onSaved={reloadAll}
      />

      <CompleteModal
        deposit={completing}
        accounts={accounts}
        expenseCategories={expenseCategories}
        onClose={() => setCompleting(null)}
        onSaved={reloadAll}
      />

      <ConfirmDialog
        open={cancelling !== null}
        title="Cancelar depósito"
        message={
          cancelling
            ? `El depósito «${cancelling.title}» quedará marcado como cancelado y sin realizar. Puedes completarlo más tarde si el envío se realiza.`
            : ''
        }
        confirmLabel="Cancelar depósito"
        danger
        busy={busy}
        onConfirm={handleCancel}
        onCancel={() => setCancelling(null)}
      />

      <ConfirmDialog
        open={deleting !== null}
        title="Eliminar depósito"
        message={
          deleting
            ? `Se eliminará «${deleting.title}». Esta acción no se puede deshacer.`
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
