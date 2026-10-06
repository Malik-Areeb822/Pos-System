# Moon Pipe POS — Pre-Deployment Fix Plan

Status: **IMPLEMENTED (2026-10-06)** — Phase 1 ✅, Phase 2 (2.1–2.7) ✅, Phase 3 ✅, Phase 4 ✅. Gates 1–6 clean, gate 7 WAL smoke clean. Remaining: 4 manual UI smoke checks + a full restore end-to-end (see D below). See HANDOFF.md session 2026-10-06.
Scope decisions (user-confirmed): critical fixes + cheap hardening only.
No UI pagination/virtualization. No backup pruning.
Phone matching = trimmed, not digit-normalized.

---

## A. Problem summary (audited findings)

| # | Severity | Problem | Root cause / evidence |
|---|----------|---------|----------------------|
| 1 | Critical | Sales Reports totals/counts wrong | `useInvoices()` → `list_invoices` default `LIMIT 50` (`commands/invoices.rs:54`), consumed by `admin.reports.tsx:124` |
| 2 | Critical | Dashboard "Sales today"/"Last 7 days" wrong | `admin.index.tsx:44-59` computes from the same 50-row window |
| 3 | Critical | Duplicate customers can be created | `repositories/customers.rs:61-67` plain INSERT, no unique index |
| 4 | Critical | Customer "Orders" count + history wrong | `admin.customers.tsx:113,132` filter the 50-row window |
| 5 | High | No pagination anywhere | `list` unbounded (`repositories/customers.rs:39`), invoices page capped but no paging UI |
| 6 | High | No WAL journal mode | sqlx 0.8.6 does not set journal_mode by default (`.pragma` code path in sqlx-sqlite `options/mod.rs:177-181`) |
| 7 | High | Non-sargable / UTC-wrong `date(created_at)` in dashboard | `commands/reports.rs:54-101`; wrong every day 00:00–05:00 PKT |
| 8 | High | Unbounded backup growth | `services/retention.rs` purges old invoices but never prunes backup files (out of scope per user) |
| 9 | Medium | Excel TOTAL row double-counts carried balances | `admin.reports.tsx:99` sums gross `total` |
| 10 | Medium | POS walk-in can create duplicates / cache staleness | `admin.pos.tsx:127-137` own lookup misses format variants; `["pos-customers"]` never invalidated |

### Verified correct — do NOT touch
- net revenue = `total − previous_balance` (≡ `subtotal − discount`)
- profit-vs-returns formula (single deduction, exact)
- outstanding credit (matches live DB)
- invoice numbering (UNIQUE + serialized counter)
- Received/monthly Balance columns
- `VACUUM INTO` backups (WAL-safe snapshots), retention purge logic
- `useInvoices()` consumers (returns picker, invoices page, dashboard recent list) — the hook itself is never modified

---

## B. Implementation phases

### Phase 1 — Duplicate-customer protection

**1.1 Migration `src-tauri/src/database/migrations/018_customer_unique.sql`**
- Merge existing duplicates keyed by `lower(trim(name))` + `trim(COALESCE(phone,''))`:
  - keeper = earliest `created_at` (tie → lowest rowid)
  - `UPDATE invoices SET customer_id = <keeper> WHERE customer_id IN (dupes)`
  - keeper.`outstanding_balance` = SUM of merged rows
  - `DELETE` the non-keeper rows
  - (returns/suppliers do not reference customers)
- `CREATE UNIQUE INDEX IF NOT EXISTS ux_customers_name_phone ON customers(lower(trim(name)), trim(COALESCE(phone,'')));`

**1.2 `src-tauri/src/repositories/customers.rs`**
- `find_duplicate(exclude_id, name, phone)` helper using the same match rule.
- Call in `create` **and** `update` → `AppError::Conflict("A customer with this name and phone already exists. Open the existing customer instead.")`
- Unique index remains the race backstop.

**1.3 Tests**
- duplicate create → Conflict; distinct create → ok; update to duplicate name → Conflict.
- Migration merge validated separately against a DB copy (see Gates).

### Phase 2 — Correct numbers (50-row root cause)

