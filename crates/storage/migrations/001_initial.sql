-- RustCash initial schema
-- All monetary amounts are stored as TEXT in decimal notation to avoid float precision loss.
-- UUIDs are stored as TEXT (SQLite) or UUID (PostgreSQL).

CREATE TABLE IF NOT EXISTS books (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    description  TEXT,
    default_commodity_id TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    modified_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commodities (
    id          TEXT PRIMARY KEY,
    book_id     TEXT NOT NULL REFERENCES books(id),
    namespace   TEXT NOT NULL,
    mnemonic    TEXT NOT NULL,
    name        TEXT NOT NULL,
    fraction    INTEGER NOT NULL DEFAULT 100,
    notes       TEXT,
    created_at  TEXT NOT NULL,
    UNIQUE (book_id, namespace, mnemonic)
);

CREATE TABLE IF NOT EXISTS accounts (
    id              TEXT PRIMARY KEY,
    book_id         TEXT NOT NULL REFERENCES books(id),
    parent_id       TEXT REFERENCES accounts(id),
    name            TEXT NOT NULL,
    full_name       TEXT NOT NULL,
    account_type    TEXT NOT NULL,
    commodity_id    TEXT NOT NULL REFERENCES commodities(id),
    description     TEXT,
    placeholder     INTEGER NOT NULL DEFAULT 0,
    hidden          INTEGER NOT NULL DEFAULT 0,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    modified_at     TEXT NOT NULL,
    deleted_at      TEXT
);

CREATE TABLE IF NOT EXISTS transactions (
    id          TEXT PRIMARY KEY,
    book_id     TEXT NOT NULL REFERENCES books(id),
    date        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    notes       TEXT,
    tags        TEXT NOT NULL DEFAULT '[]',  -- JSON array of strings
    entered_at  TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    deleted_at  TEXT
);

CREATE TABLE IF NOT EXISTS splits (
    id              TEXT PRIMARY KEY,
    transaction_id  TEXT NOT NULL REFERENCES transactions(id),
    account_id      TEXT NOT NULL REFERENCES accounts(id),
    amount          TEXT NOT NULL,   -- decimal string, never float
    value           TEXT NOT NULL,   -- decimal string, in transaction commodity
    commodity_id    TEXT NOT NULL REFERENCES commodities(id),
    reconcile_state TEXT NOT NULL DEFAULT 'unreconciled',
    reconcile_date  TEXT,
    memo            TEXT,
    action          TEXT,
    lot_id          TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS prices (
    id           TEXT PRIMARY KEY,
    book_id      TEXT NOT NULL REFERENCES books(id),
    commodity_id TEXT NOT NULL REFERENCES commodities(id),
    currency_id  TEXT NOT NULL REFERENCES commodities(id),
    date         TEXT NOT NULL,
    value        TEXT NOT NULL,  -- decimal string
    source       TEXT NOT NULL DEFAULT 'user',
    created_at   TEXT NOT NULL
);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_accounts_book     ON accounts(book_id);
CREATE INDEX IF NOT EXISTS idx_accounts_parent   ON accounts(parent_id);
CREATE INDEX IF NOT EXISTS idx_transactions_book ON transactions(book_id);
CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(book_id, date);
CREATE INDEX IF NOT EXISTS idx_splits_account    ON splits(account_id);
CREATE INDEX IF NOT EXISTS idx_splits_txn        ON splits(transaction_id);
CREATE INDEX IF NOT EXISTS idx_prices_commodity  ON prices(commodity_id, date);
