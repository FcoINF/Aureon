import { useEffect, useMemo, useRef, useState } from 'react';

import { Icon } from '../../components/Icons';
import { AmountInput, Money } from '../../components/Money';
import { ConfirmDialog, Modal } from '../../components/Modal';
import { EmptyState, Field, PageHeader, Stat } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import type {
  Contact,
  Frequency,
  Subscription,
  SubscriptionInput,
  SubscriptionShare,
} from '../../types';
import {
  FREQUENCY_LABELS,
  PAYMENT_METHODS,
  dueLabel,
  formatCurrency,
  formatDate,
  statusLabel,
  todayIso,
} from '../../utils/format';

const SUB_STATUS_LABELS: Record<string, string> = {
  active: 'Activa',
  paused: 'Pausada',
  cancelled: 'Cancelada',
  expired: 'Vencida',
};

const SUBSCRIPTION_FILTERS = [
  { value: '', label: 'Todas' },
  { value: 'active', label: 'Activas' },
  { value: 'paused', label: 'Pausadas' },
  { value: 'expired', label: 'Vencidas' },
  { value: 'cancelled', label: 'Canceladas' },
];

const FREQUENCY_OPTIONS: { value: Frequency; label: string }[] = (
  Object.keys(FREQUENCY_LABELS) as Frequency[]
).map((value) => ({ value, label: FREQUENCY_LABELS[value] }));

function subscriptionBadge(status: string): string {
  if (status === 'active') return 'badge badge-income';
  if (status === 'paused') return 'badge badge-warning';
  if (status === 'cancelled') return 'badge';
  return 'badge badge-info';
}

