-- 018_customer_unique.sql
-- Deduplicate existing customers and enforce unique constraint on name + phone.
--
-- Merge rules:
-- 1. Matching key: lower(trim(name)) and trim(COALESCE(phone, ''))
-- 2. Keeper: earliest created_at (tie-breaker: lowest rowid)
-- 3. Invoices referencing duplicate rows are updated to reference the keeper
-- 4. Keeper outstanding_balance becomes the SUM of all merged rows
-- 5. Duplicate non-keeper rows are deleted
-- 6. Unique index created on (lower(trim(name)), trim(COALESCE(phone, '')))

DROP TABLE IF EXISTS _customer_merge_map;

CREATE TABLE _customer_merge_map AS
WITH ranked AS (
    SELECT
        id,
        lower(trim(name)) AS norm_name,
        trim(COALESCE(phone, '')) AS norm_phone,
        ROW_NUMBER() OVER (
            PARTITION BY lower(trim(name)), trim(COALESCE(phone, ''))
            ORDER BY created_at ASC, rowid ASC
        ) AS rn
    FROM customers
)
SELECT
    r.id,
    k.id AS keeper_id,
    r.rn
FROM ranked r
JOIN ranked k
  ON k.norm_name = r.norm_name
 AND k.norm_phone = r.norm_phone
 AND k.rn = 1;

-- Repoint invoices referencing duplicate customer records to the keeper
UPDATE invoices
SET customer_id = (
    SELECT m.keeper_id
    FROM _customer_merge_map m
    WHERE m.id = invoices.customer_id
)
WHERE customer_id IN (
    SELECT id
    FROM _customer_merge_map
    WHERE rn > 1
);

-- Sum outstanding balances of all merged rows onto the keeper
UPDATE customers
SET outstanding_balance = (
    SELECT COALESCE(SUM(c2.outstanding_balance), 0)
    FROM _customer_merge_map m
    JOIN customers c2 ON c2.id = m.id
    WHERE m.keeper_id = customers.id
)
WHERE id IN (
    SELECT keeper_id
    FROM _customer_merge_map
    WHERE rn > 1
);

-- Delete non-keeper duplicate rows
DELETE FROM customers
WHERE id IN (
    SELECT id
    FROM _customer_merge_map
    WHERE rn > 1
);

DROP TABLE IF EXISTS _customer_merge_map;

-- Enforce uniqueness on trimmed, lowercase name and trimmed phone
CREATE UNIQUE INDEX IF NOT EXISTS ux_customers_name_phone
ON customers(lower(trim(name)), trim(COALESCE(phone, '')));
