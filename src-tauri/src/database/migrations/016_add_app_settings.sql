-- 016_add_app_settings.sql
-- Application settings table for system lock and future config
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed the lock flag (0 = unlocked, 1 = locked)
-- The value column stores "{hmac_signature}" for tamper detection
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('system_lock', '0');
