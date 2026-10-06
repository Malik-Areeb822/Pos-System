-- 017_supplier_purchase_carry_forward.sql
-- Carry a supplier's outstanding balance onto each new purchase order,
-- mirroring 009 (invoices.previous_balance) and 014 (invoices.carried_to_invoice_id).

ALTER TABLE supplier_purchases ADD COLUMN previous_balance INTEGER NOT NULL DEFAULT 0;
ALTER TABLE supplier_purchases ADD COLUMN carried_to_purchase_id TEXT REFERENCES supplier_purchases(id);
CREATE INDEX IF NOT EXISTS idx_supplier_purchases_carried_to ON supplier_purchases(carried_to_purchase_id);
