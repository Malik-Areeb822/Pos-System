-- 003_invoice_counter.sql
-- Invoice counter table for sequential invoice numbers (INV-YYYY-NNNN)

-- Table already created in 001, just initialize if empty
INSERT OR IGNORE INTO invoice_counter (id, year, sequence) VALUES (1, CAST(strftime('%Y', 'now') AS INTEGER), 0);

-- Trigger to reset sequence on year change
CREATE TRIGGER IF NOT EXISTS reset_invoice_counter_year
AFTER UPDATE OF year ON invoice_counter
WHEN NEW.year != OLD.year
BEGIN
    UPDATE invoice_counter SET sequence = 0 WHERE id = 1;
END;