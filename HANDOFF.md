# Handoff Document — Stone Flow POS Offline Migration

> **Goal**: Convert to single executable, auto-start, fully offline, loosely coupled
> **Constraint**: Zero functional changes to core POS behavior
> **Status**: 🟢 ~90% complete — receipts raster-print, exports/imports fixed, full-history invoice search live, 12-month auto-retention keeping the store lean. **Working tree has uncommitted session changes (see below)**
> **Date**: 2026-08-24
> **Phase**: Phase B verification (post-stabilization) → Phase C release pending

---

## 🔥 Session 2026-08-24: Invoice full-history search + 12-month sales retention

**Uncommitted as of this update.** Gates green: ESLint clean on touched files,
`npx tsc --noEmit` exactly the pre-existing errors (0 new), `npm run build` ✓,
`cargo check`/`cargo build` 0 errors (40 pre-existing warnings, none new).

### 1. Invoices page: recent-50 window + full-history search

The long-standing "only latest 50 invoices exist" limitation got its owner-approved fix —
**not** pagination: keep the fast recent-50 default, add server-side search over all history.
The Rust `list_invoices` command already supported `ListInvoicesInput.query` (case-insensitive
LIKE on invoice_no + customer_name, newest-first, capped at `SEARCH_CAP = 200`) — the frontend
just never sent it.

| Change | File(s) | Behavior |
|--------|---------|----------|
| Optional query param | `lib/api-client.ts` | `invoices.list(params?)`; no params → `{}` → unchanged recent-50 behavior everywhere |
| Search hook | `features/invoices/api.ts` | `useInvoiceSearch(query)` under cache key `["invoices","search",q]`, enabled only when non-empty |
| Search UI | `admin.invoices.index.tsx` | Debounced (300 ms) input above the table, Search/X icons, live match count ("N matches across all invoices"), dedicated loading/empty states; Print / Payment cleared / detail links all work on search results |

Zero system-wide impact by design: dashboard, customers, returns, reports still consume
untouched `useInvoices()` (recent-50); TanStack prefix invalidation means every existing
`["invoices"]` invalidation (create, mark-paid, realtime) also refreshes search results.

### 2. Sales-data retention: auto-purge settled invoices older than 12 months

Owner-approved policy: keep only the last 12 months of *sales data* so the SQLite store stays
light for years. Implemented entirely in Rust — zero frontend changes.

| Change | File(s) | Behavior |
|--------|---------|----------|
| Retention service | `services/retention.rs` (new) | On each run: cutoff = now − 365 d; count invoices `created_at < ? AND amount_paid >= total`; if none → no-op; else `VACUUM INTO` snapshot into the standard backups folder (**snapshot failure aborts the purge**), then DELETE; items + returns cascade via FK; logs result |
| Startup hook | `lib.rs` `.setup()` | Spawns purge in background right after DB init — effectively daily since the shop opens the app daily; emits `invoices:changed` + `customers:changed` only when rows were removed |
| Post-restore hook | `commands/backup.rs` `import_database` | Same run after a restore swaps the pool, so an old restored DB is brought up to policy |

**Safety rules baked in:**

1. **Unpaid/credit invoices are never deleted** — `customers.outstanding_balance` is denormalized
   and unreconcilable without its source invoice; such invoices become purge-eligible automatically
   once fully settled.
2. **Nothing is ever truly lost** — every purge day leaves one recoverable snapshot visible under
   Settings → Backups (`citytiles_backup_*.sqlite`).
3. Invoice numbering (`invoice_counter`), products, customers, users are untouched; dashboards/
   reports/customers UIs read the recent-50 window so nothing visibly shifts.

### 3. Verification notes (this session)

- Prettier run on the three frontend files normalized CRLF→LF per `.prettierrc` — their diffs
  include cosmetic reformatting of pre-existing lines.
- Manual test not yet done: backdate a fully-paid invoice >12 months → expect it gone from
  search/list at next launch + matching backup snapshot present.

## 🔥 Session 2026-08-23 (continued): Receipt overhaul v2 + Export/Import fix

