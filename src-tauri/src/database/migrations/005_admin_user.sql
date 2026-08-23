-- 005_admin_user.sql
-- Admin user creation logic (handled in Rust auth::register)
-- This migration ensures the table structure supports the "first user = admin" flow

-- The users table from 001 already has the required structure
-- Rust code in auth::register() will:
-- 1. Check if any user exists (SELECT COUNT(*) FROM users)
-- 2. If count = 0: assign 'admin' role + status 'approved'
-- 3. If count > 0: assign 'cashier' role + status 'pending'

-- No additional SQL needed - logic is in application layer
-- This file exists as a placeholder for the migration sequence