**2.1 `src-tauri/src/commands/invoices.rs` + `src-tauri/src/repositories/invoices.rs`**
- `ListInvoicesInput` gains `from_date: Option<String>`, `to_date: Option<String>` (RFC3339), `customer_id: Option<String>`.
- `InvoiceRepository::list` gains optional range + customer filter; `ORDER BY created_at DESC`; range path capped at `RANGE_CAP = 50_000`.
- Existing callers pass nothing → byte-identical behavior (limit 50 / search cap 200).

**2.2 `src/features/invoices/api.ts`**
- New `useInvoicesInRange(fromIso)` → queryKey `["invoices", "range", fromIso]`.
- New `useCustomerInvoices(customerId, enabled)` → queryKey `["invoices", "customer", id]`.
- `useInvoices()` unchanged.

**2.3 `src/routes/_authenticated/admin.reports.tsx`**
- Swap `useInvoices()` → `useInvoicesInRange(from)` where `from` = earliest of (Jan 1 local, start of 12-month table window) so last-year month rows aren't zeroed.
- All existing math untouched (`within`, `netRevenue`, `effectiveBalance`, monthly table, exports).
- **Excel**: in `exportRows` add a second summary row `NET SALES` = `sum(total) − sum(previous_balance)` below the existing `TOTAL` row (kept byte-for-byte). Applies to all period cards, monthly rows, and "Export all".

**2.4 `src-tauri/src/commands/reports.rs`**
- `get_dashboard`: local-time **sargable** ranges `created_at >= ? AND created_at < ?` with RFC3339 bounds computed via `chrono::Local` (replaces `date(created_at) = ?` and `datetime('now','-N days')`) for sales, invoice counts **and** profit queries (profit 7d/30d currently UTC + non-sargable).
- Add `sales_7d`, `invoices_7d` fields to `DashboardStats` (keep `total_sales_today`, `total_invoices_today`, `low_stock_count`, `outstanding_balance`, profits).

**2.5 `src/lib/api-client.ts`**
- Extend `DashboardStats` interface (lines ~348-356) with `sales_7d`, `invoices_7d`.

**2.6 `src/routes/_authenticated/admin.index.tsx`**
- "Sales today" → `dashboard.total_sales_today` + sub `total_invoices_today`.
- "Last 7 days" → `dashboard.sales_7d`.
- Delete `sumBetween`; keep `useInvoices()` **only** for the Recent invoices table (`slice(0,6)`).
- `["dashboard"]` already invalidated on sale/return/product events (`tauri-events.ts`, `route.tsx`) → no staleness.

**2.7 Customers page**
- `repositories/customers.rs::list` adds SQL `invoice_count` (LEFT JOIN aggregate) to `Customer` struct (`#[serde(default)]`).
- `src/lib/api-client.ts` + `src/features/customers/api.ts` + `src/features/pos/api.ts` `Customer` types gain `invoice_count?: number` (additive; POS reads only id/name/phone).
- `src/routes/_authenticated/admin.customers.tsx`:
  - Orders column → `c.invoice_count`.
  - Drop page-level `useInvoices()` (line 29).
  - Expanded history → lazy `useCustomerInvoices(c.id, expanded === c.id)`, note "showing most recent 200" when capped.
  - Fix unkeyed Fragment (line 115).

### Phase 3 — POS resilience + cache
**3.1 `src/routes/_authenticated/admin.pos.tsx` (lines 127-137)**
- Wrap `createCustomer.mutateAsync` in try/catch:
  - on duplicate error → re-lookup (trim/case-insensitive name + phone) and **reuse existing customer**
  - still not found → continue sale as anonymous walk-in + `toast.warning`
  - a duplicate must never block a sale.

**3.2 Cache invalidation**
- `useCreateCustomer.onSuccess` and `tauri-events.ts` customers handler also invalidate `["pos-customers"]` (currently never invalidated).

### Phase 4 — Hardening
**4.1 `src-tauri/src/database/connection.rs`**
- Run `PRAGMA journal_mode=WAL` **once** on first open (NOT per-connection — per-connection journal switches can hit `SQLITE_BUSY`).
- Per-connection: `busy_timeout=5000`, `synchronous=NORMAL`, `foreign_keys=ON`.

**4.2 `src-tauri/src/commands/backup.rs` (`import_database`)**
- After `fs::copy(stage → live)` delete `moonpipe.db-wal` and `moonpipe.db-shm` (both success and failure paths) — stale-WAL safety.

---

