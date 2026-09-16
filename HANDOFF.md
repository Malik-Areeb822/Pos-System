# Handoff Document — Stone Flow POS Offline Migration

> ⚠️ **MANDATORY RULE FOR ALL AGENTS & DEVELOPERS**: 
> **You MUST read [PROJECT_ARCHITECTURE.md](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/PROJECT_ARCHITECTURE.md) before making ANY code changes, fixes, refactors, or feature upgrades.** It maps out the complete system layout, IPC bridges, database schemas, and critical component dependencies to prevent breaking connected features.

> **Goal**: Convert to single executable, auto-start, fully offline, loosely coupled
> **Constraint**: Zero functional changes to core POS behavior
> **Status**: 🟢 FREEZE RESOLVED + INSTALLERS REBUILT & RE-SIGNED (2026-08-25 late night) — React 18.3.1 pin passed full gauntlet incl. owner's manual login + sample sale; E/A/D mitigations reverted; fresh MSI + NSIS built via `cargo tauri build`, ship-gate passed on their own exe (serving/storm/typing/CPU), both signed `CN=AUZ Tech` + RFC3161 timestamp (valid to 2036). Remaining: clean-machine install test → full manual matrix → staff docs.
> **Date**: 2026-08-25
> **Phase**: Phase C release

---

## 🔥 Session 2026-09-16: Tile Area Calculation — Display-Only Area Tracking

**Status**: ✅ **COMPLETE.** Area-per-tile on products, total area computed at invoice creation, displayed on receipts (bold), PDFs, and invoice detail pages. Area is display-only — pricing remains per tile.

### What was built

| Phase | Files | Summary |
|-------|-------|---------|
| **Phase 1: DB** | `008_tile_area.sql` | `ALTER TABLE products ADD COLUMN area_per_tile REAL` + `ALTER TABLE invoice_items ADD COLUMN total_area REAL` — nullable, zero impact on existing data |
| **Phase 2: Rust structs** | `repositories/products.rs`, `repositories/invoices.rs` | `Option<f64>` fields on `Product`, `CreateProductInput`, `InvoiceItem`, `CreateInvoiceItemInput` — all SELECT/INSERT/UPDATE SQL updated |
| **Phase 3: Rust commands** | `commands/products.rs`, `commands/invoices.rs` | Fields + mappings on both input structs; `total_area` passed through from frontend |
| **Phase 4: Receipt/PDF** | `services/receipt_bitmap.rs`, `services/invoice_pdf.rs` | Bold area sub-line with `{:.3}` formatting on receipt; `[X.XXX sqm]` appended to item text on PDF |
| **Phase 5: CSV import** | `services/inventory.rs` | Reads `area_per_tile` from CSV column 11, parsed as `f64` |
| **Phase 6: NumberInput decimal** | `components/ui/number-input.tsx` | New `decimal?: boolean` prop allows `.` in numeric input, sets `inputMode="decimal"` |
| **Phase 7: TS types** | `api-client.ts`, `features/pos/api.ts`, `features/inventory/api.ts`, `lib/catalog.ts`, `features/invoices/api.ts`, `lib/pos.ts` | `area_per_tile: number \| null` on Product types; `total_area` on InvoiceItem types |
| **Phase 8: UI** | `admin.inventory.tsx`, `admin.pos.tsx`, `admin.invoices.$invoiceId.tsx` | Product form with `area_per_tile` field (tiles-only, decimal); POS checkout: `total_area = apt × qty`; Invoice detail: conditional Area column with `.toFixed(3)` |

### Bug fixes during implementation

| Bug | Root cause | Fix |
|-----|-----------|-----|
| Area formula was `ptc × apt × qty` (showed tiles × area, not total area) | Misread `qty` as cartons when it's already tiles | Corrected to `apt × qty` — `qty` in cart is tiles |
| Float artifact `31.95999999999997` | Rust `{:.2}` and TS `.toFixed(2)` only 2 decimals | Changed to `{:.3}` (Rust) and `.toFixed(3)` (TS) across all three display points |
| Area text not bold on receipt | Receipt area line used `karla_regular` | Changed to `karla_bold` |
| `cargo check` failed after migration 008 | Live DB (`%PROGRAMDATA%\CityTiles\citytiles.db`) missing `area_per_tile` and `total_area` columns — `sqlx::query!` macros validate at compile time | Applied migration to live DB manually; `sqlx::migrate!` runs it at app startup for customers |

