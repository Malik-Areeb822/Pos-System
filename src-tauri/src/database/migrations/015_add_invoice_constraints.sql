-- Defense-in-depth CHECK constraints for invoice and purchase amounts.
-- SQLite does not support ALTER TABLE ADD CONSTRAINT, so we recreate
-- the tables with the constraints included.

-- === invoices ===
CREATE TABLE invoices_new (
    id TEXT PRIMARY KEY NOT NULL,
    invoice_no TEXT NOT NULL UNIQUE,
    customer_id TEXT REFERENCES customers(id),
    customer_name TEXT NOT NULL DEFAULT 'Walk-in',
    subtotal INTEGER NOT NULL DEFAULT 0,
    discount INTEGER NOT NULL DEFAULT 0,
    total INTEGER NOT NULL DEFAULT 0,
    previous_balance INTEGER NOT NULL DEFAULT 0,
    amount_paid INTEGER NOT NULL DEFAULT 0,
    payment_method TEXT NOT NULL DEFAULT 'cash',
    notes TEXT,
    delivery_date TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    carried_to_invoice_id TEXT,
    CHECK (discount >= 0 AND discount <= subtotal),
    CHECK (amount_paid >= 0 AND amount_paid <= total)
);

INSERT INTO invoices_new (id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id)
SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id
FROM invoices;

DROP TABLE invoices;
ALTER TABLE invoices_new RENAME TO invoices;

CREATE UNIQUE INDEX IF NOT EXISTS idx_invoices_invoice_no ON invoices(invoice_no);
CREATE INDEX IF NOT EXISTS idx_invoices_customer_id ON invoices(customer_id);
CREATE INDEX IF NOT EXISTS idx_invoices_created_at ON invoices(created_at);

-- === supplier_purchases ===
CREATE TABLE supplier_purchases_new (
    id TEXT PRIMARY KEY NOT NULL,
    purchase_no TEXT NOT NULL UNIQUE,
    supplier_id TEXT REFERENCES suppliers(id),
    supplier_name TEXT NOT NULL DEFAULT 'Unknown',
    subtotal INTEGER NOT NULL DEFAULT 0,
    discount INTEGER NOT NULL DEFAULT 0,
    total INTEGER NOT NULL DEFAULT 0,
    amount_paid INTEGER NOT NULL DEFAULT 0,
    payment_method TEXT NOT NULL DEFAULT 'cash',
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (discount >= 0 AND discount <= subtotal),
    CHECK (amount_paid >= 0 AND amount_paid <= total)
);

INSERT INTO supplier_purchases_new (id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, amount_paid, payment_method, notes, created_at, updated_at)
SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, amount_paid, payment_method, notes, created_at, updated_at
FROM supplier_purchases;

DROP TABLE supplier_purchases;
ALTER TABLE supplier_purchases_new RENAME TO supplier_purchases;

CREATE UNIQUE INDEX IF NOT EXISTS idx_supplier_purchases_purchase_no ON supplier_purchases(purchase_no);
CREATE INDEX IF NOT EXISTS idx_supplier_purchases_supplier_id ON supplier_purchases(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_purchases_created_at ON supplier_purchases(created_at);
