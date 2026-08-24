-- 006_product_company.sql
-- Optional supplier/company name, surfaced for sanitary-ware products in the UI.
-- Nullable for all rows: other categories simply leave it empty.
ALTER TABLE products ADD COLUMN company TEXT;