### Supplier purchase default quantity change

Changed default quantity for new supplier purchase items from `1` to `0`. Updated `admin.suppliers.tsx` validation from `< 1` to `<= 0` so entering `0` saves cleanly.

### Key patterns confirmed

- `sqlx::query!` macros validate SQL at compile time against the live database at `C:\ProgramData\CityTiles\citytiles.db`
- After adding columns via raw SQL, must also insert a row into `_sqlx_migrations` with correct SHA384 checksum before `cargo check`
- `sqlx::migrate!` runs all pending migrations at app startup automatically — no manual DB patching needed for customers
- Area stored as `Option<f64>` (SQL `REAL`) for decimal precision; display limited to 3 decimal places
- Area is **display only** — no pricing impact; price per tile stays `price × quantity`

### Installer builds

Both MSI and NSIS built successfully via `cargo tauri build` (zero errors, ~5-10 min first build, ~2-4 min incremental). Signed with `CN=AUZ Tech` + RFC3161 timestamp.

---

## 🔥 Session 2026-09-06: Suppliers Module — Full Implementation & Wiring

**Status**: ✅ **COMPLETE & VERIFIED.** Suppliers module fully operational end-to-end — DB migration, Rust backend (10 IPC handlers, 8 tests), frontend wiring (types, API hooks, realtime events), UI page, and nav entry.

### What was built

| Phase | Files | Summary |
|-------|-------|---------|
| **Phase 1: DB** | `007_suppliers.sql` | `suppliers` table (id, name, phone, email, company, address, notes, outstanding_balance) + `supplier_purchases` + `supplier_purchase_items` tables (FK cascade, indexes) |
| **Phase 2: Rust Backend** | `repositories/suppliers.rs`, `commands/suppliers.rs` | 10 IPC handlers: `list_suppliers`, `get_supplier`, `create_supplier`, `update_supplier`, `delete_supplier`, `list_supplier_purchases`, `get_supplier_purchase`, `get_supplier_purchase_with_items`, `create_supplier_purchase`, `mark_supplier_purchase_paid`. Tests: 8/8 pass |
| **Phase 3: Frontend Types & Hooks** | `lib/api-client.ts`, `lib/tauri-events.ts`, `features/suppliers/api.ts` | Types: `Supplier`, `CreateSupplierInput`, `SupplierPurchase`, `SupplierPurchaseItem`, `CreateSupplierPurchaseItemInput`, `CreateSupplierPurchaseInput`, `ListSupplierPurchasesParams`, `SuppliersApi`. 9 React Query hooks. Realtime `"suppliers:changed"` event listener |
| **Phase 4: UI Page** | `routes/_authenticated/admin.suppliers.tsx` | ~770 lines: Supplier Directory (add/edit/delete with inline edit form), Record Purchase (line items, discount, payment method), Purchase History (list with paid status, mark-as-paid button) |
| **Phase 5: Wiring** | `lib.rs`, `commands/mod.rs`, `repositories/mod.rs`, `events/emitter.rs`, `events/mod.rs`, `AdminShell.tsx` | Registered 10 IPC handlers in `invoke_handler`, added `pub mod suppliers` to commands + repositories mod.rs, added `emit_suppliers_changed` to emitter + mod re-exports, added Suppliers nav entry (Truck icon, `adminOnly: true`) |

### Bugs fixed during implementation

| Bug | Root cause | Fix |
|-----|-----------|-----|
| Blank white screen after adding suppliers page | `useSuppliersRealtime` imported but not exported from `tauri-events.ts` | Added the hook + `"suppliers:changed"` to `TauriEventName` union |
| `"Command create_supplier not found"` | Supplier commands not registered in `lib.rs` `invoke_handler` | Added all 10 supplier commands to `generate_handler![]` |
| `cannot find suppliers in commands` | `pub mod suppliers` missing from `commands/mod.rs` | Added module declaration |
| `cannot find suppliers in repositories` | `pub mod suppliers` missing from `repositories/mod.rs` | Added module declaration |
| `emit_suppliers_changed not found` | Function missing from `events/emitter.rs` | Added `emit_suppliers_changed` function + re-export |
| TS error `CreateSupplierPurchaseItemInput` not exported | Type inlined inside `CreateSupplierPurchaseInput` items array | Extracted to standalone `CreateSupplierPurchaseItemInput` interface |
| Suppliers tab missing from sidebar | Nav entry never added to `AdminShell.tsx` | Added `{ to: "/admin/suppliers", label: "Suppliers", icon: Truck, adminOnly: true }` |