export function SubscriptionsPage() {
  const { settings, refresh } = useAppData();
  const toast = useToast();

  const [status, setStatus] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Subscription | null>(null);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [chargeOnOpen, setChargeOnOpen] = useState(false);
  const [cancelling, setCancelling] = useState<Subscription | null>(null);
  const [busy, setBusy] = useState(false);

  const openDetail = (id: string, withCharge: boolean) => {
    setChargeOnOpen(withCharge);
    setDetailId(id);
  };

  const all = useAsync(() => api.listSubscriptions(), []);
  const list = useAsync(() => api.listSubscriptions(status || undefined), [status]);
  const items = list.data ?? [];

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

  // Todas las cifras se expressan en la moneda principal, incluidos los
  // contadores: así el resumen no mezcla universos distintos.
  const stats = useMemo(() => {
    const subscriptions = all.data ?? [];
    const currency = settings.defaultCurrency;
    const active = subscriptions.filter((item) => item.status === 'active');
    const inCurrency = subscriptions.filter((item) => item.currencyCode === currency);
    const monthly = active
      .filter((item) => item.currencyCode === currency)
      .reduce((acc, item) => acc + item.monthlyEstimate, 0);
    const yearly = active
      .filter((item) => item.currencyCode === currency)
      .reduce((acc, item) => acc + item.yearlyEstimate, 0);
    const paid = inCurrency.reduce((acc, item) => acc + item.paidTotal, 0);
    const paymentCount = inCurrency.reduce((acc, item) => acc + item.paymentCount, 0);
    const activeCount = active.filter((item) => item.currencyCode === currency).length;
    return { currency, monthly, yearly, paid, paymentCount, active: activeCount };
  }, [all.data, settings.defaultCurrency]);

  /** Refresca los vencimientos y vuelve a cargar la lista de suscripciones. */
  const handleChanged = () => {
    void (async () => {
      try {
        await api.refreshOverdueStatus();
      } catch {
        // Si no se pueden actualizar los vencimientos, se muestran los estados guardados.
      }
      list.reload();
      all.reload();
      await refresh();
    })();
  };

  const handleStatus = async (subscription: Subscription, next: string) => {
    try {
      await api.setSubscriptionStatus(subscription.id, next);
      toast.success(
        next === 'active'
          ? 'Suscripción activada.'
          : next === 'paused'
            ? 'Suscripción pausada.'
            : 'Estado de la suscripción actualizado.',
      );
      handleChanged();
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : 'No se pudo cambiar el estado de la suscripción.',
      );
    }
  };

  const handleCancel = async () => {
    if (!cancelling) return;
    setBusy(true);
    try {
      await api.setSubscriptionStatus(cancelling.id, 'cancelled');
      toast.success('Suscripción cancelada.');
      setCancelling(null);
      setDetailId(null);
      handleChanged();
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : 'No se pudo cancelar la suscripción.',
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="Suscripciones"
        subtitle="Controla tus servicios recurrentes y registra cada cobro real."
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
            Nueva suscripción
          </button>
        }
      />

      <div className="stat-grid">
        <Stat
          label="Total estimado mensual"
          value={formatCurrency(stats.monthly, stats.currency)}
          meta={`Estimado en ${stats.currency}`}
          tone="expense"
        />
        <Stat
          label="Total estimado anual"
          value={formatCurrency(stats.yearly, stats.currency)}
          meta={`Estimado en ${stats.currency}`}
          tone="expense"
        />
        <Stat
          label="Suscripciones activas"
          value={String(stats.active)}
          meta={`Sólo suscripciones en ${stats.currency}`}
        />
        <Stat
          label="Total de pagos registrados"
          value={formatCurrency(stats.paid, stats.currency)}
          meta={`${stats.paymentCount} cobro${stats.paymentCount === 1 ? '' : 's'} en el historial de ${stats.currency}`}
        />
      </div>

      <div className="alert alert-info">
        <Icon name="info" size={15} />
        <span>
          Las cifras mensuales y anuales son estimaciones calculadas a partir del
          monto y la frecuencia de cada suscripción. No son cobros reales: sólo el
          botón «Registrar cobro» genera un gasto en tus movimientos.
        </span>
      </div>

      <section className="card">
        <div className="filter-bar">
          <div className="segmented" role="group" aria-label="Estado de las suscripciones">
            {SUBSCRIPTION_FILTERS.map((filter) => (
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
          {list.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : list.error ? (
            <div className="alert alert-error">{list.error}</div>
          ) : items.length === 0 ? (
            <EmptyState
              icon="repeat"
              title={
                status === ''
                  ? 'Sin suscripciones registradas'
                  : 'Sin suscripciones en este estado'
              }
              text="Registra tus servicios recurrentes para saber cuánto gastas en ellos y controlar cada cobro."
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
                  Nueva suscripción
                </button>
              }
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Suscripción</th>
                    <th className="num">Monto</th>
                    <th>Frecuencia</th>
                    <th>Próximo cobro</th>
                    <th>Estado</th>
                    <th className="num">Est. mensual</th>
                    <th className="num">Est. anual</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {items.map((item) => (
                    <tr key={item.id}>
                      <td>
                        <button
                          type="button"
                          className="btn btn-ghost"
                          style={{ height: 'auto', padding: 0 }}
                          onClick={() => openDetail(item.id, false)}
                        >
                          {item.name}
                        </button>
                        <div className="text-sm text-soft">
                          {item.contactName ?? 'Sin proveedor'}
                        </div>
                      </td>
                      <td className="num">
                        <Money value={item.amount} currency={item.currencyCode} />
                      </td>
                      <td className="text-sm">
                        {FREQUENCY_LABELS[item.frequency] ?? item.frequency}
                      </td>
                      <td className="text-sm">
                        {item.nextChargeDate ? (
                          <>
                            {formatDate(item.nextChargeDate)}
                            <div className="text-sm text-soft">
                              {dueLabel(item.nextChargeDate)}
                            </div>
                          </>
                        ) : (
                          <span className="text-soft">Sin próximo cobro</span>
                        )}
                      </td>
                      <td>
                        <span className={subscriptionBadge(item.status)}>
                          {SUB_STATUS_LABELS[item.status] ?? statusLabel(item.status)}
                        </span>
                      </td>
                      <td className="num">
                        <Money
                          value={item.monthlyEstimate}
                          currency={item.currencyCode}
                        />
                      </td>
                      <td className="num">
                        <Money
                          value={item.yearlyEstimate}
                          currency={item.currencyCode}
                        />
                      </td>
                      <td>
                        <div className="table-actions">
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Registrar cobro"
                            aria-label={`Registrar cobro de ${item.name}`}
                            onClick={() => openDetail(item.id, true)}
                            disabled={item.status === 'cancelled'}
                          >
                            <Icon name="coins" size={14} />
                          </button>
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Editar"
                            aria-label={`Editar ${item.name}`}
                            onClick={() => {
                              setEditing(item);
                              setFormOpen(true);
                            }}
                          >
                            <Icon name="edit" size={14} />
                          </button>
                          {item.status === 'active' ? (
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Pausar"
                              aria-label={`Pausar ${item.name}`}
                              onClick={() => void handleStatus(item, 'paused')}
                            >
                              <Icon name="pause" size={14} />
                            </button>
                          ) : (
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Activar"
                              aria-label={`Activar ${item.name}`}
                              onClick={() => void handleStatus(item, 'active')}
                            >
                              <Icon name="play" size={14} />
                            </button>
                          )}
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Cancelar suscripción"
                            aria-label={`Cancelar ${item.name}`}
                            onClick={() => setCancelling(item)}
                            disabled={item.status === 'cancelled'}
                          >
                            <Icon name="close" size={14} />
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

      <SubscriptionFormModal
        open={formOpen}
        subscription={editing}
        onClose={() => {
          setFormOpen(false);
          setEditing(null);
        }}
        onSaved={() => {
          list.reload();
          all.reload();
          void refresh();
        }}
      />

      {detailId ? (
        <SubscriptionDetailModal
          subscriptionId={detailId}
          openCharge={chargeOnOpen}
          onClose={() => {
            setDetailId(null);
            setChargeOnOpen(false);
          }}
          onChanged={handleChanged}
          onRequestCancel={(subscription) => setCancelling(subscription)}
        />
      ) : null}

      <ConfirmDialog
        open={cancelling !== null}
        title="Cancelar suscripción"
        message={
          cancelling
            ? `Se cancelará «${cancelling.name}» y dejará de generar cobros. El historial de cobros y de participantes se conserva, y puedes reactivarla más tarde si la retomas.`
            : ''
        }
        confirmLabel="Cancelar suscripción"
        danger
        busy={busy}
        onConfirm={handleCancel}
        onCancel={() => setCancelling(null)}
      />
    </>
  );
}

interface SubscriptionDetailProps {
  subscriptionId: string;
  openCharge: boolean;
  onClose: () => void;
  onChanged: () => void;
  onRequestCancel: (subscription: Subscription) => void;
}

function SubscriptionDetailModal({
  subscriptionId,
  openCharge,
  onClose,
  onChanged,
  onRequestCancel,
}: SubscriptionDetailProps) {
  const { accounts } = useAppData();
  const toast = useToast();

  const subscription = useAsync(() => api.getSubscription(subscriptionId), [subscriptionId]);
  const payments = useAsync(
    () => api.listSubscriptionPayments(subscriptionId),
    [subscriptionId],
  );
  const shares = useAsync(
    () => api.listSubscriptionShares(subscriptionId),
    [subscriptionId],
  );
  const contacts = useAsync(() => api.listContacts('person'), []);

  const [charging, setCharging] = useState(false);
  const [chargeAmount, setChargeAmount] = useState<number | null>(null);
  const [chargeDate, setChargeDate] = useState(todayIso());
  const [chargeAccount, setChargeAccount] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [shareContact, setShareContact] = useState('');
  const [shareType, setShareType] = useState<'percent' | 'amount'>('percent');
  const [shareValue, setShareValue] = useState<number | null>(null);
  const [shareError, setShareError] = useState<string | null>(null);
  const [shareSaving, setShareSaving] = useState(false);

  const data = subscription.data;
  const currency = data?.currencyCode ?? 'CLP';
  const subscriptionUrl = data?.url ?? null;
  // El backend sólo admite cuentas de la misma moneda que la suscripción.
  const currencyAccounts = accounts.filter(
    (account) => account.currencyCode === currency,
  );

  const startCharge = () => {
    setCharging(true);
    // Si el formulario ya está abierto, no se borra lo que el usuario escribió.
    if (charging) return;
    setChargeAmount(data ? data.amount : null);
    setChargeDate(todayIso());
    setChargeAccount(
      currencyAccounts.some((account) => account.id === data?.accountId)
        ? (data?.accountId ?? '')
        : '',
    );
    setError(null);
  };

  const chargeStarted = useRef(false);
  useEffect(() => {
    if (!openCharge || !data || chargeStarted.current) return;
    chargeStarted.current = true;
    startCharge();
  }, [openCharge, data]);

  const submitCharge = async () => {
    if (!data) return;
    if (!chargeAmount || chargeAmount <= 0) {
      setError('Ingresa un importe mayor que cero.');
      return;
    }
    if (!chargeDate) {
      setError('Indica la fecha del cobro.');
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await api.chargeSubscription(data.id, {
        amount: chargeAmount,
        chargedAt: chargeDate,
        accountId: chargeAccount || null,
      });
      toast.success('Cobro registrado.');
      setCharging(false);
      payments.reload();
      subscription.reload();
      onChanged();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo registrar el cobro.');
    } finally {
      setSaving(false);
    }
  };

  const submitShare = async () => {
    if (!data) return;
    if (!shareContact) {
      setShareError('Elige con quién se comparte la suscripción.');
      return;
    }
    if (!shareValue || shareValue <= 0) {
      setShareError('Ingresa un valor mayor que cero.');
      return;
    }
    if (shareType === 'percent' && shareValue > 100) {
      setShareError('El porcentaje debe estar entre 1 y 100.');
      return;
    }
    setShareSaving(true);
    setShareError(null);
    try {
      await api.addSubscriptionShare(data.id, {
        contactId: shareContact,
        shareType,
        shareValue,
      });
      toast.success('Participante añadido.');
      setShareContact('');
      setShareValue(null);
      shares.reload();
    } catch (err) {
      setShareError(
        err instanceof Error ? err.message : 'No se pudo guardar la participación.',
      );
    } finally {
      setShareSaving(false);
    }
  };

  const removeShare = async (share: SubscriptionShare) => {
    try {
      await api.removeSubscriptionShare(share.id);
      toast.success('Participante eliminado.');
      shares.reload();
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : 'No se pudo eliminar la participación.',
      );
    }
  };

  return (
    <Modal
      open
      wide
      title={data?.name ?? 'Detalle de la suscripción'}
      subtitle={data?.contactName ?? undefined}
      onClose={onClose}
      footer={
        <button type="button" className="btn" onClick={onClose}>
          Cerrar
        </button>
      }
    >
      {subscription.loading ? (
        <div className="loading">
          <span className="spinner" />
        </div>
      ) : subscription.error ? (
        <div className="alert alert-error">{subscription.error}</div>
      ) : !data ? null : (
        <>
          <div className="grid-2">
            <div className="detail-list">
              <div className="detail-row">
                <span className="detail-key">Monto</span>
                <span className="detail-value">
                  <Money value={data.amount} currency={currency} />
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Frecuencia</span>
                <span className="detail-value">
                  {FREQUENCY_LABELS[data.frequency] ?? data.frequency}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Próximo cobro</span>
                <span className="detail-value">
                  {data.nextChargeDate
                    ? `${formatDate(data.nextChargeDate)} · ${dueLabel(data.nextChargeDate)}`
                    : 'Sin próximo cobro'}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Cuenta</span>
                <span className="detail-value">{data.accountName ?? '—'}</span>
              </div>
            </div>

            <div className="detail-list">
              <div className="detail-row">
                <span className="detail-key">Categoría</span>
                <span className="detail-value">{data.categoryName ?? '—'}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Método de pago</span>
                <span className="detail-value">{data.paymentMethod ?? '—'}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Inicio</span>
                <span className="detail-value">{formatDate(data.startDate)}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Estado</span>
                <span className="detail-value">
                  <span className={subscriptionBadge(data.status)}>
                    {SUB_STATUS_LABELS[data.status] ?? statusLabel(data.status)}
                  </span>
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Estimado mensual</span>
                <span className="detail-value">
                  <Money value={data.monthlyEstimate} currency={currency} />
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Estimado anual</span>
                <span className="detail-value">
                  <Money value={data.yearlyEstimate} currency={currency} />
                </span>
              </div>
            </div>
          </div>

          <div className="form-section">
            <div className="row">
              <button
                type="button"
                className="btn btn-primary btn-sm"
                onClick={startCharge}
                disabled={data.status === 'cancelled'}
              >
                <Icon name="coins" size={13} />
                Registrar cobro
              </button>
              <button
                type="button"
                className="btn btn-sm btn-danger"
                onClick={() => onRequestCancel(data)}
                disabled={data.status === 'cancelled'}
              >
                <Icon name="close" size={13} />
                Cancelar suscripción
              </button>
            </div>
            <p className="text-sm text-soft">
              Registrar un cobro genera un gasto en tus movimientos y actualiza el
              próximo cobro según la frecuencia.
            </p>
          </div>

          {subscriptionUrl ? (
            <div className="form-section">
              <p className="form-section-title">Enlace del servicio</p>
              <button
                type="button"
                className="btn btn-sm"
                onClick={() => {
                  void api
                    .openExternalUrl(subscriptionUrl)
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

          {charging ? (
            <div className="form-section">
              <p className="form-section-title">Registrar cobro</p>
              {error ? <div className="alert alert-error">{error}</div> : null}
              <div className="form-grid">
                <Field label="Importe del cobro" required>
                  {(id) => (
                    <AmountInput
                      id={id}
                      value={chargeAmount}
                      currency={currency}
                      onChange={setChargeAmount}
                    />
                  )}
                </Field>
                <Field label="Fecha del cobro" required>
                  {(id) => (
                    <input
                      id={id}
                      className="input"
                      type="date"
                      value={chargeDate}
                      onChange={(event) => setChargeDate(event.target.value)}
                    />
                  )}
                </Field>
                <Field
                  label="Cuenta"
                  className="span-2"
                  hint="Si la dejas vacía se usa la cuenta de la suscripción; si la suscripción no tiene cuenta, debes elegir una."
                >
                  {(id) => (
                    <>
                      <select
                        id={id}
                        className="select"
                        value={chargeAccount}
                        onChange={(event) => setChargeAccount(event.target.value)}
                      >
                        <option value="">Sin especificar</option>
                        {currencyAccounts.map((account) => (
                          <option key={account.id} value={account.id}>
                            {account.name} · {account.currencyCode}
                          </option>
                        ))}
                      </select>
                      {currencyAccounts.length === 0 ? (
                        <p className="text-sm text-soft" style={{ marginTop: 6 }}>
                          No tienes cuentas en {currency}. Necesitas una cuenta en{' '}
                          {currency} para registrar el cobro.
                        </p>
                      ) : null}
                    </>
                  )}
                </Field>
              </div>
              <div className="row">
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => void submitCharge()}
                  disabled={saving}
                >
                  {saving ? 'Registrando…' : 'Registrar cobro'}
                </button>
                <button
                  type="button"
                  className="btn"
                  onClick={() => {
                    setCharging(false);
                    setError(null);
                  }}
                  disabled={saving}
                >
                  Cancelar
                </button>
              </div>
            </div>
          ) : null}

          <div className="form-section">
            <p className="form-section-title">Historial de cobros</p>
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
                text="Cuando registres un cobro real aparecerá aquí, junto con su movimiento de gasto."
              />
            ) : (
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th>Fecha</th>
                      <th className="num">Importe</th>
                      <th>Cuenta</th>
                      <th>Próximo cobro</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(payments.data ?? []).map((payment) => (
                      <tr key={payment.id}>
                        <td className="text-sm">{formatDate(payment.chargedAt)}</td>
                        <td className="num">
                          <Money value={payment.amount} currency={currency} />
                        </td>
                        <td className="text-sm text-muted">
                          {payment.accountName ?? '—'}
                        </td>
                        <td className="text-sm text-muted">
                          {payment.nextChargeDate
                            ? formatDate(payment.nextChargeDate)
                            : '—'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>

          <div className="form-section">
            <p className="form-section-title">Compartir la suscripción</p>
            {shares.error ? (
              <div className="alert alert-error">{shares.error}</div>
            ) : null}
            {(shares.data ?? []).length === 0 ? (
              <p className="text-sm text-soft">
                Esta suscripción todavía no se comparte con nadie.
              </p>
            ) : (
              <div className="table-wrap">
                <table className="table">
                  <thead>
                    <tr>
                      <th>Participante</th>
                      <th>Tipo</th>
                      <th className="num">Valor</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {(shares.data ?? []).map((share) => (
                      <tr key={share.id}>
                        <td className="text-sm">{share.contactName}</td>
                        <td className="text-sm text-muted">
                          {share.shareType === 'percent'
                            ? 'Porcentaje'
                            : 'Monto fijo'}
                        </td>
                        <td className="num">
                          {share.shareType === 'percent' ? (
                            `${share.shareValue}%`
                          ) : (
                            <Money value={share.shareValue} currency={currency} />
                          )}
                        </td>
                        <td>
                          <div className="table-actions">
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Quitar participante"
                              aria-label={`Quitar a ${share.contactName} de la suscripción`}
                              onClick={() => void removeShare(share)}
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

            {shareError ? (
              <div className="alert alert-error">{shareError}</div>
            ) : null}

            <div className="form-grid">
              <Field label="Persona" className="span-2">
                {(id) => (
                  <select
                    id={id}
                    className="select"
                    value={shareContact}
                    onChange={(event) => setShareContact(event.target.value)}
                  >
                    <option value="">Selecciona una persona</option>
                    {(contacts.data ?? []).map((contact: Contact) => (
                      <option key={contact.id} value={contact.id}>
                        {contact.name}
                      </option>
                    ))}
                  </select>
                )}
              </Field>

              <Field label="Tipo de participación">
                {(id) => (
                  <select
                    id={id}
                    className="select"
                    value={shareType}
                    onChange={(event) =>
                      setShareType(event.target.value as 'percent' | 'amount')
                    }
                  >
                    <option value="percent">Porcentaje</option>
                    <option value="amount">Monto fijo</option>
                  </select>
                )}
              </Field>

              {shareType === 'percent' ? (
                <Field label="Porcentaje" required hint="Entre 1 y 100.">
                  {(id) => (
                    <input
                      id={id}
                      className="input"
                      type="number"
                      min={1}
                      max={100}
                      value={shareValue ?? ''}
                      onChange={(event) =>
                        setShareValue(Number(event.target.value) || null)
                      }
                    />
                  )}
                </Field>
              ) : (
                <Field label="Monto" required>
                  {(id) => (
                    <AmountInput
                      id={id}
                      value={shareValue}
                      currency={currency}
                      onChange={setShareValue}
                    />
                  )}
                </Field>
              )}
            </div>

            <div className="row">
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void submitShare()}
                disabled={shareSaving}
              >
                {shareSaving ? 'Guardando…' : 'Añadir participante'}
              </button>
            </div>
          </div>
        </>
      )}
    </Modal>
  );
}

interface SubscriptionFormProps {
  open: boolean;
  subscription?: Subscription | null;
  onClose: () => void;
  onSaved: () => void;
}

interface SubscriptionFormState {
  name: string;
  categoryId: string;
  contactName: string;
  url: string;
  amount: number | null;
  currency: string;
  frequency: Frequency;
  frequencyDays: number;
  startDate: string;
  nextChargeDate: string;
  accountId: string;
  paymentMethod: string;
  notes: string;
}

function emptySubscriptionForm(currency: string): SubscriptionFormState {
  return {
    name: '',
    categoryId: '',
    contactName: '',
    url: '',
    amount: null,
    currency,
    frequency: 'monthly',
    frequencyDays: 30,
    startDate: todayIso(),
    nextChargeDate: '',
    accountId: '',
    paymentMethod: '',
    notes: '',
  };
}

function formFromSubscription(subscription: Subscription): SubscriptionFormState {
  return {
    name: subscription.name,
    categoryId: subscription.categoryId ?? '',
    contactName: subscription.contactName ?? '',
    url: subscription.url ?? '',
    amount: subscription.amount,
    currency: subscription.currencyCode,
    frequency: subscription.frequency,
    frequencyDays: subscription.frequencyDays ?? 30,
    startDate: subscription.startDate,
    nextChargeDate: subscription.nextChargeDate ?? '',
    accountId: subscription.accountId ?? '',
    paymentMethod: subscription.paymentMethod ?? '',
    notes: subscription.notes ?? '',
  };
}

function SubscriptionFormModal({
  open,
  subscription,
  onClose,
  onSaved,
}: SubscriptionFormProps) {
  const { settings, currencies, accounts, expenseCategories, refresh } = useAppData();
  const toast = useToast();
  const [form, setForm] = useState<SubscriptionFormState>(
    emptySubscriptionForm(settings.defaultCurrency),
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setError(null);
    setForm(
      subscription
        ? formFromSubscription(subscription)
        : emptySubscriptionForm(settings.defaultCurrency),
    );
  }, [open, subscription, settings.defaultCurrency]);

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

  const update = <K extends keyof SubscriptionFormState>(
    key: K,
    value: SubscriptionFormState[K],
  ) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  // Las cuentas sólo pueden ser de la misma moneda que la suscripción.
  const currencyAccounts = accounts.filter(
    (account) => account.currencyCode === form.currency,
  );

  const handleCurrencyChange = (next: string) => {
    setForm((current) => ({
      ...current,
      currency: next,
      accountId: accounts.some(
        (account) => account.id === current.accountId && account.currencyCode === next,
      )
        ? current.accountId
        : '',
    }));
  };

  const handleSave = async () => {
    if (!form.name.trim()) {
      setError('Escribe el nombre del servicio.');
      return;
    }
    if (!form.amount || form.amount <= 0) {
      setError('Ingresa el monto de la suscripción.');
      return;
    }
    if (!form.startDate) {
      setError('Indica la fecha de inicio.');
      return;
    }
    if (form.frequency === 'custom' && form.frequencyDays <= 0) {
      setError('Indica cada cuántos días se repite el cobro.');
      return;
    }

    const input: SubscriptionInput = {
      name: form.name.trim(),
      categoryId: form.categoryId || null,
      contactName: form.contactName.trim() || null,
      url: form.url.trim() || null,
      amount: form.amount,
      currencyCode: form.currency,
      frequency: form.frequency,
      frequencyDays: form.frequency === 'custom' ? form.frequencyDays : null,
      startDate: form.startDate,
      nextChargeDate: form.nextChargeDate || null,
      accountId: form.accountId || null,
      paymentMethod: form.paymentMethod || null,
      // Sin esto, cada edición borraría el responsable asignado.
      responsibleContactId: subscription?.responsibleContactId ?? null,
      status: subscription?.status ?? 'active',
      notes: form.notes.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      if (subscription) {
        await api.updateSubscription(subscription.id, input);
        toast.success('Suscripción actualizada.');
      } else {
        await api.createSubscription(input);
        toast.success('Suscripción registrada.');
      }
      await refresh();
      onSaved();
      onClose();
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : 'No se pudo guardar la suscripción.',
      );
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      wide
      title={subscription ? 'Editar suscripción' : 'Nueva suscripción'}
      subtitle="Crear una suscripción no genera ningún gasto: registra cada cobro cuando ocurra."
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
        <p className="form-section-title">Detalle</p>
        <div className="form-grid">
          <Field label="Nombre del servicio" required className="span-2">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.name}
                placeholder="Ej.: streaming de video"
                onChange={(event) => update('name', event.target.value)}
              />
            )}
          </Field>

          <Field
            label="Proveedor"
            hint="Si no existe, se creará automáticamente como comercio."
          >
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.contactName}
                placeholder="Ej.: Netflix"
                onChange={(event) => update('contactName', event.target.value)}
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
                onChange={(event) => handleCurrencyChange(event.target.value)}
              >
                {currencyOptions.map((currency) => (
                  <option key={currency.code} value={currency.code}>
                    {currency.code} · {currency.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="URL del servicio" className="span-2">
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
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Cobros</p>
        <div className="form-grid">
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
                  step={1}
                  value={form.frequencyDays}
                  onChange={(event) =>
                    update('frequencyDays', Math.max(1, Math.round(Number(event.target.value) || 1)))
                  }
                />
              )}
            </Field>
          ) : null}

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

          <Field
            label="Próximo cobro"
            hint="Si lo dejas vacío se usa la fecha de inicio."
          >
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.nextChargeDate}
                onChange={(event) => update('nextChargeDate', event.target.value)}
              />
            )}
          </Field>

          <Field label="Cuenta">
            {(id) => (
              <>
                <select
                  id={id}
                  className="select"
                  value={form.accountId}
                  onChange={(event) => update('accountId', event.target.value)}
                >
                  <option value="">Sin especificar</option>
                  {currencyAccounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {account.name} · {account.currencyCode}
                    </option>
                  ))}
                </select>
                {currencyAccounts.length === 0 ? (
                  <p className="text-sm text-soft" style={{ marginTop: 6 }}>
                    No tienes cuentas en {form.currency}. Puedes guardar la
                    suscripción igual, pero no podrás registrar cobros hasta crear
                    una cuenta en {form.currency}.
                  </p>
                ) : null}
              </>
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
