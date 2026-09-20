-- 013_add_hardware_category.sql
-- SQLite doesn't support ALTER TABLE to modify CHECK constraints
-- Must rebuild the products table to add 'hardware' to the category CHECK

PRAGMA foreign_keys = OFF;

-- 1. Create new table with updated CHECK constraint
CREATE TABLE products_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sku TEXT UNIQUE,
    category TEXT NOT NULL CHECK (category IN ('marble', 'tiles', 'chips', 'sanitary', 'hardware')),
    description TEXT NOT NULL DEFAULT '',
    color TEXT,
    size TEXT,
    finish TEXT,
    unit TEXT NOT NULL DEFAULT 'pcs',
    price INTEGER NOT NULL DEFAULT 0,
    purchase_price INTEGER,
    pieces_per_carton INTEGER,
    area_per_tile REAL,
    stock_qty INTEGER NOT NULL DEFAULT 0,
    low_stock_threshold INTEGER NOT NULL DEFAULT 5,
    image_url TEXT,
    is_published INTEGER NOT NULL DEFAULT 1,
    company TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 2. Copy all existing data (explicit column mapping due to order differences)
INSERT INTO products_new (id, name, sku, category, description, color, size, finish, unit, price, purchase_price, pieces_per_carton, area_per_tile, stock_qty, low_stock_threshold, image_url, is_published, company, created_at, updated_at)
SELECT id, name, sku, category, description, color, size, finish, unit, price, purchase_price, pieces_per_carton, area_per_tile, stock_qty, low_stock_threshold, image_url, is_published, company, created_at, updated_at FROM products;

-- 3. Drop old table
DROP TABLE products;

-- 4. Rename new table
ALTER TABLE products_new RENAME TO products;

-- 5. Recreate indexes
CREATE INDEX IF NOT EXISTS idx_products_category ON products(category);
CREATE INDEX IF NOT EXISTS idx_products_sku ON products(sku);
CREATE INDEX IF NOT EXISTS idx_products_published ON products(is_published);

PRAGMA foreign_keys = ON;