### Key patterns confirmed

- IPC command names: `snake_case` (e.g., `create_supplier`, `list_supplier_purchases`)
- Types defined in `api-client.ts` (not feature files)
- `getSupplierPurchaseWithItems` destructures Rust tuple `[SupplierPurchase, SupplierPurchaseItem[]]` → `{ purchase, items }`
- Route: `src/routes/_authenticated/admin.suppliers.tsx`
- Admin nav: `adminOnly: true` for financial/admin sections

---

## 🔥 Session 2026-08-25: Release-build signin freeze — RESOLVED (Option B)

**Status**: ✅ **FIXED & VERIFIED.** Root cause = react-dom v19 production event-dispatch
machinery (`findInstanceBlockingEvent` + Suspense-marker walkers) entering an infinite
synchronous loop when input events land during early commit windows. Fix = pin
`react`/`react-dom` to **18.3.1 exact** — different dispatch generation, trap does not exist.

### ✅ Option B execution log (final, 2026-08-25 late evening)

| Step | Result |
|------|--------|
| Pin react/react-dom `18.3.1` exact; npm install | ✓ versions confirmed in node_modules |
| `npx tsc --noEmit` vs baseline | ✓ identical 20 lines |
| SPA rebuild | ✓ `index-MycxdKGo.js` 869 KB (React 18 verified inside: "18.3.1" ×4, "19.x" ×0) |
| Exe rebuild `--features custom-protocol` | ✓ serving gate: `http://tauri.localhost/` via CDP |
| Verification on E/A/D-still-active build | storm 8 s ×100 rounds ALIVE; typing exact; ×5 boot+immediate-storm all clean; owner manual login + sample sale OK |
| **Mitigation revert** (E deferred-mount, A delayed-Toaster, D one-shot adminExists) → back to committed originals | rationale below |
| Re-verification of reverted final build | storm 75/63 rounds ALIVE; typing `owner@check.com`/`final@check.pk` exact; CPU clean; tsc still 20-line baseline |

**Why E/A/D were reverted (do not re-add):**
- All three proved useless against the freeze (it reproduced with each and with all combined)
— React 18 alone is the fix; the hacks only added startup delay and indirection.
- **D contained a real latent bug**: module-level cache never invalidates and register's
`invalidateQueries(["admin-exists"])` became a no-op → after creating the first admin and
logging out, the signin page would show "first-time admin setup" instead of sign-in.
Reverted to the battle-tested React Query hook.

**Gotcha hit during rebuild:** cargo `os error 5` because the running exe locked the file —
kill the app before `cargo build --release`. Also: first launch after a rebuild can exceed
15 s to expose its CDP page target (cold start); poll longer before declaring failure.

**Ship sequence from here:** `cargo tauri build` → signtool re-sign both bundles → verify
signer/timestamp → full manual regression matrix → clean-machine install test → staff docs.

---

### Evening continuation — every fix tried, in order (ALL insufficient for the freeze)

| # | Attempt | Exact change | Files touched | Verification performed | Outcome |
|---|---------|--------------|---------------|------------------------|---------|
| 1 | **F1 — React downgrade within v19** | `"react": "19.1.9"`, `"react-dom": "19.1.9"` exact pins (no caret) | `package.json`, `package-lock.json` | `npm install` clean; installed versions confirmed 19.1.9; `npx tsc --noEmit` = baseline only (baseline saved `%TEMP%\opencode\tsc-baseline.txt`, 20 lines); bundle hash `CNK14Fo3`→`POTYyE8N` (945→932 KB) | ❌ Still froze on regression run ("typing email"). An earlier "5/5 clean" verdict was RETRACTED — see measurement-traps section |
| 2 | **E — deferred mount** | Render `<App/>` only after `window.load` + double-rAF + 300 ms settle, so WebView2's boot-time native focus storm fires against an empty root | `src/tauri-entry.tsx` (new SPA entry file) | Code on disk verified; bundle rebuilt `index-CANUfJLx.js` | ❌ Freeze reproduced with E active |
| 3 | **D — kill guaranteed late-boot commit window** | `useCheckAdminExists()` rewritten from React Query (`staleTime:0, refetchOnMount:"always"` — fired IPC + re-render on EVERY signin mount) to module-cached one-shot fetch (`useState`+`useEffect`, single IPC per app process) | `src/features/auth/api.ts`; side effect: register's `invalidateQueries(["admin-exists"])` is now a harmless no-op | tsc diff vs baseline: same error line-shifted only, 0 new | ❌ Freeze reproduced with D active (user froze typing email seconds after launch ⇒ commit windows persisted past boot regardless) |
| 4 | **A — delayed Toaster** | sonner `<Toaster>` now mounts via `DelayedToaster` after 1.5 s instead of initial commit (toasts fired in first 1.5 s are dropped — acceptable) | `src/main.tsx` | tsc diff clean; bundle rebuilt | ❌ Freeze reproduced with A active |

