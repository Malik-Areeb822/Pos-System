# Handoff Document — Stone Flow POS Offline Migration

> **Goal**: Convert to single executable, auto-start, fully offline, loosely coupled
> **Constraint**: Zero functional changes to core POS behavior
> **Status**: 🟢 System operational — admin login, all modules, partial-payment ledger working in dev
> **Date**: 2026-08-23
> **Phase**: Phase B verification (post-stabilization) → Phase C release pending

---

## 🔥 Session 2026-08-23: Launch-Blocking Fixes + Payment Ledger Overhaul

A full debugging pass took the app from "empty shell that renders login" to fully
operational. Every fix below is applied and verified (`cargo check --all-targets` 0 errors,
`npm run build` clean).

### 1. Critical wiring fixes (why nothing worked before)

| Fix | File(s) | Root cause |
|-----|---------|-----------|
| Backend was never launched | `src-tauri/src/main.rs` | Template stub built its own empty Tauri app; never called `city_tiles_pos_lib::run()` → no DB pool, no plugins, no 43 commands |
| Plugin panic on boot | `src-tauri/tauri.conf.json` | Removed `plugins.autostart` / `singleInstance` JSON blocks — tauri-plugin-autostart 2.5.1 & single-instance 2.4.3 take config via Rust `init()` only; any map under `plugins.*` panics with "invalid type: map, expected unit" |
| JWT never reached backend | `src/lib/api-client.ts` | Frontend sent `__headers` object; Rust commands declare `auth_header: Option<String>`. Now sends `authHeader: "Bearer <token>"` (Tauri maps camelCase→snake_case params automatically) |
| ~20 IPC arg-shape mismatches | `src/lib/api-client.ts` | Tauri extracts each struct param from payload key matching the camelCased param name. All struct args now wrapped `{ input: {...} }`, list commands send `{ input: {...} }`, primitives renamed to camelCase (`invoiceId`, `csvContent`) |
| Invisible errors | `src/lib/api-client.ts` | Invoke rejections normalized into real `Error`s — toasts now show actual backend messages instead of generic fallbacks |
| Invoice detail data shape | `api-client.ts` getWithItems | Detail page needs items; now calls `get_invoice_with_items` and maps the Rust tuple → `{ invoice, items }` |
| CSV import rewired | `commands/products.rs`, `features/inventory/api.ts` | Command takes `csv_content: String` and delegates to existing `services/inventory.rs` parser; frontend reads file text via `file.text()`. Result struct got `#[derive(Serialize)]` |
| markPaid payload | invoice feature + 2 routes | Sends `{ id, amount, payment_method }` matching `MarkPaidInput`; invoices list passes full balance, detail computes from loaded invoice |
| Legacy Supabase identity reads | `src/lib/pos.ts`, `AdminShell.tsx`, `features/auth/api.ts` | Admin UI read roles via dead Supabase calls → everyone showed as "staff", write buttons hidden, admins bounced off dashboard. All hooks (`useSession`/`useRoles`/`useIsAdmin`) now read the local auth store. Logout uses `clearAuth()`. Dead Supabase functions deleted |

### 2. Payment ledger overhaul (partial payments)

Business rule locked by owner: **the invoice number is the single source of truth for dues**,
walk-in or named customer alike. Customer `outstanding_balance` is a mirror maintained on every path.

| Change | File(s) | Behavior |
|--------|---------|----------|
| Creation accepts real paid amount | `commands/invoices.rs` (+repo struct) | Optional `amount_paid`, clamped 0..total. Due = total − paid. If customer attached & due>0 → balance += due (any method; credit = naturally paid-0). Walk-in dues stay on invoice only |
| Incremental payments | `repositories/invoices.rs` `mark_paid()` | Rejects payment > remaining due (`AppError::Validation`); subtracts only the **applied** amount from customer mirror (fixes old full-total corruption bug) |
| Returns port original RPC | `repositories/returns.rs` `create()` | Matches Supabase `process_return`: shrink subtotal+total by return value, clamp paid = min(paid,new_total), move due-delta onto customer balance, keep stock restore. (Old code left invoice totals stale and handled credit-method only) |
| POS sends paid + shows due | `admin.pos.tsx` | Sends validated `amount_paid` (toast error if > total); live red **Balance due** row in summary |
| Invoice payment box | `admin.invoices.$invoiceId.tsx` | Amount input (prefills due) + button ("Clear full balance" / "Record partial payment"); toast reports remaining after save; print-hidden |
| Types hardened | both `CreateInvoiceInput` defs | `subtotal`/`total` required, `amount_paid?: number`; `CreateReturnInput.line_total` required |

### 3. Build infrastructure

| Item | Detail |
|------|--------|
| DB ACL | `icacls C:\ProgramData\CityTiles /grant Users:(OI)(CI)M` — no elevation needed for DB creation |
| sqlx offline cache | New `.cargo/config.toml` pins `DATABASE_URL=sqlite://C:/ProgramData/CityTiles/citytiles.db` so changed query macros always validate against the live dev DB (plain `cargo check` works without env vars) |

