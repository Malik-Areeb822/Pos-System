# Cleanup Plan — Clear Known Issues Before Release

> **Goal**: Resolve all known issues from `HANDOFF.md` (§ Known Issues / Out-of-Scope Leftovers) plus issues discovered during research, without breaking verified POS behavior.
> **Constraint**: Zero functional changes to core POS behavior.
> **Status**: 📝 Plan only — implementation NOT started.
> **Date**: 2026-08-23
> **Companion docs**: `HANDOFF.md` (current state), `FIX_PLAN.md`, `MIGRATION_PLAN.md`

---

## Locked Decisions (owner-approved 2026-08-23)

| Decision | Choice |
|----------|--------|
| Public `/website/*` routes | **Delete from app** (routes, SiteShell, InquiryForm, Supabase dep) |
| JWT secret | **Per-install random key file** persisted in `%PROGRAMDATA%\CityTiles\jwt.key`; env var still wins; hardcoded fallback removed |
| Dashboard/reports mismatch | **Type-truth cleanup only** — align TS types to Rust payloads, delete dead code, keep UI computing client-side |

---

## ⚠️ Discovered Beyond the Handoff List

These were found during code-level verification and drive scope below:

1. **Money-unit conflict (financial-correctness landmine).**
   Frontend stores/transacts **whole rupees as integers**:
   - `src/routes/_authenticated/admin.pos.tsx:62` computes `subtotal = Σ product.price × qty` raw (no ×100 anywhere in `src/`)
   - `src/features/inventory/api.ts:121-126` `currency()` formats raw integers
   - But `src-tauri/src/services/invoice_pdf.rs:94-97` `format_price()` divides by 100 (the "paise" schema comment is wrong vs actual data).
   - Today invisible because font loading fails first (`invoice_pdf.rs:19`). Fixing fonts alone would print invoices with totals **100× too small**.
   - Resolution: whole-rupees is canonical (matches what users see); fix the PDF formatter.

2. **Dead duplicate modules** (zero callers, verified by grep):
   - `src-tauri/src/services/reports.rs` — duplicate `DashboardStats` + query, never called (only `commands::reports` registered, `lib.rs:83-86`)
   - `src-tauri/src/services/backup.rs` — 71-line twin of backup commands, never called

3. **Broken unused hook**: `src/features/cashiers/api.ts:55` calls `api.cashiers.resetPassword(id)` with one argument; signature needs `(id, newPassword)` (`api-client.ts:169,429-430`). Latent type error because nothing imports the hook yet.

4. **SQLite pragmas missing** → see "Flagged, Not Scheduled" at bottom.

---

## Phase 0 — Checkpoint (prerequisite)

- [ ] Verify git state — workspace root is currently **not** a git repo. Establish version-control checkpoint (init+commit or equivalent) so each phase is individually revertable.
- [ ] Baseline verification:
  ```bash
  npm run build
  cd src-tauri && cargo check --all-targets
  cargo tauri dev   # smoke test login → dashboard → POS
  ```
- **Gate**: baseline recorded before any edits.

---

## Phase 1 — Type Truth + Dead-Code Removal (zero behavior change)

Context: the shape mismatch is real but **latent** — dashboard (`admin.index.tsx`) and reports (`admin.reports.tsx`) compute all displayed numbers client-side from invoice/product lists; `useSalesReport`/`useInventoryReport` have zero callers.

### Tasks
1. Align TS types to actual Rust wire shapes:
   - Rust truth (`src-tauri/src/commands/reports.rs:9-38`, snake_case — no serde rename exists):
     ```rust
     DashboardStats { total_sales_today, total_invoices_today, low_stock_count, outstanding_balance }  // i64s
     SalesReportItem { date, total_sales, invoice_count, cash_sales, credit_sales, bank_sales }        // Vec<>
     InventoryReportItem { id, name, sku?, category, stock_qty, low_stock_threshold, unit, price, value } // Vec<>
     ```
   - Update both TS copies: `src/lib/api-client.ts:309-339` and `src/features/reports/api.ts:4-67`
   - Update `ReportsApi` bindings (`api-client.ts:158-162, 414-422`)
2. Delete dead code:
   - `src-tauri/src/services/reports.rs` (+ remove decl in `services/mod.rs:7`)
   - `src-tauri/src/services/backup.rs` (+ remove decl in `services/mod.rs:8`)
   - `src/hooks/useInvoiceRealtime.ts` (fossil of `lib/tauri-events.ts:37-65`; all 3 call sites import `@/lib/tauri-events` instead)
3. Document (do NOT fix) semantic divergences as known limitations — add to `HANDOFF.md`:
   - Sales aggregates don't net returns
   - Rust day boundary is UTC (`commands/reports.rs:48,51` uses `date(created_at)` vs `Utc::now()`) — PK is UTC+5; adopting server aggregates later would visibly shift "today"
   - `outstanding_balance`: Rust sums `customers.outstanding_balance > 0` (`reports.rs:70-73`); UI sums `max(0, total − amount_paid)` over invoices (`admin.index.tsx:44-47`)
   - `low_stock_count` filters `is_published=1` (`reports.rs:64-66`); dashboard card doesn't filter

