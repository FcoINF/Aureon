/**
 * Pantalla de cuentas.
 *
 * Las cuentas son el origen de los saldos: efectivo, bancos y tarjetas.
 * El saldo actual lo calcula el backend; aquí sólo se muestra y se mantiene
 * la ficha de cada cuenta.
 */
import { useEffect, useMemo, useState } from 'react';

import { Icon } from '../../components/Icons';
import { AmountInput, Money } from '../../components/Money';
import { ConfirmDialog, Modal } from '../../components/Modal';
import { EmptyState, Field, PageHeader, Stat } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { useAsync } from '../../hooks/useAsync';
import { api, readError } from '../../services/api';
import { useAppData } from '../../services/store';
import type { Account, AccountInput, AccountType, Currency } from '../../types';
import {
  ACCOUNT_TYPE_LABELS,
  exponentOf,
  formatCurrency,
  formatDate,
  symbolOf,
  todayIso,
} from '../../utils/format';

const ACCOUNT_TYPES: AccountType[] = [
  'cash',
  'checking',
  'savings',
  'rut',
  'credit_card',
  'other',
];

interface AccountFormState {
  name: string;
  accountType: AccountType;
  currencyCode: string;
  initialBalance: number | null;
  initialBalanceDate: string;
  description: string;
}

function emptyAccountForm(currency: string): AccountFormState {
  return {
    name: '',
    accountType: 'cash',
    currencyCode: currency,
    initialBalance: 0,
    initialBalanceDate: todayIso(),
    description: '',
  };
}

interface AccountFormProps {
  open: boolean;
  account: Account | null;
  onClose: () => void;
  onSaved: () => void;
}

function AccountForm({ open, account, onClose, onSaved }: AccountFormProps) {
  const { settings, currencies, refresh } = useAppData();
  const toast = useToast();
  const [form, setForm] = useState<AccountFormState>(() =>
    emptyAccountForm(settings.defaultCurrency),
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMovements, setHasMovements] = useState(false);
  const [checkingMovements, setCheckingMovements] = useState(false);
  const [movementsError, setMovementsError] = useState<string | null>(null);

  const currencyOptions = useMemo<Currency[]>(
    () =>
      currencies.length > 0
        ? currencies
        : [
            {
              code: settings.defaultCurrency,
              name: settings.defaultCurrency,
              symbol: symbolOf(settings.defaultCurrency),
              exponent: exponentOf(settings.defaultCurrency),
            },
          ],
    [currencies, settings.defaultCurrency],
  );

  useEffect(() => {
    if (!open) return undefined;
    setError(null);
    setHasMovements(false);
    setCheckingMovements(false);
    setMovementsError(null);

    if (!account) {
      setForm(emptyAccountForm(settings.defaultCurrency));
      return undefined;
    }

    setForm({
      name: account.name,
      accountType: account.accountType,
      currencyCode: account.currencyCode,
      initialBalance: account.initialBalance,
      initialBalanceDate: account.initialBalanceDate,
      description: account.description ?? '',
    });

    // La moneda de una cuenta con movimientos ya registrados no se puede
    // cambiar: se consulta al backend para saberlo con certeza.
    let cancelled = false;
    setCheckingMovements(true);
    void (async () => {
      try {
        const movimientos = await api.listTransactions({
          accountIds: [account.id],
          includeVoided: true,
          limit: 1,
        });
        if (cancelled) return;
        const tieneMovimientos = movimientos.length > 0;
        setHasMovements(tieneMovimientos);
        if (tieneMovimientos) {
          setForm((current) => ({ ...current, currencyCode: account.currencyCode }));
        }
      } catch (err) {
        if (!cancelled) setMovementsError(readError(err));
      } finally {
        if (!cancelled) setCheckingMovements(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [open, account, settings.defaultCurrency]);

  const update = <K extends keyof AccountFormState>(
    key: K,
    value: AccountFormState[K],
  ) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSave = async () => {
    if (!form.name.trim()) {
      setError('Escribe un nombre para la cuenta.');
      return;
    }
    if (form.initialBalance === null || form.initialBalance < 0) {
      setError('El saldo inicial no puede ser negativo.');
      return;
    }
    if (!form.initialBalanceDate) {
      setError('Indica la fecha del saldo inicial.');
      return;
    }

    const input: AccountInput = {
      name: form.name.trim(),
      accountType: form.accountType,
      currencyCode: form.currencyCode,
      initialBalance: form.initialBalance,
      initialBalanceDate: form.initialBalanceDate,
      archived: account?.archived ?? false,
      description: form.description.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      if (account) {
        await api.updateAccount(account.id, input);
        toast.success('Cuenta actualizada.');
      } else {
        await api.createAccount(input);
        toast.success('Cuenta creada.');
      }
      await refresh();
      onSaved();
      onClose();
    } catch (err) {
      const message = readError(err);
      setError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title={account ? 'Editar cuenta' : 'Nueva cuenta'}
      subtitle="El saldo inicial es el punto de partida con el que cuentas en esa cuenta."
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
            {saving ? 'Guardando…' : 'Guardar'}
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
        <p className="form-section-title">Datos de la cuenta</p>
        <div className="form-grid">
          <Field label="Nombre" required className="span-2">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.name}
                placeholder="Ej.: Cuenta corriente principal"
                onChange={(event) => update('name', event.target.value)}
              />
            )}
          </Field>

          <Field label="Tipo" required>
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.accountType}
                onChange={(event) =>
                  update('accountType', event.target.value as AccountType)
                }
              >
                {ACCOUNT_TYPES.map((type) => (
                  <option key={type} value={type}>
                    {ACCOUNT_TYPE_LABELS[type] ?? type}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field
            label="Moneda"
            required
            error={movementsError ?? undefined}
            hint={
              hasMovements
                ? 'No se puede cambiar la moneda de una cuenta con movimientos.'
                : checkingMovements
                  ? 'Comprobando si la cuenta ya tiene movimientos…'
                  : undefined
            }
          >
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.currencyCode}
                disabled={account !== null && (checkingMovements || hasMovements)}
                onChange={(event) => update('currencyCode', event.target.value)}
              >
                {currencyOptions.map((currency) => (
                  <option key={currency.code} value={currency.code}>
                    {currency.code} · {currency.name}
                  </option>
                ))}
              </select>
            )}
          </Field>
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Saldo inicial</p>
        <div className="form-grid">
          <Field label="Monto" required>
            {(id) => (
              <AmountInput
                id={id}
                value={form.initialBalance}
                currency={form.currencyCode}
                onChange={(value) => update('initialBalance', value)}
              />
            )}
          </Field>

          <Field label="Fecha del saldo inicial" required>
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.initialBalanceDate}
                onChange={(event) => update('initialBalanceDate', event.target.value)}
              />
            )}
          </Field>

          <Field label="Descripción" className="span-2">
            {(id) => (
              <textarea
                id={id}
                className="textarea"
                value={form.description}
                placeholder="Ej.: cuenta donde recibes tu sueldo"
                onChange={(event) => update('description', event.target.value)}
              />
            )}
          </Field>
        </div>
      </div>
    </Modal>
  );
}