**Uncommitted as of this update** — 15 modified files + 5 new (`receipt_bitmap.rs`, `file-save.ts`, 3 font TTFs). All gates green: `cargo check --all-targets` 0 errors, clippy clean in touched files, `npm run build` ✓, `npx tsc --noEmit` still exactly the 40 pre-existing errors (0 new), `cargo test --lib` 8/8.

### 1. Thermal receipts: wired up + raster redesign

Root cause of "squished text + full roll per receipt": the `1c99a75` ESC/POS code was
**dead** — every print button used `window.print()` on the A4 HTML layout through
WebView2's GDI pipeline (scaled render + page-length form feed). Also explains the
"localhost" header artifact in print output.

| Fix | File(s) | Behavior |
|-----|---------|----------|
| Auto-print after checkout | `admin.pos.tsx` | `usePrintReceipt().mutate()` fire-and-forget post-create; toasts report success/backend error |
| Invoices list Print works | `admin.invoices.index.tsx` | Was a blocked `window.open` popup — now calls thermal receipt directly |
| Detail page Print receipt | `admin.invoices.$invoiceId.tsx` | "Print / Save PDF" replaced by "Print receipt"; `?print=1` deep-link handling deleted |
| Raster receipt renderer | `services/receipt_bitmap.rs` (new) | Whole receipt drawn to a 576-dot bitmap with the POS's real typefaces and sent via `GS v 0`; text-mode ESC/POS kept as automatic fallback if fonts fail to parse |
| Bundled fonts | `src-tauri/src/assets/fonts/` | Karla Regular/Bold + Cormorant Garamond Bold TTFs (Google Fonts static instances, latin subset); loaded via `include_bytes!` |
| Spooler-first dispatch | `services/print.rs` | Windows spooler RAW is path 1, direct USB ESC/POS fallback (was USB-first; printer lives in the spooler) |
| Business header from frontend | `commands/print.rs`, `api-client.ts` | Optional `business {name,address,phone}` arg sourced from `BUSINESS` constant; Rust defaults mirror it |

Receipt design (client-approved direction): Cormorant Garamond Bold banner, Karla body,
generous line leading, pixel-measured right-aligned bold amounts, dashed/solid rules,
large bold TOTAL row, centered "Thank you!" + "Developed by AUZ Tech" footer,
**equal ~6 mm top/bottom margins**, auto-cut. Layout constants live at the top of
`receipt_bitmap.rs` for taste tuning after the first live paper test.

### 2. Exports/imports: one working save pathway

Root cause: no functioning file-save route existed in the webview. Inventory export
fetched products then discarded them; reports used browser-download `XLSX.writeFile`
(silently does nothing under Tauri).

| Fix | File(s) | Behavior |
|-----|---------|----------|
| Shared save helper | `lib/file-save.ts` (new) | Native Save As dialog (`plugin-dialog`) → write via `@tauri-apps/plugin-fs` → returns chosen path |
| FS permission granted | `capabilities/default.json` | `fs:allow-write-file` scoped `**`, write-only, exercised only via user-picked dialog paths |
| Inventory Export Excel implemented | `admin.inventory.tsx` | Builds workbook from all products using headers that exactly match the importer's aliases → round-trips back through Import |
| Reports exports fixed | `admin.reports.tsx` | All three buttons ("Export all", period, monthly) routed through the helper |
| Import polish + dead code out | multiple | Import also invalidates `dashboard`; removed unused `useExportProducts` / wrong-typed `products.export(): Promise<Blob>` / unused page hooks. Backend `export_products` command kept |

### 3. Returns module un-stubbed + numeric input hardening (same day, later)

