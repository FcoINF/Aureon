import { useEffect, useState } from 'react';

import { Icon } from '../../components/Icons';
import { AmountInput, Money } from '../../components/Money';
import { ConfirmDialog, Modal } from '../../components/Modal';
import { EmptyState, Field, PageHeader } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import type {
  PurchaseInput,
  WishlistInput,
  WishlistItem,
  WishlistStatus,
} from '../../types';
import { exponentOf, formatDate, statusLabel, todayIso } from '../../utils/format';

const STATUS_OPTIONS: { value: WishlistStatus; label: string }[] = [
  { value: 'wanted', label: 'Deseado' },
  { value: 'saving', label: 'Ahorrando' },
  { value: 'purchased', label: 'Comprado' },
  { value: 'discarded', label: 'Descartado' },
];

const PRIORITY_OPTIONS: { value: number; label: string }[] = [
  { value: 1, label: 'Alta' },
  { value: 2, label: 'Media' },
  { value: 3, label: 'Baja' },
];

type StatusFilter = WishlistStatus | 'all';

function itemLabel(item: WishlistItem): string {
  return item.name ?? item.store ?? 'Producto sin nombre';
}

function itemToInput(item: WishlistItem): WishlistInput {
  return {
    name: item.name,
    url: item.url,
    store: item.store,
    estimatedPrice: item.estimatedPrice,
    currencyCode: item.currencyCode,
    addedDate: item.addedDate,
    categoryId: item.categoryId,
    priority: item.priority,
    savingsGoal: item.savingsGoal,
    status: item.status,
    notes: item.notes,
  };
}

// ---------------------------------------------------------------------------
// Formulario de alta y edición
// ---------------------------------------------------------------------------

interface FormState {
  name: string;
  url: string;
  store: string;
  estimatedPrice: number | null;
  currency: string;
  categoryId: string;
  priority: number;
  savingsGoal: number | null;
  status: WishlistStatus;
  notes: string;
  addedDate: string;
}

function emptyForm(currency: string): FormState {
  return {
    name: '',
    url: '',
    store: '',
    estimatedPrice: null,
    currency,
    categoryId: '',
    priority: 2,
    savingsGoal: null,
    status: 'wanted',
    notes: '',
    addedDate: todayIso(),
  };
}

interface WishlistFormProps {
  open: boolean;
  item: WishlistItem | null;
  onClose: () => void;
  onSaved: () => void;
}

