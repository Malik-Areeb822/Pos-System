# Fix Plan — 51 Compilation Errors

## Phase 1: Quick Wins (5 mins) ✅ DONE
- [x] `src/database/mod.rs` — Remove `pub mod migrations;` (line 3)
- [x] `src/lib.rs:77` — Remove `commands::invoices::print_receipt,` (duplicate)
- [x] `src/lib.rs:21` — Fix autostart: `.plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::default(), None))`
- [x] `src/lib.rs` — Add `use tauri::Manager;` import
- [x] `src/lib.rs:23` — Fix single-instance: use `app.webview_windows()` instead of `get_webview_window`
- [x] `src/lib.rs:32` — Fix `.manage()` with Manager import

## Phase 2: Repository Struct Field Fixes (15 mins)
Change nullable fields to `Option<String>` in:
- [ ] `src/repositories/products.rs` — sku, description, color, size, finish, image_url
- [ ] `src/repositories/customers.rs` — phone, email, address
- [ ] `src/repositories/invoices.rs` — customer_id, notes, delivery_date, InvoiceItem.product_id
- [ ] `src/repositories/returns.rs` — product_id
- [ ] `src/repositories/cashiers.rs` — phone, employee_id, id (unwrap in construction)
- [ ] `src/repositories/users.rs` — phone, employee_id, id (unwrap in construction)
- [ ] `src/repositories/invoices.rs` — Add `use chrono::Datelike;` for `.year()`

## Phase 3: Borrow Checker & Auth Fixes (5 mins)
- [ ] `src/commands/returns.rs:34,38` — Clone `auth_header` before first use
- [ ] `src/auth/middleware.rs:38` — Fix `try_state().map_err()` → `try_state().ok_or_else(|| ...)`

## Phase 4: sqlx Query Result Fixes (10 mins)
- [ ] `src/commands/reports.rs:114-118` — Remove `.unwrap_or(0)` on i64 (COALESCE handles NULL)
- [ ] `src/commands/reports.rs:139` — `id: r.id.unwrap_or_default()`
- [ ] `src/commands/reports.rs:147` — Remove `.unwrap_or(0)` on value
- [ ] `src/services/pos.rs:8` — Remove arg from `repo.list(None)` → `repo.list()`

## Phase 5: genpdf Rewrite (10 mins)
- [ ] `src/services/invoice_pdf.rs` — Rewrite using genpdf 0.2 API (no font_size, render takes 1 arg)

## Phase 6: rusb 0.9 API Fixes (10 mins)
- [ ] `src/services/print.rs:42` — Match on `Result` from `open_device_with_vid_pid`
- [ ] `src/services/print.rs:220` — Add `Duration` to `read_languages()`
- [ ] `src/services/print.rs:222,225,228` — Add `Duration` 3rd arg to `read_*_string()`

## Phase 7: Autostart Plugin Fix (5 mins)
- [ ] `src/autostart.rs` — Remove `set_app_name()`, fix ManagerExt usage, remove unused imports

## Phase 8: Database Connection Fix (2 mins)
- [ ] `src/database/connection.rs:54-56` — Remove `.cloned()`, fix `try_state()` handling

## Phase 1: Quick Wins (5 mins) ✅ DONE
- [x] `src/database/mod.rs` — Remove `pub mod migrations;` (line 3)
- [x] `src/lib.rs:77` — Remove `commands::invoices::print_receipt,` (duplicate)
- [x] `src/lib.rs:21` — Fix autostart: `.plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::default(), None))`
- [x] `src/lib.rs` — Add `use tauri::Manager;` import
- [x] `src/lib.rs:23` — Fix single-instance: use `app.webview_windows()` instead of `get_webview_window`
- [x] `src/lib.rs:32` — Fix `.manage()` with Manager import

## Phase 2: Repository Struct Field Fixes (15 mins) ✅ DONE
- [x] `src/repositories/products.rs` — `description` → `Option<String>` (and other nullable fields)
- [x] `src/repositories/customers.rs` — `phone`, `email`, `address` → `Option<String>`
- [x] `src/repositories/invoices.rs` — `customer_id`, `notes`, `delivery_date`, `InvoiceItem.product_id` → `Option<String>`; add `use chrono::Datelike;`
- [x] `src/repositories/returns.rs` — `product_id` → `Option<String>`
- [x] `src/repositories/cashiers.rs` — `phone`, `employee_id` → `Option<String>`; `id.expect()` in construction
- [x] `src/repositories/users.rs` — `phone`, `employee_id` → `Option<String>`; `id.expect()` in construction

## Phase 3: Borrow Checker & Auth Fixes (5 mins) ✅ DONE
- [x] `src/commands/returns.rs:34,38` — Clone `auth_header` before first use
- [x] `src/auth/middleware.rs:38` — Fix `try_state().map_err()` → `try_state().ok_or_else(|| ...)`

## Phase 4: sqlx Query Result Fixes (10 mins) ✅ DONE
- [x] `src/commands/reports.rs:114-118` — Remove `.unwrap_or(0)` on i64 (COALESCE handles NULL)
- [x] `src/commands/reports.rs:139` — `id: r.id.unwrap_or_default()`
- [x] `src/commands/reports.rs:147` — Remove `.unwrap_or(0)` on value
- [x] `src/services/pos.rs:8` — Remove arg from `repo.list(None)` → `repo.list()`

## Phase 5: genpdf Rewrite (10 mins) ✅ DONE
- [x] `src/services/invoice_pdf.rs` — Rewrite using genpdf 0.2 API (no `font_size()`, `Line`, `Alignment`, `Break::1`; use `doc.push()`, `doc.render(&mut file)`)

## Phase 6: rusb 0.9 API Fixes (10 mins) ✅ DONE
- [x] `src/services/print.rs:42` — Match on `Result` from `open_device_with_vid_pid`; use `if let Some`
- [x] `src/services/print.rs:220` — Add `Duration` to `read_languages()`
- [x] `src/services/print.rs:222,225,228` — Add `Duration` 3rd arg to `read_*_string()`

## Phase 7: Autostart Plugin Fix (5 mins) ✅ DONE
- [x] `src/autostart.rs` — Remove `set_app_name()`, fix ManagerExt usage, remove unused imports

## Phase 8: Database Connection Fix (2 mins) ✅ DONE
- [x] `src/database/connection.rs:54-56` — Fix `get_pool` with `ok_or_else` + `map(|state| state.inner().clone())`

## Phase 9: Temporary Value Fixes (5 mins) ✅ DONE
- [x] All repositories — Remove `&` from `sqlx::query!`/`query_scalar!` bind arguments (pass owned values directly); bind `Utc::now()` to `let` before queries

---

## Execution Order
1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9

Run `cargo check` after each phase to verify.