| Fix | File(s) | Behavior |
|-----|---------|----------|
| Items actually load | `admin.returns.tsx` | Mock-era stub query replaced with real `get_invoice_with_items`; proper loading/error/empty states |
| True returnable counts | `admin.returns.tsx` | Returned quantities summed per product from the returns ledger; over-return blocked client-side |
| Recent returns fixed | `admin.returns.tsx` | Client-side join to invoices for invoice_no/customer; product × qty + per-row value shown. Fixed 3 pre-existing tsc errors |
| Selection block on top | `admin.returns.tsx` | Item/qty/reason/button panel renders above the invoice picker once chosen ("Change invoice" link resets) |
| Server over-return guard | `repositories/returns.rs` | In-tx validation: cumulative returned qty per product cannot exceed invoiced qty → validation error before any writes |
| Number inputs tamed | `components/ui/number-input.tsx` (new), `styles.css`, POS/detail/returns/inventory | Wheel-over-field can no longer change values; spin buttons hidden globally; digits-only entry at all 11 numeric fields; min-clamp on blur |
| Negative-discount bug killed | `admin.pos.tsx`, `commands/invoices.rs` | `-10000` in discount previously ADDED to the bill and reached the DB unchecked. UI clamps ≥0 in bill math AND backend rejects negative discount/subtotal/total/paid/item values server-side |
| Payment guard | `commands/invoices.rs` | `mark_invoice_paid` rejects amount ≤ 0 instead of silently no-op-ing |
| Oversell hard-blocked | `repositories/invoices.rs` create + `admin.pos.tsx` | Selling beyond stock was possible (3 kg on hand → sold 30 → −27). Now: quantities aggregated per product across duplicate cart lines and validated against live stock inside the invoice transaction — rejection names every shortfall ("in stock: X, tried: Y"); client pre-check gives the same message instantly. Policy locked: hard block always, delivery date or not. Existing negative rows are NOT auto-repaired — correct via Inventory edit |

~~⚠️ **Discovered, NOT yet fixed**: `list_invoices` defaults to limit 50...~~
✅ **RESOLVED (2026-08-24)** — owner chose recent-50 + full-history server-side search over
pagination (see Session 2026-08-24 §1); store growth bounded by the 12-month retention purge (§2).

### 4. Build infrastructure

| Item | Detail |
|------|--------|
| New deps | `ab_glyph = "0.2"` (pure-Rust glyph rasterizer), npm `@tauri-apps/plugin-fs@^2` |
| Fonts note | Google Fonts CSS API serves EOT to MSIE UAs and extensionless TTF endpoints — use an Android-era UA when re-downloading static TTFs |

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

## 🔧 Session 2026-08-23 (continued): Cleanup Execution + Receipt Printing

CLEANUP_PLAN.md executed end-to-end — one commit per phase (`99c96e6`…`41f78e0`), then three
follow-up fixes from live testing. Automated gates green throughout: `cargo check --all-targets`,
`cargo clippy`, `npm run build`, `npx tsc --noEmit` (no new type errors in touched files).
Git repo initialized locally this session (no remote yet as of last update).

| Fix | Commit(s) | Notes |
|-----|-----------|-------|
| Cleanup Phases 0–7 | `99c96e6`→`41f78e0` | Type truth, Supabase excision, PDF pipeline, cashier reset UI, backup hardening + Settings page, per-install JWT, docs refresh |
| Receipt money bug | `a4219cf` | `print.rs` had its own `format_price` still ÷100 — receipts printed totals 100× too small; also widened USB detection (any printer-class 0x07 device) and discovered real bulk OUT endpoint |
| Startup crash: migration checksum | `3d401f8`, `0991fe1` | Editing applied migration files (even comments) changes their sqlx checksums → every existing DB fails boot validation. Reverted file bytes; **`.gitattributes` now pins `migrations/*.sql` to LF** so clones/checkouts can never reintroduce it. Diagnosed by hashing on-disk/git-blob/CRLF variants against `_sqlx_migrations.checksum` |
| Receipt printing overhaul | `1c99a75` | 80mm printer is installed as a Windows spooler printer → old code fell back to `notepad /p`: squished text + full-page paper feed. Now: **Print Spooler API with RAW datatype** to the default printer (printer receives ESC/POS directly → correct width, auto-cut, zero page-feed waste). Layout rebuilt: 48-char two-column rows, right-aligned amounts, wrapped names, double-height banner, bold TOTAL. Fallback order: direct USB → spooler RAW |

**Receipt printing decision (owner-approved)**: target the Windows *default* printer; no printer-picker UI. Keep `PKR ` prefix on receipt amounts.

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
| Thermal printer (80mm) live test of **raster** receipt output | Printer on-site; text path verified working via spooler RAW; new raster layout (fonts/margins/spacing) needs one paper test — tune constants in `receipt_bitmap.rs` if taste off |
| `cargo tauri build` MSI/NSIS with self-signed cert ("AUZ Tech") | Pending |
| Staff quick-start documentation | Pending |

