-- Track which invoice absorbed a previous invoice's outstanding balance.
-- When Invoice B picks up Invoice A's balance via previous_balance,
-- Invoice A.carried_to_invoice_id = B.id.

ALTER TABLE invoices ADD COLUMN carried_to_invoice_id TEXT REFERENCES invoices(id);
CREATE INDEX idx_invoices_carried_to ON invoices(carried_to_invoice_id);