function WishlistForm({ open, item, onClose, onSaved }: WishlistFormProps) {
  const { settings, currencies, expenseCategories } = useAppData();
  const toast = useToast();
  const [form, setForm] = useState<FormState>(() => emptyForm(settings.defaultCurrency));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setError(null);
    if (item) {
      setForm({
        name: item.name ?? '',
        url: item.url ?? '',
        store: item.store ?? '',
        estimatedPrice: item.estimatedPrice,
        currency: item.currencyCode,
        categoryId: item.categoryId ?? '',
        priority: item.priority,
        savingsGoal: item.savingsGoal,
        status: item.status,
        notes: item.notes ?? '',
        addedDate: item.addedDate,
      });
    } else {
      setForm(emptyForm(settings.defaultCurrency));
    }
  }, [open, item, settings.defaultCurrency]);

  const update = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  /**
   * Al cambiar la moneda, los importes se convierten al exponente de la nueva
   * moneda: el número escrito conserva su valor real y no cambia de significado.
   */
  const handleCurrencyChange = (next: string) => {
    setForm((current) => {
      const factor = 10 ** (exponentOf(next) - exponentOf(current.currency));
      const convert = (value: number | null): number | null =>
        value === null ? null : Math.round(value * factor);
      return {
        ...current,
        currency: next,
        estimatedPrice: convert(current.estimatedPrice),
        savingsGoal: convert(current.savingsGoal),
      };
    });
  };

  const handleSave = async () => {
    const name = form.name.trim();
    const url = form.url.trim();

    if (!name && !url) {
      setError('Indica al menos el nombre del producto o su enlace.');
      return;
    }
    if (url && !/^https?:\/\//i.test(url)) {
      setError('El enlace debe comenzar con http:// o https://');
      return;
    }
    if (form.estimatedPrice !== null && form.estimatedPrice < 0) {
      setError('El precio estimado no puede ser negativo.');
      return;
    }
    if (form.savingsGoal !== null && form.savingsGoal < 0) {
      setError('La meta de ahorro no puede ser negativa.');
      return;
    }

    const input: WishlistInput = {
      name: name || null,
      url: url || null,
      store: form.store.trim() || null,
      estimatedPrice: form.estimatedPrice,
      currencyCode: form.currency,
      addedDate: form.addedDate,
      categoryId: form.categoryId || null,
      priority: form.priority,
      savingsGoal: form.savingsGoal,
      status: form.status,
      notes: form.notes.trim() || null,
    };

    setSaving(true);
    setError(null);
    try {
      if (item) {
        await api.updateWishlistItem(item.id, input);
        toast.success('Producto actualizado.');
      } else {
        await api.createWishlistItem(input);
        toast.success('Producto añadido a tu lista de deseos.');
      }
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo guardar el producto.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      wide
      title={item ? 'Editar producto' : 'Nuevo producto'}
      subtitle="Aureon no descarga la página del producto: completa tú el precio, la tienda y el enlace."
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
        <p className="form-section-title">Producto</p>
        <div className="form-grid">
          <Field
            label="Enlace"
            className="span-2"
            hint="Opcional. Debe comenzar con http:// o https://"
          >
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.url}
                placeholder="https://tienda.com/producto"
                onChange={(event) => update('url', event.target.value)}
              />
            )}
          </Field>

          <Field label="Nombre">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.name}
                placeholder="Ej.: notebook"
                onChange={(event) => update('name', event.target.value)}
              />
            )}
          </Field>

          <Field label="Tienda">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.store}
                placeholder="Ej.: tienda online"
                onChange={(event) => update('store', event.target.value)}
              />
            )}
          </Field>

          <Field label="Precio estimado">
            {(id) => (
              <AmountInput
                id={id}
                value={form.estimatedPrice}
                currency={form.currency}
                onChange={(value) => update('estimatedPrice', value)}
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
                {currencies.map((currency) => (
                  <option key={currency.code} value={currency.code}>
                    {currency.code} · {currency.name}
                  </option>
                ))}
              </select>
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

          <Field label="Prioridad">
            {(id) => (
              <select
                id={id}
                className="select"
                value={String(form.priority)}
                onChange={(event) => update('priority', Number(event.target.value))}
              >
                {PRIORITY_OPTIONS.map((option) => (
                  <option key={option.value} value={String(option.value)}>
                    {option.label}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Meta de ahorro" hint="Cuánto quieres reunir para comprarlo">
            {(id) => (
              <AmountInput
                id={id}
                value={form.savingsGoal}
                currency={form.currency}
                onChange={(value) => update('savingsGoal', value)}
              />
            )}
          </Field>

          <Field label="Estado">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.status}
                onChange={(event) => update('status', event.target.value as WishlistStatus)}
              >
                {STATUS_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
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

// ---------------------------------------------------------------------------
// Conversión a compra
// ---------------------------------------------------------------------------

interface ConvertFormState {
  purchaseDate: string;
  amount: number | null;
  accountId: string;
  installmentsCount: string;
  firstDueDate: string;
}

interface ConvertModalProps {
  open: boolean;
  item: WishlistItem | null;
  onClose: () => void;
  onConverted: () => void;
}

function ConvertModal({ open, item, onClose, onConverted }: ConvertModalProps) {
  const { accounts } = useAppData();
  const toast = useToast();
  const [form, setForm] = useState<ConvertFormState>({
    purchaseDate: todayIso(),
    amount: null,
    accountId: '',
    installmentsCount: '1',
    firstDueDate: todayIso(),
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // El formulario se reinicia al abrir el modal, no cada vez que la lista se
  // actualiza: depende de la apertura y del producto abierto.
  useEffect(() => {
    if (!open || !item) return;
    setError(null);
    setForm({
      purchaseDate: todayIso(),
      amount: item.estimatedPrice,
      accountId: '',
      installmentsCount: '1',
      firstDueDate: todayIso(),
    });
  }, [open, item?.id]);

  // El backend sólo admite cuentas de la misma moneda que la compra.
  const currencyAccounts = item
    ? accounts.filter((account) => account.currencyCode === item.currencyCode)
    : [];

  const update = <K extends keyof ConvertFormState>(
    key: K,
    value: ConvertFormState[K],
  ) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleConvert = async () => {
    if (!item) return;
    if (!form.amount || form.amount <= 0) {
      setError('Indica el monto de la compra.');
      return;
    }
    const count = Number.parseInt(form.installmentsCount, 10);
    if (!Number.isFinite(count) || count < 1) {
      setError('La cantidad de cuotas debe ser al menos 1.');
      return;
    }
    if (count > 1 && !form.firstDueDate) {
      setError('Indica la fecha de vencimiento de la primera cuota.');
      return;
    }

    const input: PurchaseInput = {
      name: item.name ?? 'Producto',
      url: item.url,
      purchaseDate: form.purchaseDate,
      totalAmount: form.amount,
      currencyCode: item.currencyCode,
      installmentsCount: count,
      firstDueDate: form.firstDueDate || form.purchaseDate,
      frequency: 'monthly',
      accountId: form.accountId || null,
      // Sin esto, el gasto de las cuotas queda sin categoría.
      categoryId: item.categoryId ?? null,
      paidImmediately: false,
      notes: item.notes,
    };

    setSaving(true);
    setError(null);
    try {
      await api.convertWishlistItem(item.id, input);
      toast.success('Producto convertido en compra.');
      onConverted();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo convertir el producto.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Convertir en compra"
      subtitle="Crearás una compra con su calendario de cuotas. Convertir NO registra ningún gasto: el movimiento aparece sólo cuando pagues una cuota o registres la operación."
      onClose={onClose}
      footer={
        <>
          <button type="button" className="btn" onClick={onClose} disabled={saving}>
            Cancelar
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleConvert}
            disabled={saving}
          >
            {saving ? 'Convirtiendo…' : 'Convertir en compra'}
          </button>
        </>
      }
    >
      {error ? (
        <div className="alert alert-error" style={{ marginBottom: 'var(--space-4)' }}>
          {error}
        </div>
      ) : null}

      {item ? (
        <div className="form-section">
          <p className="form-section-title">Datos de la compra</p>
          <div className="form-grid">
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

            <Field label="Monto" required>
              {(id) => (
                <AmountInput
                  id={id}
                  value={form.amount}
                  currency={item.currencyCode}
                  onChange={(value) => update('amount', value)}
                />
              )}
            </Field>

            <Field
              label="Cuenta"
              hint="Opcional: se asocia al pago de las cuotas"
            >
              {(id) => (
                <>
                  <select
                    id={id}
                    className="select"
                    value={form.accountId}
                    onChange={(event) => update('accountId', event.target.value)}
                  >
                    <option value="">Sin cuenta</option>
                    {currencyAccounts.map((account) => (
                      <option key={account.id} value={account.id}>
                        {account.name} · {account.currencyCode}
                      </option>
                    ))}
                  </select>
                  {currencyAccounts.length === 0 ? (
                    <p className="text-sm text-soft" style={{ marginTop: 6 }}>
                      No tienes cuentas en {item.currencyCode}. Puedes convertir el
                      producto igual, pero no podrás pagar sus cuotas hasta crear una
                      cuenta en {item.currencyCode}.
                    </p>
                  ) : null}
                </>
              )}
            </Field>

            <Field label="Cantidad de cuotas" required>
              {(id) => (
                <input
                  id={id}
                  className="input"
                  type="number"
                  min={1}
                  step={1}
                  value={form.installmentsCount}
                  onChange={(event) => update('installmentsCount', event.target.value)}
                />
              )}
            </Field>

            <Field
              label="Fecha de la primera cuota"
              required={Number.parseInt(form.installmentsCount, 10) > 1}
            >
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
          </div>
        </div>
      ) : null}
    </Modal>
  );
}

// ---------------------------------------------------------------------------
// Página
// ---------------------------------------------------------------------------

export function WishlistPage() {
  const { refresh } = useAppData();
  const toast = useToast();

  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<WishlistItem | null>(null);
  const [converting, setConverting] = useState<WishlistItem | null>(null);
  const [pendingDelete, setPendingDelete] = useState<WishlistItem | null>(null);
  const [busy, setBusy] = useState(false);

  const list = useAsync(
    () => api.listWishlist(statusFilter === 'all' ? undefined : statusFilter),
    [statusFilter],
  );

  const items = list.data ?? [];

  const handleStatusChange = async (item: WishlistItem, status: WishlistStatus) => {
    try {
      await api.setWishlistStatus(item.id, status);
      toast.success(`«${itemLabel(item)}» quedó en estado ${statusLabel(status)}.`);
      list.reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo cambiar el estado.');
      list.reload();
    }
  };

  const handlePriorityChange = async (item: WishlistItem, priority: number) => {
    try {
      await api.updateWishlistItem(item.id, {
        ...itemToInput(item),
        priority,
      });
      toast.success('Prioridad actualizada.');
      list.reload();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo cambiar la prioridad.');
      list.reload();
    }
  };

  const handleOpenUrl = async (item: WishlistItem) => {
    if (!item.url) return;
    try {
      await api.openExternalUrl(item.url);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo abrir el enlace.');
    }
  };

  const handleDelete = async () => {
    if (!pendingDelete) return;
    setBusy(true);
    try {
      await api.deleteWishlistItem(pendingDelete.id);
      toast.success('Producto eliminado.');
      setPendingDelete(null);
      list.reload();
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo eliminar el producto.');
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHeader
        title="Lista de deseos"
        subtitle="Anota lo que quieres comprar, cuánto cuesta y tu meta de ahorro."
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
            Nuevo producto
          </button>
        }
      />

      <section className="card">
        <div className="filter-bar">
          <Field label="Estado">
            {(id) => (
              <select
                id={id}
                className="select"
                value={statusFilter}
                onChange={(event) => setStatusFilter(event.target.value as StatusFilter)}
              >
                <option value="all">Todos</option>
                {STATUS_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            )}
          </Field>
          <span className="text-sm text-muted">
            {items.length} producto{items.length === 1 ? '' : 's'}
          </span>
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
              icon="heart"
              title="No hay productos en esta lista"
              text="Agrega lo que quieres comprar y sigue su precio, su prioridad y tu meta de ahorro."
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
                  Nuevo producto
                </button>
              }
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Producto</th>
                    <th>Enlace</th>
                    <th className="num">Precio estimado</th>
                    <th className="num">Meta de ahorro</th>
                    <th>Prioridad</th>
                    <th>Estado</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {items.map((item) => (
                    <tr key={item.id}>
                      <td>
                        <div className="truncate">{itemLabel(item)}</div>
                        <span className="text-xs text-soft">
                          {item.store ?? 'Sin tienda'}
                          {item.categoryName ? ` · ${item.categoryName}` : ''}
                          {` · Agregado el ${formatDate(item.addedDate)}`}
                        </span>
                      </td>
                      <td>
                        {item.url ? (
                          <div className="row" style={{ gap: 6 }}>
                            <span className="truncate text-sm text-muted" title={item.url}>
                              {item.url}
                            </span>
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Abrir enlace"
                              aria-label={`Abrir el enlace de ${itemLabel(item)}`}
                              onClick={() => void handleOpenUrl(item)}
                            >
                              <Icon name="external" size={14} />
                            </button>
                          </div>
                        ) : (
                          <span className="text-sm text-soft">Sin enlace</span>
                        )}
                      </td>
                      <td className="num">
                        {item.estimatedPrice === null ? (
                          <span className="text-soft">—</span>
                        ) : (
                          <Money value={item.estimatedPrice} currency={item.currencyCode} />
                        )}
                      </td>
                      <td className="num">
                        {item.savingsGoal === null ? (
                          <span className="text-soft">—</span>
                        ) : (
                          <Money value={item.savingsGoal} currency={item.currencyCode} />
                        )}
                      </td>
                      <td>
                        <select
                          className="select"
                          style={{ width: 104 }}
                          value={String(item.priority)}
                          aria-label={`Prioridad de ${itemLabel(item)}`}
                          onChange={(event) =>
                            void handlePriorityChange(item, Number(event.target.value))
                          }
                        >
                          {PRIORITY_OPTIONS.map((option) => (
                            <option key={option.value} value={String(option.value)}>
                              {option.label}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>
                        <select
                          className="select"
                          style={{ width: 132 }}
                          value={item.status}
                          aria-label={`Estado de ${itemLabel(item)}`}
                          onChange={(event) =>
                            void handleStatusChange(item, event.target.value as WishlistStatus)
                          }
                        >
                          {STATUS_OPTIONS.map((option) => (
                            <option key={option.value} value={option.value}>
                              {option.label}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td>
                        <div className="table-actions">
                          {item.purchaseId ? (
                            <span className="text-xs text-soft">Convertido</span>
                          ) : (
                            <button
                              type="button"
                              className="btn btn-icon"
                              title="Convertir en compra"
                              aria-label={`Convertir ${itemLabel(item)} en compra`}
                              onClick={() => setConverting(item)}
                            >
                              <Icon name="bag" size={14} />
                            </button>
                          )}
                          <button
                            type="button"
                            className="btn btn-icon"
                            title="Editar"
                            aria-label={`Editar ${itemLabel(item)}`}
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
                            title="Eliminar"
                            aria-label={`Eliminar ${itemLabel(item)}`}
                            onClick={() => setPendingDelete(item)}
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

      <WishlistForm
        open={formOpen}
        item={editing}
        onClose={() => {
          setFormOpen(false);
          setEditing(null);
        }}
        onSaved={() => {
          list.reload();
          void refresh();
        }}
      />

      <ConvertModal
        open={converting !== null}
        item={converting}
        onClose={() => setConverting(null)}
        onConverted={() => {
          list.reload();
          void refresh();
        }}
      />

      <ConfirmDialog
        open={pendingDelete !== null}
        title="Eliminar producto"
        message={
          pendingDelete
            ? `Se eliminará «${itemLabel(pendingDelete)}» de tu lista de deseos. Esta acción no se puede deshacer.`
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
