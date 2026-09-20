-- Add purchase_price to products (cost price for profit tracking)
-- and to invoice_items (locked at time of sale for historical accuracy)
ALTER TABLE products ADD COLUMN purchase_price INTEGER NOT NULL DEFAULT 0;
ALTER TABLE invoice_items ADD COLUMN purchase_price INTEGER NOT NULL DEFAULT 0;
