import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';

import { Icon } from '../../components/Icons';
import { Money } from '../../components/Money';
import { EmptyState, PageHeader } from '../../components/Primitives';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import type { CalendarEvent } from '../../types';
import {
  MONTH_NAMES,
  SOURCE_LABELS,
  WEEKDAY_NAMES,
  formatCurrency,
  formatDate,
  statusLabel,
  todayIso,
} from '../../utils/format';

interface CalendarCell {
  iso: string;
  day: number;
  inMonth: boolean;
  isToday: boolean;
}

const SOURCE_ROUTES: Record<string, { to: string; label: string }> = {
  installment: { to: '/compras', label: 'Ver en compras y cuotas' },
  subscription: { to: '/suscripciones', label: 'Ver en suscripciones' },
  payable: { to: '/por-pagar', label: 'Ver en cuentas por pagar' },
  receivable: { to: '/por-cobrar', label: 'Ver en cuentas por cobrar' },
  deposit: { to: '/depositos', label: 'Ver en depósitos pendientes' },
};

function toIso(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/** Construye la grilla del mes: lunes primero, con cinco o seis semanas según el mes. */
function buildGrid(year: number, month: number): CalendarCell[] {
  const first = new Date(year, month, 1);
  const offset = (first.getDay() + 6) % 7;
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  const total = Math.ceil((offset + daysInMonth) / 7) * 7;
  const cells: CalendarCell[] = [];
  const start = new Date(year, month, 1 - offset);
  for (let index = 0; index < total; index += 1) {
    const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + index);
    cells.push({
      iso: toIso(date),
      day: date.getDate(),
      inMonth: date.getMonth() === month,
      isToday: toIso(date) === todayIso(),
    });
  }
  return cells;
}

/** Vencido, próximo o completado. */
function statusBadgeClass(status: string): string {
  if (status === 'overdue') return 'badge badge-warning';
  if (status === 'paid' || status === 'completed' || status === 'refunded') {
    return 'badge badge-income';
  }
  return 'badge badge-info';
}

function statusDotColor(status: string): string {
  if (status === 'overdue') return 'var(--warning)';
  if (status === 'paid' || status === 'completed' || status === 'refunded') {
    return 'var(--income)';
  }
  return 'var(--info)';
}