### Manual regression matrix after CLEANUP_PLAN execution (GUI required)

All phases verified by `cargo check --all-targets` (0 errors), `cargo clippy`, `npm run build`
and `npx tsc --noEmit` (no new type errors in touched files). The following need a human at the
running app:

- [ ] Login → dashboard → POS smoke test; dashboard numbers identical to pre-cleanup
- [ ] Thermal receipt (80mm) raster output: Karla/Cormorant render at roll width, symmetric
  top/bottom margins, auto-cut fires, bold TOTAL; totals/paid/balance match UI exactly;
  triggers = post-checkout auto, invoices list Print, detail "Print receipt" — all identical output
- [ ] Inventory Export Excel → open file in Excel → re-import it → expect "0 added, N updated"
- [ ] Reports exports ("Export all" + period + monthly) each save via Save As dialog and open cleanly
- [ ] PDF invoice: generate from a real invoice via "Download A4 PDF"; totals/paid/balance match UI exactly
- [ ] Cashier reset-password: admin resets → new password works, old rejected
- [ ] Backup: export → import roundtrip preserves data; corrupt/garbage file rejected cleanly
- [ ] Invoice search (2026-08-24): type an old invoice_no/customer on Invoices page → results
  appear with match count; clearing the box restores recent-50; Print/Payment-cleared work on
  search rows; dashboard/reports/customers/returns pages unchanged
- [ ] Retention purge (2026-08-24): backdate a fully-paid invoice >12 months → gone at next app
  launch; `citytiles_backup_*.sqlite` snapshot appears in Settings → Backups; backdated
  partially-paid/credit invoice survives until settled
- [ ] JWT: fresh run creates `%PROGRAMDATA%\CityTiles\jwt.key`; second run reuses it; env override wins
- [ ] Reports page renders unchanged numbers

## ⚠️ Known Issues / Out-of-Scope Leftovers

Statuses updated after CLEANUP_PLAN execution (2026-08-23):

1. ~~**Dashboard/reports response shapes**~~ — ✅ FIXED (cleanup Phase 1): TS types now mirror Rust
   wire shapes exactly (`DashboardStats`, `SalesReportItem[]`, `InventoryReportItem[]`); UI keeps
   computing numbers client-side; semantic divergences documented below.
2. ~~**Public website routes Supabase-backed**~~ — ✅ FIXED (cleanup Phase 2): `/website/*` routes,
   SiteShell/InquiryForm/ProductCard and `src/integrations/{supabase,lovable}` deleted;
   `@supabase/supabase-js` + `@lovable.dev/cloud-auth-js` removed from dependencies.
3. ~~**PDF fonts**~~ — ✅ FIXED (cleanup Phase 3): DejaVuSans Regular+Bold embedded via
   `include_bytes!`; money formatter fixed to whole rupees (was ÷100 = 100× too small totals);
   output dir created on demand; filename sanitized; "Download A4 PDF" button wired on invoice detail.
4. ~~**Backup UI missing / blind import copy**~~ — ✅ FIXED (cleanup Phase 5): swappable pool holder,
   VACUUM INTO consistent export snapshots, validate-first restore with integrity_check +
   required-tables gate + auto pre-restore snapshot; admin-only Settings page with export/list/import
   (native file dialog, double-confirm).
5. ~~**Cashier reset-password no UI entry point**~~ — ✅ FIXED (cleanup Phase 4): Reset Password action
   for approved cashiers with dialog + confirm; hook arity bug fixed; `emit_cashiers_changed` wired.
6. ~~**JWT hardcoded fallback secret**~~ — ✅ FIXED (cleanup Phase 6): env `JWT_SECRET` → persisted
   `%PROGRAMDATA%\CityTiles\jwt.key` (random per install, generated first boot) → ephemeral random.
   Existing sessions log out once when the updated binary ships.

### ⚠️ Migration hygiene (learned 2026-08-23)

