-- 007_suppliers.sql — Supplier ledger tables

-- 1. Suppliers (contact directory + payables balance)
CREATE TABLE IF NOT EXISTS suppliers (
    id                  TEXT PRIMARY KEY NOT NULL,
    name                TEXT NOT NULL,
    phone               TEXT,
    email               TEXT,
    company             TEXT,
    address             TEXT,
    notes               TEXT,
    outstanding_balance INTEGER NOT NULL DEFAULT 0,
    created_at          TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_suppliers_name ON suppliers(name);
CREATE INDEX IF NOT EXISTS idx_suppliers_company ON suppliers(company);

-- 2. Supplier Purchases (purchase order master — analogous to invoices)
CREATE TABLE IF NOT EXISTS supplier_purchases (
    id              TEXT PRIMARY KEY NOT NULL,
    purchase_no     TEXT UNIQUE NOT NULL,
    supplier_id     TEXT NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    supplier_name   TEXT NOT NULL,
    subtotal        INTEGER NOT NULL DEFAULT 0,
    discount        INTEGER NOT NULL DEFAULT 0,
    total           INTEGER NOT NULL DEFAULT 0,
    amount_paid     INTEGER NOT NULL DEFAULT 0,
    payment_method  TEXT NOT NULL DEFAULT 'cash'
                    CHECK (payment_method IN ('cash', 'bank', 'credit')),
    notes           TEXT,
    created_by      TEXT REFERENCES users(id),
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_supplier_purchases_supplier
    ON supplier_purchases(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_purchases_created
    ON supplier_purchases(created_at DESC);

-- 3. Supplier Purchase Items (line items — analogous to invoice_items)
CREATE TABLE IF NOT EXISTS supplier_purchase_items (
    id              TEXT PRIMARY KEY NOT NULL,
    purchase_id     TEXT NOT NULL REFERENCES supplier_purchases(id) ON DELETE CASCADE,
    description     TEXT NOT NULL,
    quantity        INTEGER NOT NULL DEFAULT 1,
    unit            TEXT NOT NULL DEFAULT 'pcs',
    unit_price      INTEGER NOT NULL DEFAULT 0,
    line_total      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_supplier_purchase_items_purchase
    ON supplier_purchase_items(purchase_id);

-- 4. Purchase number sequence (analogous to invoice_counter)
CREATE TABLE IF NOT EXISTS purchase_counter (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    year     INTEGER NOT NULL,
    sequence INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO purchase_counter (id, year, sequence) VALUES (1, CAST(strftime('%Y', 'now') AS INTEGER), 0);