Combined E+D+A+19.1.9 build: **still froze on first input**, renderer pegged
**+5.23 s CPU over 5 s wall (~105 % of one core)** while host sat idle at 0.28 s —
identical signature to original diagnosis.

### 🚨 Infrastructure bug discovered along the way — every installer ever built was a dev-mode shell (FIXED)

While verifying attempt #2 the exe showed `ERR_CONNECTION_REFUSED` for localhost.
Investigation chain (all source-verified):

1. Nothing listened on :8080 (netstat); vite preview lives on :4173.
2. Tauri source: `tauri-codegen-2.6.3/src/context.rs:155` → `dev: cfg!(not(feature = "custom-protocol"))`;
   `context.rs:178` → when `dev && dev_url.is_some()`: **zero assets embedded**, exe loads `build.devUrl`
   (`http://localhost:8080`) at runtime.
3. The `cfg!(feature=…)` inside injected context code evaluates against **THIS crate's** features — not the
   `tauri` dependency's. This project's `Cargo.toml` never had the official template's feature declaration,
   so plain release builds were ALWAYS dev-mode shells.
4. **Implication**: the signed Aug-24 v1.0.0 MSI/NSIS were also dev-mode shells — they could only ever work
   beside a live dev server on :8080 and would fail identically on any clean machine. The clean-install test
   had never been run, which is why this survived unnoticed until tonight.

**Fix applied** (canonical template block):
```toml
[features]
default = []
custom-protocol = ["tauri/custom-protocol"]
```
Production builds must now be either `cargo tauri build` or
`cargo build --release --features custom-protocol`. Serving mode verified end-to-end via CDP:
page URL changed from `http://localhost:8080/` → **`http://tauri.localhost/`** ✓, exe size 23.1→24.2 MB
(embedded assets). A dead-end intermediate attempt (`[target."cfg(not(debug_assertions))".dependencies]
tauri = { features = ["custom-protocol"] }`) flipped only the dependency's feature, not this crate's —
removed again in favor of the `[features]` block above.

### Measurement traps that produced false verdicts today (do not repeat)

- **"5/5 clean" retraction**: the round-1 verdict ran against a dev-shell exe whose page was served by
  whatever answered on :8080 at that moment — it validated nothing about the production path. Any verdict
  requires the CDP serving check below.
- **Byte-gate flaw**: scanning the exe for plaintext asset-hash strings proves nothing — embedded assets are
  compressed. The reliable gate is launching with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223`
  and reading `/json` from :9223: `http://tauri.localhost/…` = production ✓, `http://localhost:8080/…` = broken ✗.
- **CDP harness artifacts**: enabling `Debugger.enable` *before* navigation reported wedges on bundles known
  good afterward; blind-pause capture also proved unreliable once wedged (`Runtime.enable` times out; pause
  events never surfaced within 15 s windows). CDP alone must never gate a ship decision — human typing on the
  real exe is the arbiter.
- **OneDrive sync lag**: immediately after an edit, shell tools briefly saw stale file content (manifest edit
  invisible to `Select-String`/cargo while `Read` saw it). If cargo no-ops right after an edit, re-check the
  file from the same shell before theorizing.

### Current exact file state (everything UNCOMMITTED)