**Never edit files under `src-tauri/src/database/migrations/` once they have shipped or run on any
machine.** sqlx stores a content checksum per migration in `_sqlx_migrations`; even comment-only
edits change the checksum and crash every existing install at startup ("migration N was previously
applied but has been modified"), including after backup restores. Schema corrections go in a NEW
numbered migration file. The whole-rupee money-unit truth lives in code comments + Locked Decisions,
not in migration comments.

### Semantic Divergences — documented, intentionally NOT fixed (2026-08-23 type-truth cleanup)

TS types now mirror Rust payloads exactly (`DashboardStats`, `SalesReportItem[]`, `InventoryReportItem[]`).
The UI continues to compute displayed numbers client-side from invoice/product lists. Backend aggregates
remain unused by the UI; these semantic gaps are recorded so nobody "fixes" them accidentally later:

1. **Sales aggregates don't net returns** — `get_sales_report` sums invoices only.
2. **Rust day boundary is UTC** (`commands/reports.rs` uses `date(created_at)` vs `Utc::now()`) — Pakistan is
   UTC+5; adopting server aggregates later would visibly shift the "today" window by ~5 hours.
3. **`outstanding_balance` semantics differ** — Rust sums `customers.outstanding_balance > 0`;
   the dashboard card sums `max(0, total − amount_paid)` over invoices client-side.
4. **`low_stock_count` filters `is_published = 1`** in Rust; the dashboard card does not filter.

## 🔒 Locked Decisions

| Decision | Value |
|----------|-------|
| Database | Single SQLite at `%PROGRAMDATA%\CityTiles\citytiles.db` via sqlx (plugin-sql removed) |
| Payment rule | Invoice number tracks dues universally; customer balance mirrors any attached-customer due |
| Code signing | Self-signed, publisher "AUZ Tech" |
| Stock policy | **Hard block on oversell** — invoice creation validates per-product stock in-transaction (aggregated across cart lines); no negative stock from sales, delivery date or not (2026-08-23) |
| Seed products | 20 examples ship via migration 004 |
| Printing | Receipts: **raster bitmap** (`GS v 0`) with bundled POS typefaces — Cormorant Garamond Bold banner + Karla body (`receipt_bitmap.rs`); device order = Windows spooler **RAW** first, direct USB fallback; classic ESC/POS text mode kept as automatic fallback; A4 genpdf invoices via "Download A4 PDF" (2026-08-23) |
| Receipt triggers | Auto-print after POS checkout + manual buttons (invoices list Print, detail "Print receipt"); `window.print()` is banned from receipt flows — no print dialogs, no WebView downloads |
| Exports | All Excel exports funnel through `lib/file-save.ts`: native Save As dialog → plugin-fs write. Inventory export headers must stay in lockstep with importer aliases (round-trip contract) |
| Migrations | **Immutable once applied** — never edit files in `src-tauri/src/database/migrations/`; LF endings pinned via `.gitattributes`; corrections go in new numbered migrations (2026-08-23) |
| Public `/website/*` routes | **Deleted from app** (2026-08-23): routes, SiteShell/InquiryForm, Supabase dep removed |
| JWT secret | **Per-install random key file** `%PROGRAMDATA%\CityTiles\jwt.key`; env var wins; hardcoded fallback removed (2026-08-23) |
| Money unit | **Whole rupees are canonical** everywhere incl. PDF invoices — no paise conversion anywhere (2026-08-23) |
| Invoice history window | **Recent-50 default + full-history search** (no pagination): list pages show newest 50; Invoices page search hits all history server-side, capped at 200 results (`SEARCH_CAP`) (2026-08-24) |
| Sales retention | **Auto-purge settled invoices older than 12 months** on every app launch and after backup restore; unpaid/credit invoices are exempt until fully settled; a `VACUUM INTO` snapshot is mandatory before any delete — snapshot failure aborts the purge (2026-08-24) |
| Report types | **Type-truth alignment done** — TS mirrors Rust payloads; UI computes client-side (2026-08-23) |

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

**Status**: ~90% complete — system functional end-to-end in dev. Proceed with Phase B verification checklist above; what remains is live-machine testing (printer paper test, clean install, installer build) and staff docs.
