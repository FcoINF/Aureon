/**
 * Pantalla de presupuestos.
 *
 * Un presupuesto es sólo un límite mensual por categoría: nunca genera
 * movimientos, sólo muestra cuánto llevas gastado y cuánto queda.
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
import type { Budget, BudgetInput, Currency } from '../../types';
import { exponentOf, formatCurrency, formatDate, symbolOf, todayIso } from '../../utils/format';

interface BudgetFormState {
  categoryId: string;
  currencyCode: string;
  amount: number | null;
  startDate: string;
}

function emptyBudgetForm(currency: string): BudgetFormState {
  return {
    categoryId: '',
    currencyCode: currency,
    amount: null,
    startDate: todayIso(),
  };
}

interface BudgetFormProps {
  open: boolean;
  budget: Budget | null;
  presetCategoryId?: string;
  onClose: () => void;
  onSaved: () => void;
}

function BudgetForm({
  open,
  budget,
  presetCategoryId,
  onClose,
  onSaved,
}: BudgetFormProps) {
  const { settings, currencies, expenseCategories, refresh } = useAppData();
  const toast = useToast();
  const [form, setForm] = useState<BudgetFormState>(() =>
    emptyBudgetForm(settings.defaultCurrency),
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  const categoryOptions = useMemo(() => {
    if (!budget) return expenseCategories;
    if (expenseCategories.some((category) => category.id === budget.categoryId)) {
      return expenseCategories;
    }
    return [
      ...expenseCategories,
      {
        id: budget.categoryId,
        name: budget.categoryName,
        kind: 'expense' as const,
        color: null,
        archived: false,
        createdAt: budget.startDate,
        spent: null,
        budget: null,
      },
    ];
  }, [expenseCategories, budget]);

  useEffect(() => {
    if (!open) return;
    setError(null);
    if (budget) {
      setForm({
        categoryId: budget.categoryId,
        currencyCode: budget.currencyCode,
        amount: budget.amount,
        startDate: budget.startDate,
      });
    } else {
      setForm({
        ...emptyBudgetForm(settings.defaultCurrency),
        categoryId: presetCategoryId ?? '',
      });
    }
  }, [open, budget, presetCategoryId, settings.defaultCurrency]);

  const update = <K extends keyof BudgetFormState>(
    key: K,
    value: BudgetFormState[K],
  ) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSave = async () => {
    if (!form.categoryId) {
      setError('Elige la categoría que quieres limitar.');
      return;
    }
    if (form.amount === null || form.amount <= 0) {
      setError('Ingresa un límite mayor que cero.');
      return;
    }
    if (!form.startDate) {
      setError('Indica la fecha de inicio del presupuesto.');
      return;
    }

    const input: BudgetInput = {
      categoryId: form.categoryId,
      currencyCode: form.currencyCode,
      amount: form.amount,
      startDate: form.startDate,
    };

    setSaving(true);
    setError(null);
    try {
      if (budget) {
        // Al editar se actualiza el presupuesto existente: así no se pisa el
        // límite de otra categoría ni se pierde el historial del límite.
        await api.updateBudget(budget.id, input);
        toast.success('Presupuesto actualizado.');
      } else {
        await api.saveBudget(input);
        toast.success('Presupuesto creado.');
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
      title={budget ? 'Editar presupuesto' : 'Nuevo presupuesto'}
      subtitle="El presupuesto es un límite: no registra ni genera ningún movimiento."
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
        <p className="form-section-title">Límite mensual</p>
        <div className="form-grid">
          <Field label="Categoría" required className="span-2">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.categoryId}
                onChange={(event) => update('categoryId', event.target.value)}
              >
                <option value="">Selecciona una categoría</option>
                {categoryOptions.map((category) => (
                  <option key={category.id} value={category.id}>
                    {category.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Moneda" required>
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.currencyCode}
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

          <Field label="Límite mensual" required>
            {(id) => (
              <AmountInput
                id={id}
                value={form.amount}
                currency={form.currencyCode}
                onChange={(value) => update('amount', value)}
              />
            )}
          </Field>

          <Field label="Fecha de inicio" required>
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.startDate}
                onChange={(event) => update('startDate', event.target.value)}
              />
            )}
          </Field>
        </div>
      </div>
    </Modal>
  );
}

export function BudgetsPage() {
  const { settings, refresh } = useAppData();
  const toast = useToast();

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Budget | null>(null);
  const [presetCategoryId, setPresetCategoryId] = useState<string | undefined>(
    undefined,
  );
  const [pendingDelete, setPendingDelete] = useState<Budget | null>(null);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const budgets = useAsync(() => api.listBudgets(), []);
  const categories = useAsync(
    () => api.listCategories('expense', false, 'month', settings.defaultCurrency),
    [settings.defaultCurrency],
  );

  const items = budgets.data ?? [];

  const budgetCategoryIds = useMemo(
    () => new Set((budgets.data ?? []).map((budget) => budget.categoryId)),
    [budgets.data],
  );

  const sinPresupuesto = useMemo(
    () =>
      (categories.data ?? [])
        .filter(
          (category) =>
            !budgetCategoryIds.has(category.id) && (category.spent ?? 0) > 0,
        )
        .sort((a, b) => (b.spent ?? 0) - (a.spent ?? 0)),
    [categories.data, budgetCategoryIds],
  );

  const hayGastos = useMemo(
    () => (categories.data ?? []).some((category) => (category.spent ?? 0) > 0),
    [categories.data],
  );

  const groups = useMemo(() => {
    const map = new Map<
      string,
      { limit: number; spent: number; count: number }
    >();
    for (const budget of items) {
      const current = map.get(budget.currencyCode) ?? {
        limit: 0,
        spent: 0,
        count: 0,
      };
      current.limit += budget.amount;
      current.spent += budget.periodSpent;
      current.count += 1;
      map.set(budget.currencyCode, current);
    }
    return Array.from(map.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  }, [budgets.data]);

  const openCreate = () => {
    setEditing(null);
    setPresetCategoryId(undefined);
    setFormOpen(true);
  };

  const openEdit = (budget: Budget) => {
    setPresetCategoryId(undefined);
    setEditing(budget);
    setFormOpen(true);
  };

  const openCreateForCategory = (categoryId: string) => {
    setEditing(null);
    setPresetCategoryId(categoryId);
    setFormOpen(true);
  };

  const handleDelete = async () => {
    if (!pendingDelete) return;
    setBusy(true);
    setActionError(null);
    try {
      await api.deleteBudget(pendingDelete.id);
      toast.success('Presupuesto eliminado.');
      setPendingDelete(null);
      budgets.reload();
      categories.reload();
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
        title="Presupuestos"
        subtitle="Límites mensuales por categoría para saber cuánto queda por gastar."
        actions={
          <button type="button" className="btn btn-primary" onClick={openCreate}>
            <Icon name="plus" size={14} />
            Nuevo presupuesto
          </button>
        }
      />

      {actionError ? <div className="alert alert-error">{actionError}</div> : null}

      {groups.map(([code, data]) => {
        const remaining = data.limit - data.spent;
        return (
          <div key={code} className="stack">
            <p className="form-section-title">Límites en {code}</p>
            <div className="stat-grid">
              <Stat
                label="Límite total"
                value={formatCurrency(data.limit, code)}
                meta={`${data.count} categoría${data.count === 1 ? '' : 's'}`}
              />
              <Stat
                label="Gastado"
                value={formatCurrency(data.spent, code)}
                tone="expense"
              />
              <Stat
                label="Restante"
                value={formatCurrency(remaining, code)}
                tone={remaining >= 0 ? 'income' : 'expense'}
              />
            </div>
          </div>
        );
      })}

      <section className="card">
        <div className="card-header">
          <h2 className="card-title">Presupuestos del mes en curso</h2>
          <span className="text-sm text-muted">
            {items.length} categoría{items.length === 1 ? '' : 's'}
          </span>
        </div>
        <div className="card-body">
          {budgets.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : budgets.error ? (
            <div className="alert alert-error">{budgets.error}</div>
          ) : items.length === 0 ? (
            <EmptyState
              icon="target"
              title="Todavía no tienes presupuestos"
              text="Crea un límite mensual por categoría para saber cuánto llevas gastado y cuánto te queda."
              action={
                <button type="button" className="btn btn-primary" onClick={openCreate}>
                  <Icon name="plus" size={14} />
                  Crear el primer presupuesto
                </button>
              }
            />
          ) : (
            <div className="grid-2">
              {items.map((budget) => {
                const percent =
                  budget.amount > 0
                    ? (budget.periodSpent / budget.amount) * 100
                    : 0;
                const remaining = budget.amount - budget.periodSpent;
                const over = percent > 100;
                const warning = percent >= 75;
                return (
                  <div key={budget.id} className="card">
                    <div className="card-body stack">
                      <div className="row-between">
                        <div className="grow">
                          <div className="row" style={{ gap: 8 }}>
                            <strong>{budget.categoryName}</strong>
                            <span className="badge">{budget.currencyCode}</span>
                            {over ? (
                              <span className="badge badge-expense">Excedido</span>
                            ) : warning ? (
                              <span className="badge badge-warning">Cerca del límite</span>
                            ) : null}
                          </div>
                          <div className="text-sm text-muted">
                            Límite{' '}
                            <Money value={budget.amount} currency={budget.currencyCode} />
                          </div>
                        </div>
                        <div className="row" style={{ gap: 4 }}>
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Editar"
                            aria-label={`Editar presupuesto de ${budget.categoryName}`}
                            onClick={() => openEdit(budget)}
                          >
                            <Icon name="edit" size={14} />
                          </button>
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Eliminar"
                            aria-label={`Eliminar presupuesto de ${budget.categoryName}`}
                            onClick={() => setPendingDelete(budget)}
                          >
                            <Icon name="trash" size={14} />
                          </button>
                        </div>
                      </div>

                      <div
                        className="progress"
                        role="progressbar"
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-valuenow={Math.round(Math.min(100, percent))}
                        aria-valuetext={`${Math.round(percent)}% del límite de ${budget.categoryName}`}
                        aria-label={`Gasto de ${budget.categoryName}`}
                      >
                        <div
                          className={`progress-bar${
                            over ? ' is-over' : warning ? ' is-warning' : ''
                          }`}
                          style={{ width: `${Math.min(100, percent)}%` }}
                        />
                      </div>

                      <div className="row-between text-sm">
                        <span className="text-muted">
                          Gastado{' '}
                          <Money value={budget.periodSpent} currency={budget.currencyCode} />{' '}
                          <span className="text-soft">{budget.currencyCode}</span>
                        </span>
                        <span className={over ? 'amount amount-expense' : 'text-muted'}>
                          {over
                            ? `Excedido ${formatCurrency(-remaining, budget.currencyCode)}`
                            : `Restante ${formatCurrency(remaining, budget.currencyCode)}`}
                        </span>
                      </div>

                      <div className="row-between text-sm text-soft">
                        <span>{Math.round(percent)}% del límite</span>
                        <span>Desde {formatDate(budget.startDate)}</span>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </section>

      <section className="card">
        <div className="card-header">
          <h2 className="card-title">Gastos en categorías sin presupuesto</h2>
          <span className="text-sm text-muted">Mes actual</span>
        </div>
        <div className="card-body">
          {categories.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : categories.error ? (
            <div className="alert alert-error">{categories.error}</div>
          ) : sinPresupuesto.length === 0 ? (
            <EmptyState
              icon="chart"
              title={
                hayGastos
                  ? 'Nada fuera de presupuesto'
                  : 'Sin gastos registrados este mes'
              }
              text={
                hayGastos
                  ? 'Todas las categorías con gastos de este mes tienen un límite definido.'
                  : 'Cuando registres gastos en categorías sin límite, los verás aquí.'
              }
            />
          ) : (
            <>
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th>Categoría</th>
                      <th className="num">Gastado este mes ({settings.defaultCurrency})</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {sinPresupuesto.map((category) => (
                      <tr key={category.id}>
                        <td>{category.name}</td>
                        <td className="num">
                          <Money
                            value={category.spent ?? 0}
                            currency={settings.defaultCurrency}
                            tone="expense"
                          />
                        </td>
                        <td>
                          <div className="table-actions">
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Crear presupuesto"
                              aria-label={`Crear presupuesto para ${category.name}`}
                              onClick={() => openCreateForCategory(category.id)}
                            >
                              <Icon name="plus" size={14} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="text-sm text-soft" style={{ marginTop: 'var(--space-3)' }}>
                Los montos se muestran en {settings.defaultCurrency}, tu moneda por defecto.
                Crea un presupuesto para fijar un límite a estas categorías.
              </p>
            </>
          )}
        </div>
      </section>

      <BudgetForm
        open={formOpen}
        budget={editing}
        presetCategoryId={presetCategoryId}
        onClose={() => {
          setFormOpen(false);
          setEditing(null);
          setPresetCategoryId(undefined);
        }}
        onSaved={() => {
          budgets.reload();
          categories.reload();
        }}
      />

      <ConfirmDialog
        open={pendingDelete !== null}
        title="Eliminar presupuesto"
        message={
          pendingDelete
            ? `Se eliminará el límite de «${pendingDelete.categoryName}». Los movimientos ya registrados no se modifican.`
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