### Gate
- `cargo check --all-targets` 0 errors; `npm run build` clean
- Dashboard + reports pages render **byte-identical numbers** as before

---

## Phase 2 — Supabase Excision (delete `/website/*`)

Blast radius verified contained: admin routes are grep-clean; `/website/*` fails offline gracefully today (empty states, no crash).

### Tasks
1. Delete routes & components:
   - `src/routes/website/index.tsx`, `about.tsx`, `catalog.tsx`, `product.$productId.tsx`, `trade.tsx`, `contact.tsx`
   - `src/components/website/SiteShell.tsx` (self-documents "TABLED FOR LATER", line 1)
   - Inquiry form component (`InquiryForm.tsx`)
2. Trim `src/lib/catalog.ts`: **keep pure exports** (`CATEGORIES`, `categoryMeta`, `productImage`, currency helpers, types) — 4 files depend on them with no backend need. Remove only the Supabase-backed fetch functions (`catalog.ts:74-86`).
3. Remove login-screen link to `/website` (`src/routes/index.tsx:85`).
4. Then delete (now unreferenced):
   - `src/integrations/supabase/client.ts`, `types.ts`, `client.server.ts`, `auth-middleware.ts`, `auth-attacher.ts`
   - `src/integrations/lovable/index.ts`
5. Dependencies: remove `@supabase/supabase-js` (`package.json:43`) and `@lovable.dev/cloud-auth-js` (`package.json:16`); regenerate lockfiles. Keep `@lovable.dev/vite-tanstack-config` (active build wrapper, `vite.config.ts:1`).
6. Purge `.env` Supabase URL/key lines.
7. `routeTree.gen.ts` regenerates on next vite build/dev.
8. CSP review (`src-tauri/tauri.conf.json:25`): if nothing else references Google Fonts after site removal, drop `https://fonts.googleapis.com` / `fonts.gstatic.com` allowances — verify first (check index.html/CSS).

### Gate
- Repo-wide grep: zero supabase references
- Build clean; admin flows smoke-tested (login, POS checkout, invoice detail)

---

## Phase 3 — PDF Invoice Pipeline (core business path)

Current failure: `invoice_pdf.rs:19` loads fonts via `fonts::from_files("./fonts", "DejaVuSans", None)` — CWD-relative, no fonts dir exists anywhere. Only true CWD-relative load in the repo.

### Tasks
1. Source DejaVuSans Regular + Bold TTFs (free license) into e.g. `src-tauri/src/assets/fonts/`.
2. Embed via `include_bytes!` + `genpdf::fonts::FontData::new(bytes.to_vec(), None)`; hand-build `FontFamily{regular, bold}` (italic/bold_italic slots reuse bytes — content uses only plain `Paragraph`s, verified no `TextStyle` usage).
   - No `tauri.conf.json` resources changes needed; identical behavior dev/MSI/NSIS.
3. **Fix money unit** (the landmine): rewrite `format_price` (`invoice_pdf.rs:94-97`) to format whole rupees directly — no ÷100. Optionally correct the schema comment ("paise") in `001_initial_schema.sql`.
4. Harden output path:
   - `std::fs::create_dir_all(&config.app_data_dir)` before `File::create` (`invoice_pdf.rs:36` currently depends on DB-init side effect for dir existence)
   - Sanitize `invoice_no` when building filename (`invoice_pdf.rs:34`) — strip `\ / : * ? " < > |`
5. Wire/confirm UI entry point for PDF generation on invoice detail route (verify which command exposes it; add button if absent).

### Gate
- Generate PDF from a real invoice; subtotal/total/paid/balance match UI exactly
- Generation succeeds regardless of working directory
- Works from installed layout (post-Phase-C spot check)

---

## Phase 4 — Cashier Reset-Password Completion (small)

Backend complete & admin-gated: `commands/cashiers.rs:65-72` (`ResetPasswordInput {id, new_password}`, `require_admin`), repo method `repositories/cashiers.rs:109-115`.

### Tasks
1. Fix hook arity bug: `features/cashiers/api.ts:53-57` → pass `(id, newPassword)`; add `invalidateQueries(["cashiers"])`.
2. Add Reset Password action in `admin.cashiers.tsx` action cell (lines 123-146 pattern): show for `approved` cashiers, dialog with new-password input + confirm, follow existing toast conventions (`toast.success/error(err.message)`).
3. Rust consistency: add `crate::events::emit_cashiers_changed(&app)` after successful reset (siblings do at `cashiers.rs:43,52,61`).

### Gate
- Admin resets a cashier password → cashier logs in with new password, old password rejected

---

## Phase 5 — Backup Hardening + Settings UI (largest new surface)

Current import does blind `fs::copy` over the live DB while pool holds up to 5 open connections (`commands/backup.rs:35-36` — comment admits it). Windows sharing violations or split-brain state possible. No validation, no transaction boundary, no rollback path. Export has same blind-copy risk (`backup.rs:18`).