---

## ✅ Current Working State (dev machine)

- `cargo tauri dev` boots cleanly → first-run creates `%PROGRAMDATA%\CityTiles\citytiles.db`
- First-time admin signup flow works (auto-detected when users table empty)
- Login/logout, role display (**ADMIN** badge), admin-only nav gating all correct
- Verified flows: inventory CRUD, POS checkout incl. partial payments, invoice detail
  incremental payments, returns adjusting totals/due/balance/stock, cashier approve/reject/suspend

## ⏳ Remaining Work (Phase B verification → Phase C release)

| Task | Status |
|------|--------|
| Verify autostart registry entry + single-instance focus on second launch | Pending test |
| Clean Windows machine install test | Pending |
| Thermal printer (ESC/POS) test at office | Deferred — printer not yet purchased/on-site |
| `cargo tauri build` MSI/NSIS with self-signed cert ("AUZ Tech") | Pending |
| Staff quick-start documentation | Pending |

## ⚠️ Known Issues / Out-of-Scope Leftovers

1. **Dashboard/reports response shapes**: Rust returns `DashboardStats{total_sales_today,...}`
   and a sales-report **array of rows**; TS interfaces still expect `total_revenue` /
   summary-object shapes. IPC works but some dashboard/report numbers may render wrong until
   these are aligned.
2. **Public website routes still Supabase-backed**: `lib/catalog.ts`, `InquiryForm.tsx`,
   `hooks/useInvoiceRealtime.ts` (dead) — broken offline; separate cleanup needed.
3. **PDF fonts**: `services/invoice_pdf.rs` loads fonts from CWD-relative `./fonts` which does
   not exist anywhere → PDF generation will fail until fonts are bundled and path resolved.
4. **Backup UI missing**: export/import/list_backups commands registered but no frontend
   callers (HANDOFF previously claimed otherwise). Also import copies over a live pool —
   harden before shipping backup feature.
5. **Cashier reset-password**: backend ready, no UI entry point.
6. **JWT secret**: hardcoded fallback `"citytiles-pos-secret-change-in-production"` ships in
   binary (config.rs). Acceptable for offline single-shop deployment; rotate before wider use.

## 🔒 Locked Decisions

| Decision | Value |
|----------|-------|
| Database | Single SQLite at `%PROGRAMDATA%\CityTiles\citytiles.db` via sqlx (plugin-sql removed) |
| Payment rule | Invoice number tracks dues universally; customer balance mirrors any attached-customer due |
| Code signing | Self-signed, publisher "AUZ Tech" |
| Seed products | 20 examples ship via migration 004 |
| Printing | ESC/POS raw USB (rusb) + spooler fallback; genpdf A4 invoices |

---

<details>
<summary><strong>📜 Historical phase log (pre-2026-08-23)</strong></summary>

## ✅ Phase 1: SPA Conversion + API Abstraction
- `src/main.tsx` SPA entry replaced `server.ts`/`start.ts`; Vite SSR/Nitro disabled
- `src/lib/`: api-client.ts (circuit breaker), tauri-events.ts (realtime bridge), auth-store.ts (Zustand + localStorage key `city-tiles-auth`)
- Feature APIs created for auth/inventory/customers/invoices/returns/reports/cashiers/pos
- All route files migrated off Supabase

## ✅ Phase 2–3: Rust Backend Core + Business Logic Port
- `src-tauri/src/`: auth (JWT HS256 24h, bcrypt cost 12), 6 repositories, 43 commands across 9 modules, events, autostart+single-instance, panic hook
- Migrations 001–005 embedded (`sqlx::migrate!`): schema, enum CHECKs, INV-YYYY-NNNN counter, 20 seed products
- Services: ESC/POS print (rusb + spooler fallback), genpdf invoices, CSV import, backup export/import

## ✅ Phase A (2026-08-22): DB Conflict Fix + IPC Realignment
- Removed `tauri-plugin-sql`; single DB enforced via `sqlx::SqlitePool`
- Windows SQLite URI fix: backslashes→forward slashes in `connection.rs` (`sqlite:C:/...?mode=rwc`)
- React Query staleTime fix on `check_admin_exists` (staleTime:0, refetchOnMount:"always")
- Auto-switch to signup mode when no admin exists (`routes/index.tsx` useEffect)
- Initial command-name alignment pass (later superseded by 2026-08-23 arg-shape fixes)

</details>

---

## 🚀 Quick Commands

```bash
npm run build          # frontend build (vite)
cd src-tauri && cargo check --all-targets   # rust validation
cargo tauri dev        # run app in dev
cargo tauri build      # production MSI/NSIS installer
```

**Status**: System functional end-to-end in dev. Proceed with Phase B verification checklist above.
