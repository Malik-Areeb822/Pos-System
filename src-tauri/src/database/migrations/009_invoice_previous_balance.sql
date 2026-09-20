-- 009_invoice_previous_balance.sql
-- Carry forward a customer's outstanding balance onto each new invoice.

ALTER TABLE invoices ADD COLUMN previous_balance INTEGER NOT NULL DEFAULT 0;
