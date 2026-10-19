-- Aureon · Migración 0001 · Esquema inicial
-- Todos los importes monetarios se guardan como enteros en la unidad mínima
-- de la moneda (centavos, pesos, etc.). El exponente de cada moneda vive en
-- la tabla `currencies`. Nunca se usan números de punto flotante para dinero.

CREATE TABLE IF NOT EXISTS currencies (
    code      TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    symbol    TEXT NOT NULL,
    exponent  INTEGER NOT NULL DEFAULT 0 CHECK (exponent BETWEEN 0 AND 4)
);

INSERT OR IGNORE INTO currencies (code, name, symbol, exponent) VALUES
    ('CLP', 'Peso chileno',    '$',  0),
    ('USD', 'Dólar estadounidense', 'US$', 2),
    ('EUR', 'Euro',            '€',  2);

CREATE TABLE IF NOT EXISTS categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('income', 'expense')),
    color       TEXT,
    archived    INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    created_at  TEXT NOT NULL,
    UNIQUE (name, kind)
);

CREATE TABLE IF NOT EXISTS accounts (
    id                    TEXT PRIMARY KEY,
    name                  TEXT NOT NULL,
    type                  TEXT NOT NULL CHECK (type IN ('cash', 'checking', 'savings', 'rut', 'credit_card', 'other')),
    currency_code         TEXT NOT NULL REFERENCES currencies (code),
    initial_balance       INTEGER NOT NULL DEFAULT 0,
    initial_balance_date  TEXT NOT NULL,
    archived              INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    description           TEXT,
    created_at            TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS contacts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('person', 'merchant')),
    notes       TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS transactions (
    id                      TEXT PRIMARY KEY,
    type                    TEXT NOT NULL CHECK (type IN ('income', 'expense', 'transfer', 'refund')),
    description             TEXT NOT NULL,
    amount                  INTEGER NOT NULL CHECK (amount > 0),
    currency_code           TEXT NOT NULL REFERENCES currencies (code),
    date                    TEXT NOT NULL,
    category_id             TEXT REFERENCES categories (id) ON DELETE SET NULL,
    account_id              TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    transfer_account_id     TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    contact_id              TEXT REFERENCES contacts (id) ON DELETE SET NULL,
    payment_method          TEXT,
    notes                   TEXT,
    tags                    TEXT,
    voided                  INTEGER NOT NULL DEFAULT 0 CHECK (voided IN (0, 1)),
    voided_at               TEXT,
    linked_type             TEXT,
    linked_id               TEXT,
    related_transaction_id  TEXT REFERENCES transactions (id) ON DELETE SET NULL,
    created_at              TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transactions_date       ON transactions (date);
CREATE INDEX IF NOT EXISTS idx_transactions_account    ON transactions (account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_category   ON transactions (category_id);
CREATE INDEX IF NOT EXISTS idx_transactions_contact    ON transactions (contact_id);
CREATE INDEX IF NOT EXISTS idx_transactions_type       ON transactions (type);
CREATE INDEX IF NOT EXISTS idx_transactions_linked     ON transactions (linked_type, linked_id);

CREATE TABLE IF NOT EXISTS budgets (
    id              TEXT PRIMARY KEY,
    category_id     TEXT NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    currency_code   TEXT NOT NULL REFERENCES currencies (code),
    amount          INTEGER NOT NULL CHECK (amount >= 0),
    start_date      TEXT NOT NULL,
    end_date        TEXT,
    created_at      TEXT NOT NULL,
    UNIQUE (category_id, currency_code)
);

CREATE TABLE IF NOT EXISTS purchases (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    contact_id          TEXT REFERENCES contacts (id) ON DELETE SET NULL,
    url                 TEXT,
    purchase_date       TEXT NOT NULL,
    subtotal_amount     INTEGER NOT NULL DEFAULT 0,
    interest_amount     INTEGER NOT NULL DEFAULT 0,
    fees_amount         INTEGER NOT NULL DEFAULT 0,
    total_amount        INTEGER NOT NULL CHECK (total_amount >= 0),
    currency_code       TEXT NOT NULL REFERENCES currencies (code),
    installments_count  INTEGER NOT NULL DEFAULT 1 CHECK (installments_count >= 1),
    installment_amount  INTEGER NOT NULL CHECK (installment_amount >= 0),
    first_due_date      TEXT,
    frequency           TEXT NOT NULL CHECK (frequency IN ('weekly', 'biweekly', 'monthly', 'bimonthly', 'quarterly', 'semiannual', 'yearly', 'custom')),
    frequency_days      INTEGER,
    payment_method      TEXT,
    account_id          TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    category_id         TEXT REFERENCES categories (id) ON DELETE SET NULL,
    status              TEXT NOT NULL CHECK (status IN ('active', 'completed', 'cancelled', 'refunded')),
    notes               TEXT,
    created_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_purchases_date ON purchases (purchase_date);

CREATE TABLE IF NOT EXISTS purchase_installments (
    id              TEXT PRIMARY KEY,
    purchase_id     TEXT NOT NULL REFERENCES purchases (id) ON DELETE CASCADE,
    number          INTEGER NOT NULL CHECK (number >= 1),
    amount          INTEGER NOT NULL CHECK (amount >= 0),
    due_date        TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('pending', 'paid', 'overdue', 'cancelled')),
    paid_at         TEXT,
    paid_amount     INTEGER NOT NULL DEFAULT 0 CHECK (paid_amount >= 0),
    account_id      TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    transaction_id  TEXT REFERENCES transactions (id) ON DELETE SET NULL,
    UNIQUE (purchase_id, number)
);

CREATE INDEX IF NOT EXISTS idx_installments_purchase ON purchase_installments (purchase_id);
CREATE INDEX IF NOT EXISTS idx_installments_due     ON purchase_installments (due_date, status);

CREATE TABLE IF NOT EXISTS installment_payments (
    id              TEXT PRIMARY KEY,
    installment_id  TEXT NOT NULL REFERENCES purchase_installments (id) ON DELETE CASCADE,
    amount          INTEGER NOT NULL CHECK (amount > 0),
    paid_at         TEXT NOT NULL,
    account_id      TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    transaction_id  TEXT REFERENCES transactions (id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_installment_payments_installment ON installment_payments (installment_id);

CREATE TABLE IF NOT EXISTS subscriptions (
    id                      TEXT PRIMARY KEY,
    name                    TEXT NOT NULL,
    category_id             TEXT REFERENCES categories (id) ON DELETE SET NULL,
    contact_id              TEXT REFERENCES contacts (id) ON DELETE SET NULL,
    url                     TEXT,
    amount                  INTEGER NOT NULL CHECK (amount > 0),
    currency_code           TEXT NOT NULL REFERENCES currencies (code),
    frequency               TEXT NOT NULL CHECK (frequency IN ('weekly', 'biweekly', 'monthly', 'bimonthly', 'quarterly', 'semiannual', 'yearly', 'custom')),
    frequency_days          INTEGER,
    start_date              TEXT NOT NULL,
    next_charge_date        TEXT,
    account_id              TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    payment_method          TEXT,
    responsible_contact_id  TEXT REFERENCES contacts (id) ON DELETE SET NULL,
    status                  TEXT NOT NULL CHECK (status IN ('active', 'paused', 'cancelled', 'expired')),
    cancelled_at            TEXT,
    notes                   TEXT,
    created_at              TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_next ON subscriptions (next_charge_date, status);

CREATE TABLE IF NOT EXISTS subscription_payments (
    id                TEXT PRIMARY KEY,
    subscription_id   TEXT NOT NULL REFERENCES subscriptions (id) ON DELETE CASCADE,
    amount            INTEGER NOT NULL CHECK (amount > 0),
    charged_at        TEXT NOT NULL,
    next_charge_date  TEXT,
    account_id        TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    transaction_id    TEXT REFERENCES transactions (id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_subscription_payments_sub ON subscription_payments (subscription_id);

CREATE TABLE IF NOT EXISTS subscription_shares (
    id                TEXT PRIMARY KEY,
    subscription_id   TEXT NOT NULL REFERENCES subscriptions (id) ON DELETE CASCADE,
    contact_id        TEXT NOT NULL REFERENCES contacts (id) ON DELETE CASCADE,
    share_type        TEXT NOT NULL CHECK (share_type IN ('percent', 'amount')),
    share_value       INTEGER NOT NULL CHECK (share_value >= 0),
    receivable_id     TEXT
);

CREATE TABLE IF NOT EXISTS receivables (
    id              TEXT PRIMARY KEY,
    contact_id      TEXT NOT NULL REFERENCES contacts (id) ON DELETE RESTRICT,
    title           TEXT NOT NULL,
    kind            TEXT NOT NULL CHECK (kind IN ('debt', 'advance', 'refund')),
    total_amount    INTEGER NOT NULL CHECK (total_amount >= 0),
    received_amount INTEGER NOT NULL DEFAULT 0 CHECK (received_amount >= 0),
    currency_code   TEXT NOT NULL REFERENCES currencies (code),
    origin_date     TEXT NOT NULL,
    due_date        TEXT,
    status          TEXT NOT NULL CHECK (status IN ('pending', 'partial', 'paid', 'cancelled', 'overdue')),
    notes           TEXT,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_receivables_due ON receivables (due_date, status);

CREATE TABLE IF NOT EXISTS receivable_payments (
    id              TEXT PRIMARY KEY,
    receivable_id   TEXT NOT NULL REFERENCES receivables (id) ON DELETE CASCADE,
    amount          INTEGER NOT NULL CHECK (amount > 0),
    received_at     TEXT NOT NULL,
    account_id      TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    transaction_id  TEXT REFERENCES transactions (id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_receivable_payments_recv ON receivable_payments (receivable_id);

CREATE TABLE IF NOT EXISTS payables (
    id              TEXT PRIMARY KEY,
    contact_id      TEXT NOT NULL REFERENCES contacts (id) ON DELETE RESTRICT,
    title           TEXT NOT NULL,
    total_amount    INTEGER NOT NULL CHECK (total_amount >= 0),
    paid_amount     INTEGER NOT NULL DEFAULT 0 CHECK (paid_amount >= 0),
    currency_code   TEXT NOT NULL REFERENCES currencies (code),
    created_date    TEXT NOT NULL,
    due_date        TEXT,
    account_id      TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    status          TEXT NOT NULL CHECK (status IN ('pending', 'partial', 'paid', 'cancelled', 'overdue')),
    notes           TEXT,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_payables_due ON payables (due_date, status);

CREATE TABLE IF NOT EXISTS payable_payments (
    id              TEXT PRIMARY KEY,
    payable_id      TEXT NOT NULL REFERENCES payables (id) ON DELETE CASCADE,
    amount          INTEGER NOT NULL CHECK (amount > 0),
    paid_at         TEXT NOT NULL,
    account_id      TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    transaction_id  TEXT REFERENCES transactions (id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_payable_payments_pay ON payable_payments (payable_id);

CREATE TABLE IF NOT EXISTS deposits (
    id                        TEXT PRIMARY KEY,
    contact_id                TEXT REFERENCES contacts (id) ON DELETE SET NULL,
    title                     TEXT NOT NULL,
    amount                    INTEGER NOT NULL CHECK (amount > 0),
    currency_code             TEXT NOT NULL REFERENCES currencies (code),
    created_date              TEXT NOT NULL,
    due_date                  TEXT,
    account_id                TEXT REFERENCES accounts (id) ON DELETE SET NULL,
    destination_institution   TEXT,
    destination_details       TEXT,
    payment_method            TEXT,
    status                    TEXT NOT NULL CHECK (status IN ('pending', 'scheduled', 'completed', 'overdue', 'cancelled')),
    completed_at              TEXT,
    transaction_id            TEXT REFERENCES transactions (id) ON DELETE SET NULL,
    receipt_attachment_id     TEXT,
    receivable_id             TEXT REFERENCES receivables (id) ON DELETE SET NULL,
    notes                     TEXT,
    created_at                TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_deposits_due ON deposits (due_date, status);

CREATE TABLE IF NOT EXISTS wishlist_items (
    id                TEXT PRIMARY KEY,
    name              TEXT,
    url               TEXT,
    store             TEXT,
    estimated_price   INTEGER CHECK (estimated_price IS NULL OR estimated_price >= 0),
    currency_code     TEXT NOT NULL REFERENCES currencies (code),
    added_date        TEXT NOT NULL,
    category_id       TEXT REFERENCES categories (id) ON DELETE SET NULL,
    priority          INTEGER NOT NULL DEFAULT 2 CHECK (priority BETWEEN 1 AND 3),
    savings_goal      INTEGER CHECK (savings_goal IS NULL OR savings_goal >= 0),
    status            TEXT NOT NULL CHECK (status IN ('wanted', 'saving', 'purchased', 'discarded')),
    notes             TEXT,
    purchase_id       TEXT REFERENCES purchases (id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS attachments (
    id          TEXT PRIMARY KEY,
    file_name   TEXT NOT NULL,
    stored_name TEXT NOT NULL UNIQUE,
    mime_type   TEXT,
    size_bytes  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);

-- Categorías por defecto (no son datos ficticios: son la plantilla inicial editable).
INSERT OR IGNORE INTO categories (id, name, kind, color, archived, created_at) VALUES
    ('cat-alimentacion',   'Alimentación',      'expense', '#C2703D', 0, '2024-01-01T00:00:00Z'),
    ('cat-transporte',     'Transporte',        'expense', '#4C7BA8', 0, '2024-01-01T00:00:00Z'),
    ('cat-vivienda',       'Vivienda',          'expense', '#6C6F86', 0, '2024-01-01T00:00:00Z'),
    ('cat-servicios',      'Servicios básicos', 'expense', '#3F8A7A', 0, '2024-01-01T00:00:00Z'),
    ('cat-salud',          'Salud',             'expense', '#C05A6B', 0, '2024-01-01T00:00:00Z'),
    ('cat-educacion',      'Educación',         'expense', '#7A62A8', 0, '2024-01-01T00:00:00Z'),
    ('cat-entretenimiento','Entretenimiento',   'expense', '#C08A3E', 0, '2024-01-01T00:00:00Z'),
    ('cat-compras',        'Compras online',   'expense', '#B0674F', 0, '2024-01-01T00:00:00Z'),
    ('cat-suscripciones',  'Suscripciones',     'expense', '#8A6F9C', 0, '2024-01-01T00:00:00Z'),
    ('cat-tecnologia',     'Tecnología',        'expense', '#4C7BA8', 0, '2024-01-01T00:00:00Z'),
    ('cat-ropa',           'Ropa',              'expense', '#B0768C', 0, '2024-01-01T00:00:00Z'),
    ('cat-ahorro',         'Ahorro',            'expense', '#3F8A7A', 0, '2024-01-01T00:00:00Z'),
    ('cat-salud-otros',    'Otros gastos',      'expense', '#7B7F8A', 0, '2024-01-01T00:00:00Z'),
    ('cat-laboral',        'Ingresos laborales','income',  '#2F7A5A', 0, '2024-01-01T00:00:00Z'),
    ('cat-ingresos-otros', 'Otros ingresos',    'income',  '#3F8A7A', 0, '2024-01-01T00:00:00Z');