| File | State |
|------|-------|
| `package.json`, `package-lock.json` | react/react-dom pinned `"18.3.1"` exact (Option B fix — **do not bump to 19 without re-running the signin storm gauntlet**) |
| `src/tauri-entry.tsx` | restored to plain immediate `createRoot` mount (E reverted) |
| `src/main.tsx` | restored to committed HEAD — direct `<Toaster>` (A reverted) |
| `src/features/auth/api.ts` | restored to committed HEAD — React Query `useCheckAdminExists` (D reverted, incl. its latent logout-mode bug) |
| `src-tauri/Cargo.toml` | `[features] custom-protocol = ["tauri/custom-protocol"]` added |
| `src/routeTree.gen.ts` | regenerated by builds |
| `src-tauri/tauri.conf.json`, `index.html`, `vite.config.spa.ts`, `dist-spa/` | SPA pipeline (from previous session, uncommitted; `dist-spa` is build output — add to `.gitignore` when committing) |
| `HANDOFF.md` | this document (uncommitted by owner choice, standing rule) |
| `dist-spa/assets/index-MycxdKGo.js` | current fixed bundle (React 18.3.1) |

Tooling left in `%TEMP%\opencode\`: `cdp.mjs`, `cdp-profiler.mjs`, `cdp-pause.mjs`, `cdp-hunt.mjs` (original
session), `cdp-hunt2.mjs` (injected-click hunt — keep for Option A), `tsc-baseline.txt`. A vite preview server
(node PID ~13688) may still be listening on :4173 — kill freely.

### ⛔ Quarantine — LIFTED 2026-08-25 late night

The old freeze-capable installers were **replaced** by a fresh `cargo tauri build` on the
React 18.3.1 source: both bundles rebuilt, ship-gate passed on the freshly built exe
(serving = `tauri.localhost`, 64-round click storm ALIVE, exact typing, CPU clean), then:

| Artifact | Signed | Signer | Timestamp |
|----------|--------|--------|-----------|
| `bundle/msi/City Tiles POS_1.0.0_x64_en-US.msi` (10.3 MB) | ✓ signtool sha256 | CN=AUZ Tech | RFC3161 DigiCert, valid to 2036-09-04 |
| `bundle/nsis/City Tiles POS_1.0.0_x64-setup.exe` (6.7 MB) | ✓ signtool sha256 | CN=AUZ Tech | RFC3161 DigiCert, valid to 2036-09-04 |

`Get-AuthenticodeSignature` shows Status=UnknownError on self-signed roots — expected,
same as the Aug-24 signing; signer identity + timestamp are authoritative.

**Before distributing:** run the clean-machine install test (install → login → one sale →
uninstall/reinstall keeps DB) and the full manual regression matrix below.

### Next-step decision point (where we paused)

Two options were on the table when the owner paused:

| | Option A — surgical bisect | Option B — React 18.3.1 |
|---|---|---|
| Move | Strip signin route to bare `<input>`; rebuild + type-test; re-add components (Input→Label→Button→router extras) each round until wedge appears | Pin react/react-dom `18.3.1` — different dispatch machinery sidesteps the defect entirely |
| Cost | 3–4 rounds × ~10 min ≈ 40 min | One round ≈ 15 min |
| Risk surface | Precise culprit named; minimal final diff | Peer ranges verified compatible (`"^16.8 \|\| ^17 \|\| ^18 \|\| ^19"` seen in lockfile for Radix/TanStack packages); full manual matrix mandatory anyway |

Recommendation recorded at pause time: **Option B first**, Option A as fallback if 18 shows any regression.
Either way the ship sequence afterwards is identical: rebuild → 10-launch rapid type-test (click email ≤1 s
after window) → `cargo tauri build` → re-sign both installers → full manual matrix → staff docs.

### Root cause

**React 19.2.8 production bundle enters an infinite synchronous loop inside
`react-dom`'s event-dispatch machinery (`findInstanceBlockingEvent` /
Suspense-boundary comment-marker walkers) when a pointer/focus/input event lands
during the initial render window of the signin page.**

Evidence chain (all captured live via Chrome DevTools Protocol):

1. Frozen app: renderer `msedgewebview2.exe` pegged ~92% CPU with **zero**
   interaction; host `city-tiles-pos.exe` idle; browser-process CDP responds,
   page/renderer CDP never answers ⇒ renderer JS thread wedged, Rust healthy.
2. Same `dist-spa` bundle served by `vite preview` in plain Edge reproduces:
   renderer pegged >100% CPU, `Runtime.evaluate` never returns.
3. `Debugger.pause` interrupt (works mid-loop) captured live stacks, repeatedly:
   `#0 vf (Suspense `$`/`$!` marker sibling-walker)` ← `lt` ← `Ad
   (findInstanceBlockingEvent, `a:for(;;)` fiber walk)` ← `_p/hp (dispatch)`.
   Source snippets confirm react-dom internals verbatim.
4. Non-deterministic per load in Edge (only wedges when an event arrives in the
   vulnerable window); deterministic in-app because window creation fires a
   native focus event into the page at boot — the user's click just hits the
   same trap later. Explains "freezes when I click the input".
5. App code exonerated: signin page is plain controlled inputs;
   grep found zero `while`/busy patterns; api-client circuit breaker is async-only;
   lovable-error-reporting is inert outside the editor.

Dev never reproduced because `cargo tauri dev` serves the TanStack Start
pipeline + dev React — the shipped SPA bundle path was never exercised before.

### Fix plan (Phase B) — F1 tried & INSUFFICIENT; decision pending (A-surgical-bisect vs B-React-18, see "Next-step decision point")

| # | Option | Expected efficacy | System-wide implications |
|---|--------|-------------------|--------------------------|
| F1 | ~~Pin `react`/`react-dom` to **19.1.9**~~ **TRIED 2026-08-25 — INSUFFICIENT** (see evening log) | Expected High; actual: race persists on 19.1.9 | API-compatible downgrade applied cleanly; freeze reproduced |
| F2 | If F1 still wedges: deterministic repro harness (synthetic `Input.dispatchMouseEvent` during load ×N) then bisect trigger surface (Toaster/sonner, `scrollRestoration`, router preload) | Medium | Each candidate is small frontend change; same regression cost per iteration |
| F3 | Last resort: React major downgrade to 18.3.x | Highest safety margin | ❌ Avoid unless forced — Radix v-latest + TanStack packages assume React 19; large blast radius |

Rejected: global `stopPropagation` shims / disabling events (breaks Radix menus),
CSP/GPU/IME mitigations (ruled out by evidence).

### Verification protocol after any fix (mandatory)

1. Automated wedge hunt: serve built bundle via `npx vite preview --config
   vite.config.spa.ts`, attach CDP, load ≥10 times injecting focus/click during
   the first seconds — require 0 wedges (harness scripts kept in
   `%TEMP%\opencode\cdp-hunt.mjs`; pattern documented here).
2. Built-exe smoke gate: run release exe, type through signin, login, POS smoke.
3. Full manual regression matrix (below) — any dependency bump invalidates it.
4. Rebuild installers (`cargo tauri build`) + re-sign (signtool, AUZ Tech cert);
   re-run clean-machine install test.

### Original diagnostic log (kept for the record)

Symptom: raw release exe renders signin; clicking any input freezes webview
content instantly; native chrome (drag/close) stays responsive ⇒ host + tao
event loop alive; freeze confined to WebView2 content/input path.

Context: installer fix (uncommitted) swapped bundled frontend pipeline —
`tauri.conf.json` build → `npx vite build --config vite.config.spa.ts` →
`dist-spa/`, mounted by new `src/tauri-entry.tsx`. Dev serves a different
pipeline (`vite.config.ts` / TanStack Start) — shipped JS bundle had never been
smoke-tested interactively.

Hypotheses investigated: H1 IME/TSF deadlock — DISPROVEN (wedges in Edge with
EN-US layout, no IME involvement); H2 production-bundle JS storm — CONFIRMED
(above); H3 GPU compositor — DISPROVEN (compositor paints fine during drag;
loop is main-thread JS); H4 profile corruption/AV — DISPROVEN (fresh Edge
profile wedges identically).

Environment facts gathered: WebView2 Runtime 151.0.4129.107 (current); machine
has ur-PK phonetic TIP alongside EN-US (irrelevant per H1 disproval); UDF at
`%LOCALAPPDATA%\com.citytiles.pos` (standard, not OneDrive-synced); no hang/
crash reports in Event Log (host never hangs — consistent).

### Guardrails

- If Tauri `devtools` feature is enabled temporarily for debugging → **strip
  before building the shipping installer** (console exposes privileged invoke).
- New mandatory gate after every `cargo tauri build`: run the built exe and type
  through signin. Dev/prod pipelines differ; dev-green ≠ ship-safe.
- After the fix lands: full manual matrix + verification protocol above +
  root-cause note appended here.

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
| Clean Windows machine install test | Installer ready (see Release artifacts below) |
| Thermal printer (80mm) live test of **raster** receipt output | Printer on-site; text path verified working via spooler RAW; new raster layout (fonts/margins/spacing) needs one paper test — tune constants in `receipt_bitmap.rs` if taste off |
| `cargo tauri build` MSI/NSIS with self-signed cert ("AUZ Tech") | ✅ DONE 2026-08-24 — built + signed + timestamped |
| Staff quick-start documentation | Pending |

---

## 📦 Phase C execution plan (agreed 2026-08-24)

1. ✅ **DONE** — Self-signed code-signing certificate `CN=AUZ Tech` created in `Cert:\CurrentUser\My`
   (thumbprint `FEC9024DFACAA95FCC92B710001378EA4170E6E7`, valid 5 years). Backup exported to
   `%PROGRAMDATA%\CityTiles\cert-backup\AUZTech.pfx` (password owner-chosen, NOT stored on disk).
2. ✅ **DONE** — `cargo tauri build`: both bundles produced under
   `src-tauri/target/release/bundle/`.
3. ✅ **DONE** — Both installers signed via signtool (`SHA-256`, DigiCert RFC3161 timestamp so
   signatures outlive the cert): verified embedded signer `CN=AUZ Tech`. Trust-chain errors from
   `signtool verify /pa` / `Get-AuthenticodeSignature` are expected for a self-signed root.
4. ⏳ **Owner manual tests** (checklists above): thermal paper test (+ taste tuning of layout
   constants in `receipt_bitmap.rs` per feedback), autostart + single-instance, clean-machine
   install from the fresh installer, full regression matrix incl. the search & retention items.
5. ⏳ **Staff quick-start documentation** — written last so it reflects the final receipt layout.

### Release artifacts (2026-08-24, v1.0.0)

| File | Location |
|------|----------|
| MSI | `src-tauri/target/release/bundle/msi/City Tiles POS_1.0.0_x64_en-US.msi` |
| NSIS setup | `src-tauri/target/release/bundle/nsis/City Tiles POS_1.0.0_x64-setup.exe` |

Known caveats: SmartScreen still warns customers on self-signed certs (accepted, internal use);
timestamping means no expiry warnings later.

⚠️ HANDOFF.md itself remains uncommitted by owner choice; all code was committed 2026-08-24
(squashed "Initial commit" e718245).

### Installer & client-update facts (confirmed 2026-08-24)

- **Fully local runtime**: the frontend bundle is embedded inside the binary; SQLite is statically compiled via sqlx; receipt fonts embedded via `include_bytes!`. Client machines need no Node/Vite/cargo/dev tools — those exist only where installers are built. Sole runtime dependency is WebView2 (preinstalled on Win10/11; the Tauri MSI/NSIS bootstrapper auto-installs it if missing — one-time internet only in that edge case). Fully offline after install.
- **Update procedure (manual; no auto-updater configured)**: bump `version` in `tauri.conf.json` + `Cargo.toml` → `cargo tauri build` on the dev PC → transfer installer → run on the client machine; it upgrades in place. All state that matters — DB, `jwt.key`, backups — lives in `%PROGRAMDATA%\CityTiles\`, outside the install folder, and survives every update untouched. Pending migrations auto-apply on first launch after an update (the immutable-migrations rule exists precisely to keep this safe across installed clients). Recommended ritual each update: Settings → Create backup first. Future option if push-updates are ever wanted: `tauri-plugin-updater` with a signed manifest URL.

---

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
| Architecture Plan | **Mandatory read of `PROJECT_ARCHITECTURE.md` before any changes/fixes** — maps full IPC topology, SQLite schemas, receipt pipelines, and React 18.3.1 pin to prevent breaking connected components (2026-08-29) |
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
| Tile area | **Display only** — `area_per_tile` on products (REAL), `total_area` on invoice items (area_per_tile × qty, computed at checkout); 3 decimal places; bold on receipt; no pricing impact (2026-09-16) |
| React version | **18.3.1 pinned (exact)** — react-dom 19 production builds wedge the signin page in an infinite event-dispatch loop in this app shape; ANY future React upgrade must re-pass the signin storm gauntlet + built-exe type-test first (2026-08-25) |
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