Pool facts: `DbPool = SqlitePool` max 5 connections, created once at startup, managed via `app.manage(pool)` (`database/connection.rs:23-31`, `lib.rs:30-33`); commands use `State<DbPool>` → `pool.inner().clone()`. No WAL configured (default journal mode). No settings route exists; nav lives in `AdminShell.tsx:21-30`.

### Tasks
1. Backend redesign of `import_database`:
   - **Validate-first**: copy backup to temp path, open temp pool on it, `PRAGMA integrity_check` + confirm expected tables/migration version
   - **Auto-snapshot** current DB before restore (safety copy)
   - Close live pool completely → `fs::copy` backup → db_path → rebuild pool
   - Pool swap mechanics: Tauri `manage()` cannot replace state ⇒ introduce swappable holder (e.g., `RwLock<SqlitePool>` container with acquire/replace) and mechanically update ~40 command sites (`pool.inner().clone()` → acquire pattern)
   - Rejected alternative (for the record): ATTACH-based in-place table-by-table restore inside one transaction — transactionally safe but far more complex than the mechanical holder refactor
2. Export hardening: run `integrity_check` before copy; document in-flight-write caveat (single-user desktop keeps risk low).
3. Frontend:
   - New `admin.settings.tsx` route, admin-only NAV entry "Settings" (`AdminShell.tsx` NAV array)
   - Sections: Export Backup (timestamped file under `%PROGRAMDATA%\CityTiles\backups`), List Backups table, Import (native file dialog + double-confirm modal warning data replacement)
   - Add `backupsApi { export, import, list }` to `api-client.ts` (`ApiClient` interface lines 110-118)

### Gate
- Export→import roundtrip preserves all data
- Import of corrupt/garbage file rejected cleanly, live DB untouched
- Restore blocks concurrent POS ops via modal during swap

---

## Phase 6 — Per-Install JWT Secret

Facts: secret resolution at `config.rs:22-29` (env `JWT_SECRET` → hardcoded fallback, string appears exactly once repo-wide, zero tests reference it). Consumers: `auth/jwt.rs:37` (encode) and `jwt.rs:45` (decode) via global `CONFIG.jwt_secret_bytes()`. Tokens persist client-side in localStorage key `"city-tiles-auth"` (`auth-store.ts:30,79-83`); no server-side revocation exists; expiry 24h.

### Tasks
1. `Config::default()` precedence chain:
   1. env `JWT_SECRET`
   2. persisted key file `%PROGRAMDATA%\CityTiles\jwt.key` (32 random bytes hex; generate + write on first boot; ensure `create_dir_all` first)
   3. ephemeral-random fallback if file write fails (app never bricks; forces re-login per boot instead of shipping a known constant)
2. Remove hardcoded string entirely.
3. Note `CONFIG` is lazily constructed at first access — make the secret function defensive about directory existence regardless of init order.

### Impact (accepted)
One-time logout for all users when updated binary ships (old tokens fail HS256 verification). Re-login trivial. Time deployment near release so users aren't logged out twice.

### Gate
- Fresh run creates `jwt.key`; second run reuses it (sessions survive restart)
- Env var override wins
- `cargo check --all-targets` clean

---

## Phase 7 — Regression Sweep + Docs Refresh

- [ ] Full manual matrix (money-touching flows especially):
  signup/login/logout · roles/nav gating · inventory CRUD · CSV import · POS checkout (full / partial / credit) · invoice detail incremental payments · returns (totals/due/balance/stock) · cashier approve/reject/suspend/reset · dashboard & reports unchanged numbers · PDF print · backup roundtrip · autostart registry entry · single-instance focus (pending Phase-B items from HANDOFF.md)
- [ ] `cargo check --all-targets`, `npm run build`, optional clippy
- [ ] Update `HANDOFF.md`: issue statuses, newly locked decisions (website deleted from app, per-install JWT, **whole-rupee canonical unit**, type-truth alignment done)

---

## Flagged, Not Scheduled (needs separate decision)

- **SQLite connection pragmas missing** (`database/connection.rs:23-31` sets none):
  - `PRAGMA foreign_keys = ON` appears only inside migration SQL (`001_initial_schema.sql:4`), executing on one pooled connection — FK enforcement is effectively OFF on every other pooled connection → potential referential-integrity drift.
  - No `busy_timeout` → possible spurious `SQLITE_BUSY` under concurrency.
  - Enabling FK enforcement is correct long-term but could surface latent violations as runtime errors — conflicts with the zero-functional-change constraint. Decide separately.

---

## Execution Order Rationale

| Order | Why |
|-------|-----|
| 0 → 1 | Truth-first: types/dead-code are prerequisites other phases build on; lowest risk |
| 2 | Dependency deletion shrinks surface before adding features |
| 3 | PDF is core-business-path; must precede release even though printer hardware deferred |
| 4 | Tiny completion, independent |
| 5 | Largest design surface; last feature addition so regressions localize |
| 6 | JWT logout impact should coincide closest to release (users log out once) |
| 7 | Full sweep validates everything together |
