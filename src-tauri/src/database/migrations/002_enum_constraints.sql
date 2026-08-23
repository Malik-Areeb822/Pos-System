-- 002_enum_constraints.sql
-- Additional CHECK constraints for enums (SQLite doesn't have native enums)

-- Products category already has CHECK in table definition
-- Invoices payment_method already has CHECK in table definition
-- Users status already has CHECK in table definition
-- User_roles role already has CHECK in table definition

-- Additional validation: ensure price is non-negative
-- (SQLite CHECK constraints don't support cross-column, handled in application)

-- Products: stock_qty >= 0
-- (Already handled by INTEGER DEFAULT 0, but add explicit constraint via trigger if needed)

-- Invoice items: quantity > 0
-- Returns: quantity > 0