## C. Known limits / notes
- RANGE_CAP 50 000 rows ≈ 137 sales/day for a year — far above this shop's volume; if ever hit, oldest rows in range are truncated.
- "Export all" now exports the fetched window (earliest of Jan 1 / 12-month start), not just 50 rows.
- Dashboard profit/sales figures will **change slightly** (UTC → local, full window) and will then agree with the Reports page.
- WAL means hand-copying only `moonpipe.db` while the app runs can miss recent writes (use Settings → Backups).
- Compile-time sqlx macros validate against `DATABASE_URL` (`.cargo/config.toml` → `C:/ProgramData/MoonPipe/moonpipe.db`); changed queries must stay valid against that schema.

---

## D. Verification gates (run in order)
1. `cargo check --all-targets` (0 new errors)
2. `cargo test --lib` (39 existing + new duplicate-guard tests)
3. `cargo clippy --all-targets` (no new warnings)
4. `npm run lint`
5. `npm run build`
6. Migration test on a **copy** of the live DB with synthetic duplicates injected → verify merge + index
7. Smoke: `PRAGMA journal_mode` = `wal` after startup; dashboard "Sales today" == Reports "Today" card; POS walk-in duplicate flow; customer Orders count; Excel export row count + NET SALES row

### Gate results (2026-10-06)

| Gate | Result |
|------|--------|
| 1. `cargo check --all-targets` | ✅ 0 errors |
| 2. `cargo test --lib` | ✅ 47 passed / 0 failed / 1 ignored (baseline 43 + 4 new) |
| 3. `cargo clippy --all-targets` | ✅ 39 warnings (baseline 40; removed unused `tauri::Manager`, relocated `sort_by_key`) — no new |
| 4. `npm run lint` | ✅ **repo-wide 0 errors / 6 warnings** (was: tool timeout — `eslint.config.js` ignores didn't cover `src-tauri/target` + `dist-spa`, so it traversed the build tree; 1027 of 1129 findings were CRLF artifacts because `.prettierrc` had no `endOfLine`. Fixed: added build outputs to global ignores + `"endOfLine": "auto"`, then `eslint --fix` cleared the remaining 105 prettier issues. The 6 remaining warnings are stock shadcn `react-refresh/only-export-components`) |
| 4b. `tsc --noEmit` | ⚠️ **14 errors — accepted, unchanged from baseline.** `admin.inventory` (4, `CreateProductInput` missing props), `admin.customers` (3) + `admin.pos` (3) `string \| null` vs `string \| undefined`, `admin.cashiers` (2), `tauri-events` (1, `UnlistenFn`), `main.tsx` (1, `notFoundComponent`). Vite does not typecheck, so none block the build; left alone deliberately to avoid behavior changes right before release |
| 5. `npm run build` | ✅ |
| 6. Live-DB-copy migration test | ✅ 20/20 — rewound a `Connection.backup()` copy of the live DB by dropping `ux_customers_name_phone`, injected 3 dup groups (case/whitespace, `created_at` tie, NULL-vs-'' phone); verified keeper = earliest `created_at` / lowest rowid, balance = SUM, invoices repointed, unique index created + rejects, re-run is a no-op, real rows preserved |
| 7a. WAL smoke | ✅ launched the debug binary against the live DB: `journal_mode=delete` → `wal`, `-wal`/`-shm` present while running, persists after exit, `PRAGMA integrity_check = ok`, data intact (2 customers / 19 invoices), relaunch in WAL works |
| 7b. UI smoke | ⏳ **not run** — dashboard "Sales today" == Reports "Today"; POS walk-in duplicate reuse; customer Orders count vs history; Excel row count + NET SALES row |
| 7c. Restore end-to-end | ⏳ **not run** — `remove_stale_wal_files` call sites are unit-tested only |

**Corrections made during implementation:** current SQLite numbers `synchronous` as `0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA` (the test originally asserted `2`); migrations 017 + 018 were already applied to the live DB (the note at HANDOFF line 49 was stale), so gate 6 had to drop the index to reach the pre-018 state.

## E. Implementation order
1. Phase 1 (migration + duplicate guard + tests)
2. Phase 2 (range filters → hooks → reports → dashboard → customers page)
3. Phase 3 (POS fallback + invalidations)
4. Phase 4 (WAL + restore cleanup)
5. All gates + smoke checks
