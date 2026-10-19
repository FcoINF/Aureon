import { useEffect, useMemo, useState } from 'react';

import { Icon } from '../../components/Icons';
import { Money } from '../../components/Money';
import { ConfirmDialog } from '../../components/Modal';
import { EmptyState, Field, PageHeader } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import type { Transaction, TransactionFilter } from '../../types';
import {
  PERIOD_OPTIONS,
  TRANSACTION_TYPE_LABELS,
  formatDate,
} from '../../utils/format';
import { TransactionForm } from './TransactionForm';

export function TransactionsPage() {
  const { accounts, expenseCategories, incomeCategories, refresh } = useAppData();
  const toast = useToast();

  const [period, setPeriod] = useState('month');
  const [customStart, setCustomStart] = useState('');
  const [customEnd, setCustomEnd] = useState('');
  const [types, setTypes] = useState<string[]>([]);
  const [accountId, setAccountId] = useState('');
  const [categoryId, setCategoryId] = useState('');
  const [search, setSearch] = useState('');
  const [sort, setSort] = useState('date');
  const [direction, setDirection] = useState('desc');
  const [includeVoided, setIncludeVoided] = useState(false);

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Transaction | null>(null);
  const [pendingVoid, setPendingVoid] = useState<Transaction | null>(null);
  const [busy, setBusy] = useState(false);

  const filter = useMemo<TransactionFilter>(
    () => ({
      period: period === 'custom' ? null : period,
      startDate: period === 'custom' ? customStart || null : null,
      endDate: period === 'custom' ? customEnd || null : null,
      types: types.length > 0 ? types : null,
      accountIds: accountId ? [accountId] : null,
      categoryIds: categoryId ? [categoryId] : null,
      search: search.trim() || null,
      includeVoided,
      sort,
      direction,
      limit: 500,
    }),
    [
      period,
      customStart,
      customEnd,
      types,
      accountId,
      categoryId,
      search,
      includeVoided,
      sort,
      direction,
    ],
  );

  const list = useAsync(() => api.listTransactions(filter), [filter]);
  const total = useAsync(() => api.countTransactions(filter), [filter]);

  useEffect(() => {
    if (period === 'custom' && !customStart && !customEnd) {
      const now = new Date();
      const end = now.toISOString().slice(0, 10);
      const start = new Date(now.getTime() - 30 * 86_400_000).toISOString().slice(0, 10);
      setCustomStart(start);
      setCustomEnd(end);
    }
  }, [period, customStart, customEnd]);

  const toggleType = (type: string) => {
    setTypes((current) =>
      current.includes(type)
        ? current.filter((item) => item !== type)
        : [...current, type],
    );
  };

  const handleVoid = async () => {
    if (!pendingVoid) return;
    setBusy(true);
    try {
      await api.voidTransaction(pendingVoid.id);
      toast.success('Movimiento anulado.');
      setPendingVoid(null);
      list.reload();
      total.reload();
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo anular el movimiento.');
    } finally {
      setBusy(false);
    }
  };

  const handleRestore = async (transaction: Transaction) => {
    try {
      await api.unvoidTransaction(transaction.id);
      toast.success('Movimiento restaurado.');
      list.reload();
      total.reload();
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo restaurar el movimiento.');
    }
  };

  const categories = [...expenseCategories, ...incomeCategories];
  const sum = (list.data ?? []).reduce(
    (acc, item) =>
      item.transactionType === 'income' || item.transactionType === 'refund'
        ? acc + item.amount
        : acc,
    0,
  );
  const spend = (list.data ?? []).reduce(
    (acc, item) => (item.transactionType === 'expense' ? acc + item.amount : acc),
    0,
  );

  return (
    <>
      <PageHeader
        title="Movimientos"
        subtitle="Registra ingresos, gastos y transferencias entre tus cuentas."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => {
              setEditing(null);
              setFormOpen(true);
            }}
          >
            <Icon name="plus" size={14} />
            Nuevo movimiento
          </button>
        }
      />

      <section className="card">
        <div className="filter-bar">
          <Field label="Período">
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

          <Field label="Categoría">
            {(id) => (
              <select
                id={id}
                className="select"
                value={categoryId}
                onChange={(event) => setCategoryId(event.target.value)}
              >
                <option value="">Todas</option>
                {categories.map((category) => (
                  <option key={category.id} value={category.id}>
                    {category.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Buscar">
            {(id) => (
              <input
                id={id}
                className="input"
                value={search}
                placeholder="Descripción, nota o comercio"
                onChange={(event) => setSearch(event.target.value)}
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

          <label className="checkbox" style={{ marginBottom: 6 }}>
            <input
              type="checkbox"
              checked={includeVoided}
              onChange={(event) => setIncludeVoided(event.target.checked)}
            />
            Mostrar anulados
          </label>
        </div>

        <div className="card-body">
          <div className="row-between" style={{ marginBottom: 'var(--space-4)' }}>
            <span className="text-sm text-muted">
              {total.data ?? 0} movimiento{(total.data ?? 0) === 1 ? '' : 's'}
            </span>
            <div className="row">
              <span className="text-sm text-muted">
                Ingresos <Money value={sum} currency="CLP" tone="income" />
              </span>
              <span className="text-sm text-muted">
                Gastos <Money value={spend} currency="CLP" tone="expense" />
              </span>
              <button
                type="button"
                className="btn btn-sm"
                onClick={() => {
                  setSort(sort === 'date' ? 'amount' : 'date');
                  setDirection(direction === 'desc' ? 'asc' : 'desc');
                }}
              >
                <Icon name="filter" size={13} />
                {sort === 'date' ? 'Por fecha' : 'Por monto'}
              </button>
            </div>
          </div>

          {list.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : list.error ? (
            <div className="alert alert-error">{list.error}</div>
          ) : (list.data ?? []).length === 0 ? (
            <EmptyState
              icon="arrows"
              title="Sin movimientos en este período"
              text="Registra un ingreso o un gasto para empezar a construir tu historial."
              action={
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => {
                    setEditing(null);
                    setFormOpen(true);
                  }}
                >
                  <Icon name="plus" size={14} />
                  Nuevo movimiento
                </button>
              }
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Fecha</th>
                    <th>Concepto</th>
                    <th>Categoría</th>
                    <th>Cuenta</th>
                    <th className="num">Monto</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {(list.data ?? []).map((item) => (
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
                        {item.contactName ? (
                          <div className="text-xs text-soft">{item.contactName}</div>
                        ) : null}
                      </td>
                      <td className="text-sm text-muted">
                        {item.categoryName ?? '—'}
                      </td>
                      <td className="text-sm text-muted">
                        {item.transactionType === 'transfer'
                          ? `${item.accountName} → ${item.transferAccountName}`
                          : (item.accountName ?? '—')}
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
                      <td>
                        <div className="table-actions">
                          {!item.voided ? (
                            <>
                              <button
                                type="button"
                                className="btn btn-icon"
                                title="Editar"
                                aria-label={`Editar ${item.description}`}
                                onClick={() => {
                                  setEditing(item);
                                  setFormOpen(true);
                                }}
                              >
                                <Icon name="edit" size={14} />
                              </button>
                              <button
                                type="button"
                                className="btn btn-icon"
                                title="Anular"
                                aria-label={`Anular ${item.description}`}
                                onClick={() => setPendingVoid(item)}
                              >
                                <Icon name="close" size={14} />
                              </button>
                            </>
                          ) : (
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Restaurar"
                              aria-label={`Restaurar ${item.description}`}
                              onClick={() => void handleRestore(item)}
                            >
                              <Icon name="unlock" size={14} />
                            </button>
                          )}
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

      <TransactionForm
        open={formOpen}
        transaction={editing}
        onClose={() => {
          setFormOpen(false);
          setEditing(null);
        }}
        onSaved={() => {
          list.reload();
          total.reload();
        }}
      />

      <ConfirmDialog
        open={pendingVoid !== null}
        title="Anular movimiento"
        message={
          pendingVoid
            ? `Se anulará «${pendingVoid.description}» por ${formatDate(
                pendingVoid.date,
              )}. El movimiento se conserva en el historial, pero dejará de sumar en tus saldos.`
            : ''
        }
        confirmLabel="Anular"
        danger
        busy={busy}
        onConfirm={handleVoid}
        onCancel={() => setPendingVoid(null)}
      />
    </>
  );
}
