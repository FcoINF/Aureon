import { useEffect, useMemo, useState } from 'react';

import { Modal } from '../../components/Modal';
import { AmountInput } from '../../components/Money';
import { Field } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import type { Transaction, TransactionInput, TransactionType } from '../../types';
import { PAYMENT_METHODS, TRANSACTION_TYPE_LABELS, todayIso } from '../../utils/format';

const TYPES: TransactionType[] = ['expense', 'income', 'transfer', 'refund'];

interface TransactionFormProps {
  open: boolean;
  transaction?: Transaction | null;
  onClose: () => void;
  onSaved: () => void;
}

interface FormState {
  transactionType: TransactionType;
  description: string;
  amount: number | null;
  currency: string;
  date: string;
  categoryId: string;
  accountId: string;
  transferAccountId: string;
  contactId: string;
  paymentMethod: string;
  notes: string;
  tags: string;
  relatedTransactionId: string;
}

function emptyState(currency: string): FormState {
  return {
    transactionType: 'expense',
    description: '',
    amount: null,
    currency,
    date: todayIso(),
    categoryId: '',
    accountId: '',
    transferAccountId: '',
    contactId: '',
    paymentMethod: '',
    notes: '',
    tags: '',
    relatedTransactionId: '',
  };
}

export function TransactionForm({
  open,
  transaction,
  onClose,
  onSaved,
}: TransactionFormProps) {
  const { settings, accounts, expenseCategories, incomeCategories, refresh } = useAppData();
  const toast = useToast();
  const [form, setForm] = useState<FormState>(emptyState(settings.defaultCurrency));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [contacts, setContacts] = useState<{ id: string; name: string }[]>([]);
  const [expenses, setExpenses] = useState<Transaction[]>([]);

  useEffect(() => {
    if (!open) return;
    setError(null);
    if (transaction) {
      setForm({
        transactionType: transaction.transactionType,
        description: transaction.description,
        amount: transaction.amount,
        currency: transaction.currencyCode,
        date: transaction.date,
        categoryId: transaction.categoryId ?? '',
        accountId: transaction.accountId ?? '',
        transferAccountId: transaction.transferAccountId ?? '',
        contactId: transaction.contactId ?? '',
        paymentMethod: transaction.paymentMethod ?? '',
        notes: transaction.notes ?? '',
        tags: transaction.tags ?? '',
        relatedTransactionId: transaction.relatedTransactionId ?? '',
      });
    } else {
      setForm(emptyState(settings.defaultCurrency));
    }
  }, [open, transaction, settings.defaultCurrency]);

  useEffect(() => {
    if (!open) return;
    void api.listContacts().then(setContacts).catch(() => setContacts([]));
    void api
      .listTransactions({ types: ['expense'], limit: 200 })
      .then(setExpenses)
      .catch(() => setExpenses([]));
  }, [open]);

  const categories = useMemo(
    () => (form.transactionType === 'income' ? incomeCategories : expenseCategories),
    [form.transactionType, incomeCategories, expenseCategories],
  );

  const update = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleSave = async () => {
    if (!form.description.trim()) {
      setError('Escribe una descripción.');
      return;
    }
    if (!form.amount || form.amount <= 0) {
      setError('Ingresa un monto mayor que cero.');
      return;
    }
    if (!form.accountId) {
      setError('Indica la cuenta afectada.');
      return;
    }
    if (form.transactionType === 'transfer' && !form.transferAccountId) {
      setError('Indica la cuenta de destino de la transferencia.');
      return;
    }
    if (form.transactionType === 'refund' && !form.relatedTransactionId) {
      setError('Indica el gasto original al que corresponde la devolución.');
      return;
    }

    const input: TransactionInput = {
      transactionType: form.transactionType,
      description: form.description.trim(),
      amount: form.amount,
      currencyCode: form.currency,
      date: form.date,
      categoryId: form.categoryId || null,
      accountId: form.accountId || null,
      transferAccountId:
        form.transactionType === 'transfer' ? form.transferAccountId || null : null,
      contactId: form.contactId || null,
      paymentMethod: form.paymentMethod || null,
      notes: form.notes || null,
      tags: form.tags || null,
      relatedTransactionId:
        form.transactionType === 'refund' ? form.relatedTransactionId || null : null,
    };

    setSaving(true);
    setError(null);
    try {
      if (transaction) {
        await api.updateTransaction(transaction.id, input);
        toast.success('Movimiento actualizado.');
      } else {
        await api.createTransaction(input);
        toast.success('Movimiento registrado.');
      }
      await refresh();
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudo guardar el movimiento.');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      wide
      title={transaction ? 'Editar movimiento' : 'Nuevo movimiento'}
      subtitle="Una transferencia entre tus cuentas no cuenta como ingreso ni como gasto."
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
        <p className="form-section-title">Tipo</p>
        <div className="segmented" role="group" aria-label="Tipo de movimiento">
          {TYPES.map((type) => (
            <button
              key={type}
              type="button"
              aria-pressed={form.transactionType === type}
              onClick={() => update('transactionType', type)}
            >
              {TRANSACTION_TYPE_LABELS[type]}
            </button>
          ))}
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Detalle</p>
        <div className="form-grid">
          <Field label="Descripción" required className="span-2">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.description}
                placeholder="Ej.: compra semanal del súper"
                onChange={(event) => update('description', event.target.value)}
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

          <Field label="Fecha" required>
            {(id) => (
              <input
                id={id}
                className="input"
                type="date"
                value={form.date}
                onChange={(event) => update('date', event.target.value)}
              />
            )}
          </Field>

          <Field
            label="Categoría"
            hint={
              form.transactionType === 'transfer'
                ? 'Las transferencias no llevan categoría.'
                : undefined
            }
          >
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.categoryId}
                disabled={form.transactionType === 'transfer'}
                onChange={(event) => update('categoryId', event.target.value)}
              >
                <option value="">Sin categoría</option>
                {categories.map((category) => (
                  <option key={category.id} value={category.id}>
                    {category.name}
                  </option>
                ))}
              </select>
            )}
          </Field>

          <Field label="Comercio o persona">
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.contactId}
                onChange={(event) => update('contactId', event.target.value)}
              >
                <option value="">Sin especificar</option>
                {contacts.map((contact) => (
                  <option key={contact.id} value={contact.id}>
                    {contact.name}
                  </option>
                ))}
              </select>
            )}
          </Field>
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Cuentas</p>
        <div className="form-grid">
          <Field
            label={form.transactionType === 'income' ? 'Cuenta receptora' : 'Cuenta de origen'}
            required
          >
            {(id) => (
              <select
                id={id}
                className="select"
                value={form.accountId}
                onChange={(event) => update('accountId', event.target.value)}
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

          {form.transactionType === 'transfer' ? (
            <Field label="Cuenta de destino" required>
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={form.transferAccountId}
                  onChange={(event) => update('transferAccountId', event.target.value)}
                >
                  <option value="">Selecciona una cuenta</option>
                  {accounts
                    .filter((account) => account.id !== form.accountId)
                    .map((account) => (
                      <option key={account.id} value={account.id}>
                        {account.name} · {account.currencyCode}
                      </option>
                    ))}
                </select>
              )}
            </Field>
          ) : null}

          {form.transactionType === 'refund' ? (
            <Field label="Gasto original" required className="span-2">
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={form.relatedTransactionId}
                  onChange={(event) => update('relatedTransactionId', event.target.value)}
                >
                  <option value="">Selecciona el gasto</option>
                  {expenses.map((expense) => (
                    <option key={expense.id} value={expense.id}>
                      {expense.description} · {expense.date}
                    </option>
                  ))}
                </select>
              )}
            </Field>
          ) : null}
        </div>
      </div>

      <div className="form-section">
        <p className="form-section-title">Otros datos</p>
        <div className="form-grid">
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

          <Field label="Etiquetas" hint="Separadas por comas">
            {(id) => (
              <input
                id={id}
                className="input"
                value={form.tags}
                placeholder="hogar, recurrente"
                onChange={(event) => update('tags', event.target.value)}
              />
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