export function CalendarPage() {
  const now = new Date();
  const [year, setYear] = useState(now.getFullYear());
  const [month, setMonth] = useState(now.getMonth());
  const [view, setView] = useState<'month' | 'list'>('month');
  const [selected, setSelected] = useState<CalendarEvent | null>(null);

  const grid = useMemo(() => buildGrid(year, month), [year, month]);
  const gridStart = grid[0]?.iso ?? todayIso();
  const gridEnd = grid[grid.length - 1]?.iso ?? todayIso();

  const events = useAsync(() => api.calendarEvents(gridStart, gridEnd), [gridStart, gridEnd]);

  const byDate = useMemo(() => {
    const map = new Map<string, CalendarEvent[]>();
    for (const event of events.data ?? []) {
      const list = map.get(event.date) ?? [];
      list.push(event);
      map.set(event.date, list);
    }
    return map;
  }, [events.data]);

  const monthEvents = useMemo(() => {
    const prefix = `${year}-${String(month + 1).padStart(2, '0')}`;
    return (events.data ?? []).filter((event) => event.date.slice(0, 7) === prefix);
  }, [events.data, year, month]);

  const overdueCount = useMemo(
    () => monthEvents.filter((event) => event.status === 'overdue').length,
    [monthEvents],
  );

  const goToMonth = (delta: number) => {
    const next = new Date(year, month + delta, 1);
    setYear(next.getFullYear());
    setMonth(next.getMonth());
    setSelected(null);
  };

  const goToToday = () => {
    const today = new Date();
    setYear(today.getFullYear());
    setMonth(today.getMonth());
    setSelected(null);
  };

  const selectedRoute = selected ? SOURCE_ROUTES[selected.source] : null;

  return (
    <>
      <PageHeader
        title="Calendario"
        subtitle="Vencimientos de cuotas, suscripciones, cuentas por pagar y depósitos."
        actions={
          <div className="segmented" role="group" aria-label="Vista del calendario">
            <button
              type="button"
              aria-pressed={view === 'month'}
              onClick={() => setView('month')}
            >
              Mes
            </button>
            <button
              type="button"
              aria-pressed={view === 'list'}
              onClick={() => setView('list')}
            >
              Lista
            </button>
          </div>
        }
      />

      <div className="alert alert-info">
        <Icon name="info" size={15} />
        <span>
          El calendario muestra sólo las fechas de vencimiento. Ningún pago se marca como
          realizado automáticamente: registra cada pago desde el módulo correspondiente.
        </span>
      </div>

      {overdueCount > 0 ? (
        <div className="alert alert-warning">
          <Icon name="alert" size={15} />
          <span>
            Hay {overdueCount} vencimiento{overdueCount === 1 ? '' : 's'} vencido
            {overdueCount === 1 ? '' : 's'} en este mes.
          </span>
        </div>
      ) : null}

      <section className="card">
        <header className="card-header">
          <div className="row">
            <button
              type="button"
              className="btn btn-icon"
              title="Mes anterior"
              aria-label="Ver el mes anterior"
              onClick={() => goToMonth(-1)}
            >
              <Icon name="chevron-right" size={15} style={{ transform: 'rotate(180deg)' }} />
            </button>
            <h2 className="card-title">
              {MONTH_NAMES[month]} {year}
            </h2>
            <button
              type="button"
              className="btn btn-icon"
              title="Mes siguiente"
              aria-label="Ver el mes siguiente"
              onClick={() => goToMonth(1)}
            >
              <Icon name="chevron-right" size={15} />
            </button>
          </div>
          <button type="button" className="btn btn-sm" onClick={goToToday}>
            Ir a hoy
          </button>
        </header>

        <div className="card-body">
          {events.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : events.error ? (
            <div className="alert alert-error">{events.error}</div>
          ) : view === 'month' ? (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(7, minmax(0, 1fr))',
                gap: 6,
              }}
            >
              {WEEKDAY_NAMES.map((name) => (
                <div
                  key={name}
                  className="text-sm text-soft"
                  style={{ textAlign: 'center', fontWeight: 600 }}
                >
                  {name.slice(0, 3)}
                </div>
              ))}

              {grid.map((cell) => {
                const cellEvents = byDate.get(cell.iso) ?? [];
                return (
                  <div
                    key={cell.iso}
                    style={{
                      minHeight: 92,
                      padding: 6,
                      border: '1px solid var(--border)',
                      borderRadius: 'var(--radius-sm)',
                      background: cell.inMonth ? 'var(--surface)' : 'var(--surface-alt)',
                      opacity: cell.inMonth ? 1 : 0.6,
                    }}
                  >
                    <div className="row-between" style={{ marginBottom: 4 }}>
                      <span
                        className="text-sm"
                        style={{ fontWeight: cell.isToday ? 700 : 400 }}
                      >
                        {cell.day}
                      </span>
                      {cell.isToday ? (
                        <span className="text-sm" style={{ color: 'var(--accent-strong)' }}>
                          hoy
                        </span>
                      ) : null}
                    </div>

                    <div style={{ display: 'grid', gap: 3 }}>
                      {cellEvents.slice(0, 3).map((event) => (
                        <button
                          key={`${event.source}-${event.id}`}
                          type="button"
                          className="btn btn-sm"
                          style={{
                            width: '100%',
                            height: 20,
                            padding: '0 5px',
                            justifyContent: 'flex-start',
                            gap: 4,
                          }}
                          title={`${event.title} · ${formatCurrency(event.amount, event.currencyCode)}`}
                          onClick={() => setSelected(event)}
                        >
                          <span
                            className="dot"
                            style={{ color: statusDotColor(event.status) }}
                          />
                          <span className="truncate text-sm" style={{ minWidth: 0 }}>
                            {event.title}
                          </span>
                        </button>
                      ))}
                      {cellEvents.length > 3 ? (
                        <span className="text-sm text-soft">
                          +{cellEvents.length - 3} más
                        </span>
                      ) : null}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : monthEvents.length === 0 ? (
            <EmptyState
              icon="calendar"
              title="Sin vencimientos en este mes"
              text="Elige otro mes o revisa el detalle de cada compromiso en su módulo."
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Fecha</th>
                    <th>Concepto</th>
                    <th>Origen</th>
                    <th>Estado</th>
                    <th className="num">Monto</th>
                  </tr>
                </thead>
                <tbody>
                  {monthEvents.map((event) => (
                    <tr key={`${event.source}-${event.id}`}>
                      <td className="text-sm">{formatDate(event.date)}</td>
                      <td>
                        <div className="truncate">{event.title}</div>
                        {event.detail ? (
                          <span className="text-sm text-soft">{event.detail}</span>
                        ) : null}
                      </td>
                      <td className="text-sm text-muted">
                        {SOURCE_LABELS[event.source] ?? event.source}
                      </td>
                      <td>
                        <span className={statusBadgeClass(event.status)}>
                          {statusLabel(event.status)}
                        </span>
                      </td>
                      <td className="num">
                        <Money value={event.amount} currency={event.currencyCode} />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </section>

      {view === 'month' ? (
        <section className="card">
          <header className="card-header">
            <h2 className="card-title">Detalle del vencimiento</h2>
          </header>
          <div className="card-body">
            {selected ? (
              <>
                <div className="detail-list">
                  <div className="detail-row">
                    <span className="detail-key">Concepto</span>
                    <span className="detail-value">{selected.title}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Monto</span>
                    <span className="detail-value">
                      <Money value={selected.amount} currency={selected.currencyCode} />
                    </span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Fecha</span>
                    <span className="detail-value">{formatDate(selected.date)}</span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Origen</span>
                    <span className="detail-value">
                      {SOURCE_LABELS[selected.source] ?? selected.source}
                    </span>
                  </div>
                  <div className="detail-row">
                    <span className="detail-key">Estado</span>
                    <span className="detail-value">
                      <span className={statusBadgeClass(selected.status)}>
                        {statusLabel(selected.status)}
                      </span>
                    </span>
                  </div>
                  {selected.detail ? (
                    <div className="detail-row">
                      <span className="detail-key">Detalle</span>
                      <span className="detail-value">{selected.detail}</span>
                    </div>
                  ) : null}
                </div>

                <div className="row" style={{ marginTop: 'var(--space-4)' }}>
                  {selectedRoute ? (
                    <Link to={selectedRoute.to} className="btn btn-primary">
                      {selectedRoute.label}
                      <Icon name="chevron-right" size={14} />
                    </Link>
                  ) : null}
                  <span className="text-sm text-soft">
                    El estado no cambia por sí solo: registra el pago en su módulo.
                  </span>
                </div>
              </>
            ) : (
              <EmptyState
                icon="calendar"
                title="Selecciona un vencimiento"
                text="Elige un compromiso en la grilla para ver su monto, su estado y el módulo donde puedes gestionarlo."
              />
            )}
          </div>
        </section>
      ) : null}
    </>
  );
}
