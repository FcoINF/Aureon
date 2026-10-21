import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';

import { Icon } from '../../components/Icons';
import { Money } from '../../components/Money';
import { EmptyState, Stat } from '../../components/Primitives';
import {
  BalanceChart,
  BreakdownBars,
  CategoryDonut,
  IncomeExpenseChart,
} from '../../components/charts/Charts';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import {
  PERIOD_OPTIONS,
  SOURCE_LABELS,
  TRANSACTION_TYPE_LABELS,
  dueLabel,
  formatDate,
} from '../../utils/format';

export function DashboardPage() {
  const { settings, accounts } = useAppData();
  const [period, setPeriod] = useState('month');
  const [currency, setCurrency] = useState(settings.defaultCurrency);

  useEffect(() => {
    setCurrency(settings.defaultCurrency);
  }, [settings.defaultCurrency]);

  const summary = useAsync(
    () => api.dashboardSummary(period, currency),
    [period, currency],
  );
  const summaryMonedas = useAsync(
    () => api.summaryByCurrency(period),
    [period],
  );
  const series = useAsync(
    () =>
      api.incomeExpenseSeries(
        period,
        currency,
        // En el gráfico de barras anual conviene agrupar por mes.
        period === 'year' ? 'month' : 'day',
      ),
    [period, currency],
  );
  const categories = useAsync(
    () => api.expensesByCategory(period, currency),
    [period, currency],
  );
  const contacts = useAsync(
    () => api.expensesByContact(period, currency),
    [period, currency],
  );
  const balance = useAsync(
    () => api.balanceEvolution(period, currency),
    [period, currency],
  );
  const recent = useAsync(() => api.recentTransactions(8), [period]);
  const upcoming = useAsync(() => api.upcomingCommitments(6), [period]);

  const hasAccounts = accounts.length > 0;

  const chartLabels = useMemo(
    () => (series.data ?? []).map((point) => point.label),
    [series.data],
  );
  const incomes = useMemo(
    () => (series.data ?? []).map((point) => point.value),
    [series.data],
  );
  const expenses = useMemo(
    () => (series.data ?? []).map((point) => point.secondary ?? 0),
    [series.data],
  );

  const data = summary.data;

  if (!hasAccounts) {
    return (
      <div className="card">
        <EmptyState
          icon="wallet"
          title="Todavía no hay cuentas registradas"
          text="Crea tu primera cuenta para empezar a llevar el control de tus finanzas. Aureon guardará todo en tu equipo, sin conexión a internet."
          action={
            <Link to="/cuentas" className="btn btn-primary">
              <Icon name="plus" size={14} />
              Crear la primera cuenta
            </Link>
          }
        />
      </div>
    );
  }

  return (
    <>
      <div className="row-between" style={{ flexWrap: 'wrap' }}>
        <div className="row">
          <div className="segmented" role="group" aria-label="Período">
            {PERIOD_OPTIONS.filter((option) =>
              ['today', 'week', 'month', 'last_month', 'year'].includes(option.value),
            ).map((option) => (
              <button
                key={option.value}
                type="button"
                aria-pressed={period === option.value}
                onClick={() => setPeriod(option.value)}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>
        <div className="row">
          <label className="field-label" htmlFor="dashboard-currency">
            Moneda
          </label>
          <select
            id="dashboard-currency"
            className="select"
            style={{ width: 110 }}
            value={currency}
            onChange={(event) => setCurrency(event.target.value)}
          >
            {(summaryMonedas.data ?? []).map((item) => (
              <option key={item.currencyCode} value={item.currencyCode}>
                {item.currencyCode}
              </option>
            ))}
            {(summaryMonedas.data ?? []).length === 0 ? (
              <option value={settings.defaultCurrency}>{settings.defaultCurrency}</option>
            ) : null}
          </select>
        </div>
      </div>

      {summary.error ? <div className="alert alert-error">{summary.error}</div> : null}

      <div className="stat-grid">
        <Stat
          label="Saldo total"
          value={<Money value={data?.balance ?? 0} currency={currency} />}
          meta="Suma de tus cuentas activas"
          tone="accent"
        />
        <Stat
          label="Ingresos del período"
          value={<Money value={data?.income ?? 0} currency={currency} />}
          tone="income"
        />
        <Stat
          label="Gastos del período"
          value={<Money value={data?.expenses ?? 0} currency={currency} />}
          tone="expense"
        />
        <Stat
          label="Balance neto"
          value={<Money value={data?.net ?? 0} currency={currency} />}
          meta="Ingresos menos gastos"
        />
      </div>

      <div className="stat-grid">
        <Stat
          label="Por cobrar"
          value={<Money value={data?.receivablePending ?? 0} currency={currency} />}
          meta="Aún no recibido"
        />
        <Stat
          label="Por pagar"
          value={<Money value={data?.payablePending ?? 0} currency={currency} />}
          meta="Obligaciones pendientes"
        />
        <Stat
          label="Cuotas por pagar"
          value={String(data?.installmentsPending ?? 0)}
          meta={
            <Money
              value={data?.installmentsPendingAmount ?? 0}
              currency={currency}
              withSymbol={false}
            />
          }
        />
        <Stat
          label="Suscripciones activas"
          value={String(data?.subscriptionsActive ?? 0)}
          meta={
            <>
              ~<Money
                value={data?.subscriptionsMonthly ?? 0}
                currency={currency}
                withSymbol={false}
              />{' '}
              al mes (estimado)
            </>
          }
        />
      </div>

      {data && data.overdueCount > 0 ? (
        <div className="alert alert-warning">
          <Icon name="alert" size={15} />
          <span>
            Tienes {data.overdueCount} compromiso{data.overdueCount === 1 ? '' : 's'} vencido
            {data.overdueCount === 1 ? '' : 's'} por un total de{' '}
            <Money value={data.overdueAmount} currency={currency} />.
          </span>
        </div>
      ) : null}

      <div className="grid-main">
        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Ingresos y gastos</h2>
            <span className="text-sm text-muted">
              {PERIOD_OPTIONS.find((option) => option.value === period)?.label}
            </span>
          </header>
          <div className="card-body">
            {chartLabels.length === 0 ? (
              <EmptyState
                icon="chart"
                title="Sin movimientos en este período"
                text="Registra ingresos y gastos para ver su evolución."
              />
            ) : (
              <IncomeExpenseChart
                labels={chartLabels}
                income={incomes}
                expenses={expenses}
                currency={currency}
                variant="bar"
              />
            )}
          </div>
        </section>

        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Gastos por categoría</h2>
          </header>
          <div className="card-body">
            {(categories.data ?? []).length === 0 ? (
              <EmptyState icon="target" title="Sin gastos registrados" />
            ) : (
              <CategoryDonut slices={categories.data ?? []} currency={currency} />
            )}
          </div>
        </section>
      </div>

      <div className="grid-main">
        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Evolución del saldo</h2>
          </header>
          <div className="card-body">
            {(balance.data ?? []).length === 0 ? (
              <EmptyState icon="chart" title="Sin historial suficiente" />
            ) : (
              <BalanceChart
                labels={(balance.data ?? []).map((point) => point.label)}
                values={(balance.data ?? []).map((point) => point.value)}
                currency={currency}
              />
            )}
          </div>
        </section>

        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Gastos por comercio o plataforma</h2>
          </header>
          <div className="card-body">
            {(contacts.data ?? []).length === 0 ? (
              <EmptyState icon="bag" title="Sin comercios registrados" />
            ) : (
              <BreakdownBars items={contacts.data ?? []} currency={currency} />
            )}
          </div>
        </section>
      </div>

      <div className="grid-2">
        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Próximos compromisos</h2>
            <Link to="/calendario" className="text-sm">
              Ver calendario
            </Link>
          </header>
          <div className="card-body flush">
            {(upcoming.data ?? []).length === 0 ? (
              <EmptyState icon="calendar" title="Sin compromisos próximos" />
            ) : (
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th>Concepto</th>
                      <th>Vencimiento</th>
                      <th className="num">Monto</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(upcoming.data ?? []).map((item, index) => (
                      <tr key={`${item.source}-${item.id}-${index}`}>
                        <td>
                          <div>{item.title}</div>
                          <span className="text-xs text-soft">
                            {SOURCE_LABELS[item.source] ?? item.source}
                          </span>
                        </td>
                        <td className="text-sm">
                          {formatDate(item.dueDate)}
                          <br />
                          <span className="text-xs text-soft">
                            {dueLabel(item.dueDate)}
                          </span>
                        </td>
                        <td className="num">
                          <Money value={item.amount} currency={item.currencyCode} />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </section>

        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Movimientos recientes</h2>
            <Link to="/movimientos" className="text-sm">
              Ver todos
            </Link>
          </header>
          <div className="card-body flush">
            {(recent.data ?? []).length === 0 ? (
              <EmptyState
                icon="arrows"
                title="Sin movimientos"
                text="Registra tu primer ingreso o gasto."
              />
            ) : (
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th>Concepto</th>
                      <th className="num">Monto</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(recent.data ?? []).map((item) => (
                      <tr key={item.id}>
                        <td>
                          <div className="truncate">{item.description}</div>
                          <span className="text-xs text-soft">
                            {formatDate(item.date)} ·{' '}
                            {TRANSACTION_TYPE_LABELS[item.transactionType]}
                            {item.accountName ? ` · ${item.accountName}` : ''}
                            {item.voided ? ' · Anulado' : ''}
                          </span>
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
      </div>

      {data && data.depositsPending > 0 ? (
        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Depósitos pendientes</h2>
            <Link to="/depositos" className="text-sm">
              Ver todos
            </Link>
          </header>
          <div className="card-body">
            <div className="row-between">
              <span className="text-muted">
                {data.depositsPending} depósito
                {data.depositsPending === 1 ? '' : 's'} por realizar
              </span>
              <Money value={data.depositsPendingAmount} currency={currency} />
            </div>
          </div>
        </section>
      ) : null}

      {data && data.upcomingCount > 0 ? (
        <p className="text-sm text-soft">
          {data.upcomingCount} compromiso
          {data.upcomingCount === 1 ? '' : 's'} pendiente
          {data.upcomingCount === 1 ? '' : 's'} por{' '}
          <Money value={data.upcomingAmount} currency={currency} /> en los próximos 30 días.
        </p>
      ) : null}
    </>
  );
}