export function AccountsPage() {
  const { refresh } = useAppData();
  const toast = useToast();

  const [showArchived, setShowArchived] = useState(false);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Account | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Account | null>(null);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  // Una sola consulta trae todas las cuentas: de ella se derivan las activas.
  const all = useAsync(() => api.listAccounts(true), []);
  const allAccounts = all.data;

  const visible = useMemo(
    () =>
      showArchived
        ? (allAccounts ?? [])
        : (allAccounts ?? []).filter((account) => !account.archived),
    [allAccounts, showArchived],
  );
  const activeCount = (allAccounts ?? []).filter((account) => !account.archived).length;
  const archivedCount = (allAccounts ?? []).length - activeCount;

  const totals = useMemo(() => {
    const map = new Map<string, { total: number; count: number }>();
    for (const account of allAccounts ?? []) {
      if (account.archived) continue;
      const current = map.get(account.currencyCode) ?? { total: 0, count: 0 };
      current.total += account.balance;
      current.count += 1;
      map.set(account.currencyCode, current);
    }
    return Array.from(map.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  }, [allAccounts]);

  // Primera ejecución: la consulta terminó y no hay ninguna cuenta, ni
  // siquiera archivada.
  const isFirstRun = all.data !== null && all.data.length === 0;

  const openCreate = () => {
    setEditing(null);
    setFormOpen(true);
  };

  const openEdit = (account: Account) => {
    setEditing(account);
    setFormOpen(true);
  };

  const reload = () => {
    all.reload();
  };

  const handleToggleArchived = async (account: Account) => {
    setActionError(null);
    try {
      await api.setAccountArchived(account.id, !account.archived);
      toast.success(account.archived ? 'Cuenta restaurada.' : 'Cuenta archivada.');
      reload();
      await refresh();
    } catch (err) {
      const message = readError(err);
      setActionError(message);
      toast.error(message);
    }
  };

  const handleDelete = async () => {
    if (!pendingDelete) return;
    setBusy(true);
    setActionError(null);
    try {
      await api.deleteAccount(pendingDelete.id);
      toast.success('Cuenta eliminada.');
      setPendingDelete(null);
      reload();
      await refresh();
    } catch (err) {
      const message = readError(err);
      setActionError(message);
      toast.error(message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="Cuentas"
        subtitle="Efectivo, bancos y tarjetas donde se registran tus movimientos."
        actions={
          <button type="button" className="btn btn-primary" onClick={openCreate}>
            <Icon name="plus" size={14} />
            Nueva cuenta
          </button>
        }
      />

      {actionError ? <div className="alert alert-error">{actionError}</div> : null}

      <div className="stat-grid">
        <Stat
          label="Cuentas"
          value={String(visible.length)}
          meta={`${activeCount} activas · ${archivedCount} archivadas`}
        />
        {totals.map(([code, data]) => (
          <Stat
            key={code}
            label={`Saldo · ${code}`}
            value={formatCurrency(data.total, code)}
            meta={`${data.count} cuenta${data.count === 1 ? '' : 's'}`}
            tone="accent"
          />
        ))}
      </div>

      <section className="card">
        <div className="filter-bar">
          <div className="row" style={{ gap: 8 }}>
            <button
              type="button"
              className="btn btn-sm"
              aria-pressed={showArchived}
              style={
                showArchived
                  ? {
                      background: 'var(--accent-soft)',
                      borderColor: 'var(--accent)',
                      color: 'var(--accent-strong)',
                    }
                  : undefined
              }
              onClick={() => setShowArchived((value) => !value)}
            >
              <Icon name="filter" size={13} />
              Mostrar archivadas
            </button>
            <span className="text-sm text-muted">
              {visible.length} cuenta{visible.length === 1 ? '' : 's'}
            </span>
          </div>
        </div>

        <div className="card-body">
          {all.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : all.error ? (
            <div className="alert alert-error">{all.error}</div>
          ) : visible.length === 0 ? (
            isFirstRun ? (
              <EmptyState
                icon="wallet"
                title="Todavía no hay cuentas registradas"
                text="Crea tu primera cuenta para empezar a llevar el control de tus finanzas. Todo queda guardado en tu equipo, sin conexión a internet."
                action={
                  <button type="button" className="btn btn-primary" onClick={openCreate}>
                    <Icon name="plus" size={14} />
                    Crear la primera cuenta
                  </button>
                }
              />
            ) : (
              <EmptyState
                icon="wallet"
                title="No hay cuentas activas"
                text="Todas tus cuentas están archivadas. Actívalas o crea una nueva para empezar a registrar movimientos."
                action={
                  <button type="button" className="btn btn-primary" onClick={openCreate}>
                    <Icon name="plus" size={14} />
                    Nueva cuenta
                  </button>
                }
              />
            )
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Cuenta</th>
                    <th>Tipo</th>
                    <th className="num">Saldo actual</th>
                    <th className="num">Saldo inicial</th>
                    <th>Fecha del saldo inicial</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {visible.map((account) => (
                    <tr
                      key={account.id}
                      style={account.archived ? { opacity: 0.6 } : undefined}
                    >
                      <td>
                        <div className="row" style={{ gap: 8 }}>
                          <span className={account.archived ? 'text-soft' : ''}>
                            {account.name}
                          </span>
                          <span className="badge">{account.currencyCode}</span>
                          {account.archived ? (
                            <span className="badge badge-info">Archivada</span>
                          ) : null}
                        </div>
                        {account.description ? (
                          <div className="text-sm text-soft">{account.description}</div>
                        ) : null}
                      </td>
                      <td className="text-sm text-muted">
                        {ACCOUNT_TYPE_LABELS[account.accountType] ?? account.accountType}
                      </td>
                      <td className="num">
                        <Money
                          value={account.balance}
                          currency={account.currencyCode}
                          tone={account.balance < 0 ? 'expense' : 'default'}
                        />
                      </td>
                      <td className="num text-muted">
                        <Money value={account.initialBalance} currency={account.currencyCode} />
                      </td>
                      <td className="text-sm text-muted">
                        {formatDate(account.initialBalanceDate)}
                      </td>
                      <td>
                        <div className="table-actions">
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Editar"
                            aria-label={`Editar ${account.name}`}
                            onClick={() => openEdit(account)}
                          >
                            <Icon name="edit" size={14} />
                          </button>
                          <button
                            type="button"
                            className="btn btn-icon"
                            title={account.archived ? 'Restaurar' : 'Archivar'}
                            aria-label={
                              account.archived
                                ? `Restaurar ${account.name}`
                                : `Archivar ${account.name}`
                            }
                            onClick={() => void handleToggleArchived(account)}
                          >
                            <Icon name={account.archived ? 'play' : 'pause'} size={14} />
                          </button>
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Eliminar"
                            aria-label={`Eliminar ${account.name}`}
                            onClick={() => setPendingDelete(account)}
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

      <AccountForm
        open={formOpen}
        account={editing}
        onClose={() => {
          setFormOpen(false);
          setEditing(null);
        }}
        onSaved={reload}
      />

      <ConfirmDialog
        open={pendingDelete !== null}
        title="Eliminar cuenta"
        message={
          pendingDelete
            ? `Se eliminará «${pendingDelete.name}». Sólo es posible si la cuenta no tiene movimientos asociados; si los tiene, archívala en su lugar.`
            : ''
        }
        confirmLabel="Eliminar"
        danger
        busy={busy}
        onConfirm={handleDelete}
        onCancel={() => setPendingDelete(null)}
      />
    </>
  );
}
