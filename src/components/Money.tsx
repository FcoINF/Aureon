import { useEffect, useState } from 'react';

import { exponentOf, formatAmount, formatCurrency, parseAmountToMinor } from '../utils/format';

interface MoneyProps {
  value: number;
  currency?: string;
  tone?: 'default' | 'income' | 'expense';
  withSymbol?: boolean;
  signed?: boolean;
}

export function Money({
  value,
  currency = 'CLP',
  tone = 'default',
  withSymbol = true,
  signed,
}: MoneyProps) {
  const className =
    tone === 'default' ? 'amount' : `amount amount-${tone}`;
  const prefix = signed ? (value > 0 ? '+' : '') : '';
  return (
    <span className={className}>
      {prefix}
      {formatCurrency(value, currency, withSymbol)}
    </span>
  );
}

interface AmountInputProps {
  value: number | null;
  onChange: (minor: number | null) => void;
  currency?: string;
  placeholder?: string;
  id?: string;
  autoFocus?: boolean;
  disabled?: boolean;
  required?: boolean;
}

/**
 * Campo de monto. Acepta el formato chileno (1.234,56) y entrega siempre el
 * valor entero en unidades mínimas de la moneda.
 */
export function AmountInput({
  value,
  onChange,
  currency = 'CLP',
  placeholder = '0',
  id,
  autoFocus,
  disabled,
  required,
}: AmountInputProps) {
  const exponent = exponentOf(currency);
  const [text, setText] = useState(() =>
    value === null || value === undefined ? '' : formatAmount(value, exponent),
  );
  const [focused, setFocused] = useState(false);

  useEffect(() => {
    if (focused) return;
    setText(value === null || value === undefined ? '' : formatAmount(value, exponent));
  }, [value, exponent, focused]);

  const handleChange = (raw: string) => {
    setText(raw);
    if (raw.trim() === '') {
      onChange(null);
      return;
    }
    const minor = parseAmountToMinor(raw, exponent);
    onChange(minor);
  };

  return (
    <div className="input-group">
      <input
        id={id}
        className="input input-amount"
        inputMode="numeric"
        autoComplete="off"
        autoFocus={autoFocus}
        placeholder={placeholder}
        value={text}
        disabled={disabled}
        required={required}
        aria-label="Monto"
        onFocus={() => setFocused(true)}
        onBlur={() => {
          setFocused(false);
          const minor = parseAmountToMinor(text, exponent);
          setText(minor === null ? '' : formatAmount(minor, exponent));
        }}
        onChange={(event) => handleChange(event.target.value)}
      />
      <span className="input-suffix">{currency}</span>
    </div>
  );
}
