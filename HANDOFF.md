# Handoff Document — Moon Pipe and Sanitary POS

> ⚠️ **MANDATORY RULE FOR ALL AGENTS & DEVELOPERS**: 
> **You MUST read [PROJECT_ARCHITECTURE.md](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/PROJECT_ARCHITECTURE.md) before making ANY code changes, fixes, refactors, or feature upgrades.** It maps out the complete system layout, IPC bridges, database schemas, and critical component dependencies to prevent breaking connected features.

> **Goal**: Convert to single executable, auto-start, fully offline, loosely coupled
> **Constraint**: Zero functional changes to core POS behavior
> **Status**: 🟢 **v1.1.2 BUILT AND SIGNED (rebuilt) — awaiting install.** Backup & Restore hardening **implemented** (see 🗄️ session below): restore is now a 10-step guarded swap (live-DB guard, WAL sidecar staging, migration-version check, fatal pre-restore snapshot, rollback on failure); `list_backups` is admin-gated; ms-resolution backup filenames; Settings gets a real error state. Gates: `cargo test --lib` **63 / 0 / 1** (was 55), clippy **38** (baseline 39), lint **0 errors / 6 warnings**, tsc **14** (= baseline), `npx tauri build` EXIT 0, signtool sign+verify ×2 = **Successfully verified**. Install **1.1.1 first, then 1.1.2** (1.1.1 fixes the CHECK crash; 1.1.2 adds the bulk path + hardened restore).
> **Next**: user manual UI smoke (Gate 7b) + git commit (user handles). ⛔ **Never roll back below 1.1.0** — migrations 016-018; a build lacking them exits at launch.
> **Date**: 2026-10-08
> **Phase**: Phase C release — multi-line returns + backup/restore hardening

---

## 🗄️ Session 2026-10-08 (release 1.1.2): Backup & Restore hardening

**Status**: ✅ **IMPLEMENTED, BUILT, SIGNED — all gates green.** Scope agreed with the user: **all three tiers**, picker option **A** (kept the manual "Choose backup file…" button), version stayed **1.1.2** (rebuilt over the earlier 1.1.2 artifacts as planned).

**Files touched (exactly the planned set):**
`src-tauri/src/commands/backup.rs` · `src-tauri/src/services/retention.rs` · `src/routes/_authenticated/admin.settings.tsx` · `HANDOFF.md` · `PROJECT_ARCHITECTURE.md` · `PRE_DEPLOY_FIX_PLAN.md`

> ⛔ **Downgrade rule (from §3, now documented in both docs):** never roll the app back below **1.1.0** — migrations 016-018 shipped in `adf0a39`, and sqlx validates every applied migration on every launch, so e.g. the old 1.0.0 installer **exits instead of booting**. 1.1.0 / 1.1.1 / 1.1.2 are mutually restorable.

---

### ✅ Results

**Backend (`commands/backup.rs`, full rewrite):**
- 5 helpers extracted: `ensure_not_live_db` (H1), `prepare_stage` (H2), `validate_stage` (M3+M4), `remove_stale_wal_files` → `Result<(), String>` (M1), `rollback_to_snapshot` (M2).
- New 10-step `import_database` per the table in §5 below — every step implemented as specified (stage copy now includes `-wal`/`-shm`; pre-restore snapshot FATAL; swap failures roll back; `emit_database_restored` fires only on the true success path).
- `REQUIRED_TABLES` 8 → 7 (`app_settings` dropped → M4); validation SQL generated from the constant at runtime.
- L1: `list_backups` renamed `_auth` → `auth_header` + `require_admin` (load-bearing — the frontend sends `authHeader`).
- L2: `timestamp()` → `%Y%m%d_%H%M%S%.3f`; same in `retention.rs:49`.

**Frontend (`admin.settings.tsx`):**
- F2: `backups.isError` branch with the real error message + Retry that calls `resetCircuitBreaker("backups")` before refetching.
- L7: dialog copy no longer says "This cannot be undone" — it now points at the automatic `pre_restore` snapshot in the backups list as the rollback path.

**Tests:** 8 new in `backup.rs` (as planned) + the 2 existing `remove_stale_wal_files` tests updated to the `Result` signature → `cargo test --lib` **63 passed / 0 failed / 1 ignored** (baseline 55/0/1). Notably `prepare_stage_folds_wal_sidecars_into_a_self_contained_file` is the H2 proof (row living only in the `-wal` survives staging; header byte 18 == `1`, no sidecar left).

**Deviations from the plan (all minor, none behavioral):**
- `Path::eq_ignore_ascii_case` doesn't exist in std → `ensure_not_live_db` canonicalizes both paths and compares `to_string_lossy().eq_ignore_ascii_case` (also resolves Windows case/8.3 aliases — the test uses `MOONPIPE.DB` vs `moonpipe.db`).
- `validate_stage` binds `i64` (not `String`) for the `COUNT(*)` IN-list; migration guard query uses `query_scalar` too.
- `wal_checkpoint(TRUNCATE)` issued via `fetch_all` (may return zero rows on a non-WAL source) + `journal_mode=DELETE` asserted case-insensitively after `fetch_one`.
- clippy flagged `unnecessary_sort_by` in the new `list_backups` → fixed with `sort_by_key(Reverse(..))`, so clippy ended at **38** (baseline 39).

**Gates (all green):**

| Gate | Result |
|---|---|
| 1. `cargo check --all-targets` | ✅ 0 errors, no new warnings |
| 2. `cargo test --lib` | ✅ **63 / 0 / 1** (baseline 55/0/1) |
| 3. `cargo clippy --all-targets` | ✅ 0 errors, **38** warnings (baseline 39) |
| 4. `npm run lint` | ✅ 0 errors / 6 warnings (= baseline) |
| 4b. `npx tsc --noEmit` | ✅ exactly **14** (= baseline) |
| 5. `npx tauri build` | ✅ EXIT 0 — MSI + NSIS rebuilt (overwrote the earlier signed 1.1.2 as planned) |
| 6. `signtool sign` ×2 + `verify /pa` ×2 | ✅ both **Successfully verified**, sha256/RFC3161, 0 errors / 0 warnings |
| 7. Doc encoding | ✅ UTF-8, no BOM, 0 U+FFFD (all edits via `edit` tool) |

**Artifacts (both signed):**
- `src-tauri\target\release\bundle\msi\Moon Pipe POS_1.1.2_x64_en-US.msi`
- `src-tauri\target\release\bundle\nsis\Moon Pipe POS_1.1.2_x64-setup.exe`

**Docs updated (§9):** `PROJECT_ARCHITECTURE.md` IPC rows now name the real commands (`export_database`/`import_database`/`list_backups` + `api.backups.*`, purge row → `retention.rs::purge_old_sales`), line ~115 drops "Purge", settings row → "Backup & Restore Settings", downgrade rule added by the migrations tree; `PRE_DEPLOY_FIX_PLAN.md` Gate 7c → ✅ with the 2026-10-08 evidence + status line updated.

**Still to run by hand:** Gate 7b UI smoke; **git commit is the user's** (nothing committed by the agent).

---

### 1. Why (the audit)

Read `commands/backup.rs` (all 291 lines), `database/connection.rs`, `services/retention.rs`, `admin.settings.tsx`, `route.tsx`, plus on-disk evidence and the app log.

**The normal flow is sound. Do not re-audit it.**

- Exports use `VACUUM INTO` off the live pool ⇒ transactionally consistent.
- Verified on disk: both existing backups are **`rollback(delete)` journal, `change_counter=1`, zero `-wal`/`-shm` sidecars** — fully self-contained, nothing to lose later.
- Restore validates (`integrity_check` + required tables) **before** touching the live DB, auto-snapshots first, closes the old pool, clears stale WAL, reopens + migrates + seeds, emits `database:restored` (frontend invalidates *all* queries, `route.tsx:47-49`).
- **A real restore already ran today** — log `2026-10-08 10:47:54 UTC` (= 15:47:54 local) matches `pre_restore_20261008_104754.sqlite`, **zero errors**, live DB 233,472 B / 57 pages / `schema_cookie 66` identical to both backups.

Everything below is an **edge case**, ranked by how likely it actually is.

### 2. Findings

| # | Sev | Issue | Probability | Impact |
|---|---|---|---|---|
| **H1** | High | Picker accepts `.db` and nothing rejects the **live `moonpipe.db`** as a restore source | Low, but the only realistic one — `admin.settings.tsx:57-66` filter allows `.db`, and HANDOFF lines ~118/215 tell the user to "Backup `C:\ProgramData\MoonPipe\moonpipe.db`" | **Silent permanent loss** of recent un-checkpointed commits |
| **H2** | High | Stage copies only the main file; a WAL-mode source loses its sidecar WAL. Validation runs *with* WAL replay but the swap copies *without* it | Same as H1 (hand-copied file) | Silent data loss |
| **M1** | Med | `remove_stale_wal_files` ignores failures (`let _ =`), yet its own comment says a leftover old WAL would be **replayed into the fresh DB** | Very low (<1%) — a foreign `-wal` handle almost always implies a `moonpipe.db` handle too, and that path already fails safely | Split-brain corruption in theory |
| **M2** | Med | If `open_pool` fails post-swap, `*guard` still holds the **closed** pool; `pre_restore_*.sqlite` exists as recovery but nothing says so | Low — realistic trigger is **disk full** (correlated with unbounded backup growth) | App dead until restart; recovery undocumented |
| **M3** | Med | No migration-version guard — failures surface late at `open_pool`, post-swap | Low (same trigger as M2) | Confusing error, no corruption |
| **M4** | Med | `app_settings` is in `REQUIRED_TABLES` (`:119`) so pre-`016` backups are rejected, while `:161-171` is written to handle exactly those — **dead branch** | Very low (needs a backup older than 2026-10-06; both on-disk files have `schema_cookie=66`) | Misleading error, not corruption |
| **L1** | Low | `list_backups` does not call `require_admin` (export/import do) | Negligible (single machine, admin-only UI) | Info disclosure |
| **L2** | Low | `timestamp()` is 1-second resolution → two snapshots in the same second overwrite | Negligible | One backup lost |
| **L3** | Low | Pre-restore snapshot is best-effort (`log::warn` then continue) while retention purge then runs | Very low — if VACUUM is broken, retention's own snapshot also fails and the purge aborts | Self-limiting |
| **L4** | Low | Unbounded backup growth (`PRE_DEPLOY_FIX_PLAN.md:21`) | Certain, but only disk space | **Already accepted out-of-scope by the user** |
| **L5** | Low | `moonpipe.db.pre_018_backup` sits in `app_data_dir` but matches neither prefix → invisible in Settings | Already true | Harmless |
| **L6** | Low | `PROJECT_ARCHITECTURE.md:517-519` documents commands that **do not exist** | Already true | Doc drift |
| **L7** | Low | Dialog says "This cannot be undone" but `pre_restore_*.sqlite` *is* the undo | Already true | UX only |

### 3. Separate finding — downgrade constraint (document, do NOT code)

Migrations **016, 017, 018 were all added in `adf0a39` (2026-10-06)**, the commit immediately before the **1.1.0** bump (`baf1040`). So:

- **1.0.0** (built 2026-09-22) embeds only migrations **≤ 015**.
- **1.1.0 / 1.1.1 / 1.1.2** embed **016–018** ⇒ mutually restorable ✅

sqlx 0.8.6 validates *every* applied migration on *every* `open_pool` (`sqlx-core-0.8.6/src/migrate/migrator.rs:28-45` → `MigrateError::VersionMissing`, `ignore_missing = false`), and `lib.rs:35-37` propagates it out of `setup` ⇒ **the app exits instead of launching.**

**Consequence: the `Moon Pipe POS_1.0.0_x64-setup.exe` still in `target/release/bundle/` can never boot against the current DB. Never roll back below 1.1.0.** This is a general downgrade constraint, not a backup defect — it goes in the docs only.

### 4. Decision

**Option A — keep the manual "Choose backup file…" picker.** The client may have the backup on a USB and will not dig around in `C:\` for POS source files. Backend guards make the picker safe instead of removing it.

---

### 5. Backend — `src-tauri/src/commands/backup.rs` (bulk of the work)

Refactor `import_database` (currently a monolithic L70-202) into thin orchestration over extracted helpers.

**New operation order:**

| # | Step | Status | Fixes |
|---|---|---|---|
| 1 | `require_admin(&app, auth_header)?` | unchanged | — |
| 2 | `src.exists()` check | unchanged | — |
| 3 | `ensure_not_live_db(&src, &config.db_path)?` — canonicalize both, `Path::eq_ignore_ascii_case`, reject with `AppError::Validation` | **NEW** | **H1** |
| 4 | Stage copy: `src → stage` **plus `src-wal → stage-wal` and `src-shm → stage-shm` when present** | **EXTENDED** | **H2** |
| 5 | `prepare_stage(&stage)?` — open stage **read-write**, `PRAGMA wal_checkpoint(TRUNCATE)`, then `PRAGMA journal_mode=DELETE` and **assert the returned value is `delete`**, close, `remove_stale_wal_files(&stage)?` | **NEW** | **H2** |
| 6 | `validate_stage(&stage)?` — `integrity_check` → required tables (SQL built *from* `REQUIRED_TABLES`) → **migration-version guard** | **MOVED after 5 + EXTENDED** | **M3, M4** |
| 7 | Pre-restore snapshot: `fs::create_dir_all` + `VACUUM INTO` — **now FATAL on failure** (live DB untouched at this point) | **CHANGED** | **L3** (and guarantees the file exists for step 10) |
| 8 | Swap A: `holder.write()` → `old = clone` → `old.close()` → `remove_stale_wal_files(&db_path)?` **FATAL**; on `Err`: `open_pool(&config.db_path)`, install it, return `Err` (original file is still intact here) | **CHANGED** | **M1** |
| 9 | Swap B: `fs::copy(stage → db_path)` (existing revive branch on `Err` unchanged) → `remove_stale_wal_files(&db_path)?` **FATAL + ROLLBACK** → `fs::remove_file(stage)` | **CHANGED** | **M1** |
| 10 | `open_pool(&db_path)` — **Ok**: unchanged tail (`app_settings` ensure → install → drop guard → `emit_database_restored` → spawn retention + reconciliation). **Err**: **ROLLBACK** — copy `pre_restore_*.sqlite` → `db_path`, clear side files, `open_pool` again; success ⇒ install + return `Err("Restore failed and was rolled back to your previous data: {e}")`; failure ⇒ return `Err` **including the snapshot path** | **CHANGED** | **M2** |

**Invariant to preserve:** `emit_database_restored` fires **only** on the true success path. Every failure and every rollback returns before it, so the UI never observes a database it did not receive.

**Helpers to extract:**

```rust
fn   ensure_not_live_db(src: &Path, live: &Path) -> Result<(), AppError>        // H1
async fn prepare_stage(stage: &Path) -> Result<(), AppError>                    // H2
async fn validate_stage(stage: &Path) -> Result<(), AppError>                   // M3 + integrity/tables
async fn remove_stale_wal_files(db_path: &Path) -> Result<(), String>           // M1 — SIGNATURE CHANGE
async fn rollback_to_snapshot(db_path: &Path, snapshot: &Path)
       -> Result<DbPool, String>                                                // M2
```

**Other edits in this file:**

- **M4** — drop `app_settings` from `REQUIRED_TABLES` (L11-20, 8 → 7 entries). Unblocks the existing `CREATE TABLE IF NOT EXISTS app_settings` block at L161-171, which is currently unreachable. Build the `IN (…)` list **from `REQUIRED_TABLES` at runtime** so the constant and the SQL can never drift again (that drift is exactly what created M4):
  ```rust
  let placeholders = vec!["?"; REQUIRED_TABLES.len()].join(",");
  let sql = format!("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ({placeholders})");
  let mut q = sqlx::query_scalar::<_, String>(&sql);
  for t in REQUIRED_TABLES { q = q.bind(t); }
  ```
- **M3 guard** — `sqlx::migrate!("./src/database/migrations").iter().map(|m| m.version).max()` (`Migrator::iter()` is `pub`, `migrator.rs:104`) vs `SELECT COALESCE(MAX(version),0) FROM _sqlx_migrations` on the stage. If stage > embedded → `AppError::Validation("Backup was created by a newer version of Moon Pipe POS (migration {n}) — update the app first")`.
- **L1** — `list_backups(app, db, auth_header)` + `require_admin(&app, auth_header).await?;`
- **L2** — `timestamp()` (L27) → `chrono::Utc::now().format("%Y%m%d_%H%M%S%.3f")`

> 🔴 **F1 — the `auth_header` rename is LOAD-BEARING, not cosmetic.** The frontend sends `authHeader` (`api-client.ts:97`); Tauri maps Rust `auth_header` → `authHeader`. The current param is named **`_auth`**, which maps to `auth`, so **the token never arrives**. Renaming is mandatory — skip it and `require_admin` receives `None` and every `list_backups` call fails with *"Missing or invalid Authorization header"*, breaking Settings → Backups entirely. (`export_database:47` has the correct spelling and is the working reference.)

### 6. Backend — `src-tauri/src/services/retention.rs:49`

Same `%Y%m%d_%H%M%S%.3f` format, otherwise a retention snapshot and a user export can still collide within one second. Prefix/suffix unchanged ⇒ `list_backups` prefix matching (`:222`) is unaffected.

### 7. Frontend — `src/routes/_authenticated/admin.settings.tsx`

- **L7** — replace *"This cannot be undone"* (L180) with a pointer to the automatic `pre_restore_*.sqlite` snapshot in Settings → Backups as the undo path.
- 🔴 **F2 — error state.** `backups.isError` currently falls through to *"No backups yet — create one above."* (L129-131), which is actively misleading. Add an `isError` branch with the message and a Retry that calls `resetCircuitBreaker("backups")`.

  **Why it matters:** `api-client.ts:16-59` — one circuit breaker is shared by **all three** backup commands under `feature: "backups"`, `THRESHOLD = 5` failures / `TIMEOUT = 30_000`. Today `list_backups` never fails; after adding `require_admin` an expired token can fail it on page mount, and 5 failures would block **export and import too** for 30 s. `onSuccess` resets the counter, so a single stray failure is harmless — the danger is only repeated retries.

### 8. Tests — 8 new in `backup.rs` `mod tests` (total **55 → ~63**)

Existing two `remove_stale_wal_files` tests get the new `Result` signature.

| Test | Proves |
|---|---|
| `remove_stale_wal_files_fails_when_side_file_cannot_be_deleted` | Make `x-wal` a **directory** so `remove_file` fails → assert `Err` (M1) |
| `ensure_not_live_db_rejects_same_path_case_insensitively` | `MOONPIPE.DB` vs `moonpipe.db` → `Err` (H1) |
| `ensure_not_live_db_allows_same_name_in_another_folder` | USB/backup copy with the same name → `Ok` (H1) |
| `prepare_stage_folds_wal_sidecars_into_a_self_contained_file` | **The H2 proof.** `open_pool` a temp DB with `wal_autocheckpoint=0`, insert a row (lives only in `-wal`), copy **main + sidecars** to stage, run `prepare_stage` → assert header byte 18 == `1`, no `stage-wal`, and the row **is** readable. Fails without the fix |
| `validate_stage_rejects_newer_migration_backup` | Fake `_sqlx_migrations` row at `max_version + 1` → `Err` containing "newer" (M3) |
| `validate_stage_accepts_backup_without_app_settings` | Stage missing only `app_settings` → `Ok` (M4) |
| `validate_stage_rejects_garbage_file` | Non-SQLite bytes → `Err` |
| `validate_stage_rejects_unrelated_sqlite_db` | Valid SQLite, no MoonPipe tables → `Err` |

Fixtures: `crate::database::open_pool(temp_path)` already runs real migrations + seed ⇒ genuine full-schema DB. Existing `connection.rs` scratch-DB tests prove this pattern works in `--lib` (never touches the live `DATABASE_URL`). Tests run in temp dirs only.

### 9. Docs (Tier 3)

| File | Change |
|---|---|
| `PROJECT_ARCHITECTURE.md:517-519` | Rows claim `create_backup` / `restore_backup` / `purge_old_invoices` + `apiClient.backup.*` — **none exist** → `export_database` / `import_database` / `list_backups` + `api.backups.export/import/list`; purge row → `services/retention.rs::purge_old_sales` (there is no purge command) |
| `PROJECT_ARCHITECTURE.md:115` | `backup.rs # DB Snapshot, Restore & Purge Commands` → drop "Purge" |
| `PROJECT_ARCHITECTURE.md:96` | `admin.settings.tsx` → "Backup & Restore Settings" (there is no retention UI) |
| `HANDOFF.md` | This session block → implementation results, test count, gates, artifacts |
| `HANDOFF.md` + `PROJECT_ARCHITECTURE.md` | **Downgrade warning** from §3 above: never roll back below 1.1.0 |
| `PRE_DEPLOY_FIX_PLAN.md:148` | Gate 7c row → ✅ with the evidence from §1; status line at `:3` |

All doc edits via the `edit` tool (**never `Set-Content -Encoding utf8`**), then verify UTF-8 / no BOM / 0 U+FFFD.

### 10. Impact analysis (done 2026-10-08 — do not redo unless the code moves)

**Complete consumer map — the entire change surface is 2 Rust files + 1 TSX file + 3 docs:**

| Artifact | Consumers |
|---|---|
| `list_backups` | **1** — `admin.settings.tsx:48` via `api-client.ts:643` |
| `export_database` / `import_database` | **1 each** — `admin.settings.tsx:52,69` |
| `BackupInfo` | `api-client.ts:203` — **shape unchanged** |
| Command registration | `lib.rs:133-135` — **names unchanged, no `lib.rs` edit** |
| `remove_stale_wal_files` | 3 call sites in `backup.rs` + 2 tests (`:263`, `:285`) |
| `REQUIRED_TABLES` | 1 use (`backup.rs:119`) |
| `timestamp()` | 2 uses (`backup.rs:55`, `:127`) |
| `moonpipe_backup_` prefix | **written by 2**: `backup.rs:55` *and* `retention.rs:48`; read by `backup.rs:222` |
| `services/backup.rs` | **does not exist** (removed in cleanup Phase 5) |

**Explicitly NOT impacted:**
- v1.1.2 returns feature — zero file overlap; gates 1-6 must still pass unchanged
- **Schema** — no migration files added ⇒ 1.1.2 stays mutually restorable with 1.1.0/1.1.1
- **sqlx live-DB gate** — `backup.rs` contains **zero** `query!` macros (verified) ⇒ `.cargo/config.toml` / `DATABASE_URL` untouched, no `.sqlx` churn
- Auth / JWT / roles / autostart / single-instance — and **F3**: `require_admin` is **JWT-only, no DB access** (`middleware.rs:8-12` → `validate_token`), so `list_backups` stays safe even while a restore swap holds the write lock
- Existing on-disk backups remain restorable; eslint/tsc baselines unchanged (0/6 and 14)

**Behaviour changes to watch:**
- Restore now **refuses** when it cannot snapshot (L3) — new failure mode, but it fails *before* the swap, live DB untouched; `backups\` is the DB's own parent, so if the DB is writable so is it
- Restore now **aborts** where it used to proceed (M1) — correct, proceeding was the corruption path
- `list_backups` can now 401 ⇒ F2 error state + breaker
- Backup filenames get ~3 ms digits longer (prefix/suffix unchanged, sorting uses `metadata.created`)

**Residual risk NOT being fixed:** power loss during `fs::copy(stage → moonpipe.db)` leaves a truncated live DB (recovery = manual `pre_restore_*.sqlite`). Window ≈ ms for a ~230 KB file. Considered switching the swap to `fs::rename` (atomic replace) — **rejected**: larger change to proven, today-tested code. Raise it only if the user asks.

### 11. Explicitly out of scope

- **L4 backup file pruning** — accepted out-of-scope by the user (`PRE_DEPLOY_FIX_PLAN.md:21`)
- **Option B** (removing the manual picker) — rejected, see §4
- `services/returns.rs::process_return` dead code, `PROJECT_ARCHITECTURE.md` stale `returns`/`return_items` schema, `BEGIN IMMEDIATE` TOCTOU — all pre-existing and untouched
- Gates **7b** (UI smoke) stays manual

### 12. Gates (same framework, `PRE_DEPLOY_FIX_PLAN.md` §D)

| Gate | Expect |
|---|---|
| 1. `cargo check --all-targets` | 0 errors |
| 2. `cargo test --lib` | **~63 passed / 0 failed / 1 ignored** (baseline 55 / 0 / 1) |
| 3. `cargo clippy --all-targets` | 0 errors, no new warnings (baseline 39) |
| 4. `npm run lint` | 0 errors / 6 warnings |
| 4b. `npx tsc --noEmit` | exactly 14 |
| 5. `npx tauri build` | EXIT 0 — **replaces today's signed 1.1.2 artifacts** |
| 6. `signtool sign` ×2 + `signtool verify /pa` ×2 | 0 errors / 0 warnings |
| 7. Doc encoding check | UTF-8, no BOM, 0 U+FFFD |

**Do NOT run `cargo fmt`** (repo is not rustfmt-clean). **No commits/pushes.**

### 13. Execution order ~~for tomorrow~~ (✅ executed 2026-10-08, all steps done — see Results above)

1. `commands/backup.rs` — extract the 5 helpers, wire the new 10-step order, M4 + L1 + L2
2. `services/retention.rs:49` — ms timestamp
3. Run gate 1 (`cargo check --all-targets`) immediately; fix
4. Update the 2 existing tests, add the 8 new ones → gate 2
5. Gate 3 (clippy)
6. `admin.settings.tsx` — copy + `isError` + breaker reset → gates 4 / 4b
7. Docs (§9) + turn this block into the implementation-results block
8. Gates 5 → 6 → 7
9. Hand the git commands to the user — **do not commit**

---

## 🧾 Session 2026-10-08 (release 1.1.2): Atomic multi-line returns

**Status**: 🟢 **CODE + BUILD COMPLETE — awaiting install + manual verification.** Uncommitted at time of writing: `src-tauri/src/repositories/returns.rs`, `src-tauri/src/commands/returns.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/repositories/mod.rs`, `src-tauri/src/services/returns.rs`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src/lib/api-client.ts`, `src/features/returns/api.ts`, `src/routes/_authenticated/admin.returns.tsx`.

### The problem

`admin.returns.tsx` built an array of line payloads and drove `create_return` **once per line from the frontend**:

```ts
for (const p of payload) { await createReturn.mutateAsync(p); }   // old :95-97
```

Each call is its own IPC round-trip and its own SQLite transaction. A guard rejection, auth failure or dropped call on line 3 left lines 1–2 **already committed** — a permanent partial return with stock, ledger and profit all half-adjusted. The v1.1.1 hotfix removed the *most common* cause (the CHECK failure) but could not remove the structural one. It also cost `N` IPC round-trips, `3N` realtime events and ~`11N` `invalidateQueries()` batches per submission.

### The design

One new command becomes the primitive; the old single-line path delegates to it.

```
create_return(cmd)      ──► ReturnRepository::create()      ──► create_bulk(1 line)
create_returns_bulk(cmd)──► ReturnRepository::create_bulk() ──► ONE tx, N lines
```

`create_bulk` flow, all inside a single `pool.begin()` transaction:

1. **Pre-validate before opening the tx** — non-empty `lines`, every `quantity >= 1`, every `unit_price >= 0`.
2. Per line: `line_total = quantity * unit_price` (server-derived) → over-return guard → `INSERT returns` → `UPDATE products` → read invoice → discount proration → `UPDATE invoices` → accumulate `total_reduction` + `customer_id`.
3. **After** the loop: one `propagate_carry_forward(&mut tx, invoice_id, total_reduction)` + one `reconcile_customer(&mut tx, cid)`.
4. `commit` → return the created rows.

Any step failing anywhere drops the transaction ⇒ **zero rows, zero stock change, zero ledger change.**

### Why "once at the end" is provably identical to "once per line"

- `saturating_sub` / `.min()` are order-independent, so `new_subtotal`/`new_paid` chain identically either way.
- `reduction = old_due − new_due` **telescopes**: `Σ(old − new) = due₀ − due_final`. Feeding the sum to `propagate_carry_forward` is the same as feeding each term — with `N` chain walks collapsed to **1**.
- `reconcile_customer` recomputes from the leaf invoices, so running it once yields the same value as running it `N` times.
- The over-return guard reads through *this* transaction, so a line earlier in the loop **is** visible to a later line — duplicate-product lines are handled correctly.

Pinned numerically by `bulk_return_matches_n_sequential_single_returns`: `disc_back = 1000*3000/11000 = 272` then `728*1000/8000 = 91` ⇒ final `7000 / 637 / 6363` on **both** paths.

### Changes

| File | Change |
|---|---|
| `src-tauri/src/repositories/returns.rs` | Added `CreateReturnLineInput` + `CreateReturnsInput`; **`line_total` removed from `CreateReturnInput`**; new `create_bulk()`; `create()` is now a thin 1-line delegate |
| `src-tauri/src/commands/returns.rs` | `CreateReturnInputCmd` minus `line_total`; new `CreateReturnLineInputCmd`, `CreateReturnsBulkInput`, `#[tauri::command] create_returns_bulk` (auth + `get_current_user` + 3 emits, all **once**) |
| `src-tauri/src/lib.rs` | Registered `commands::returns::create_returns_bulk` in `generate_handler!` |
| `src-tauri/src/repositories/mod.rs:15` | Re-exports `CreateReturnsInput`, `CreateReturnLineInput` |
| `src-tauri/src/services/returns.rs` | Dropped the `line_total` param — **this wrapper has no callers** (dead code), so the change is inert |
| `src/lib/api-client.ts` | `ReturnsApi.createBulk`, impl calling `create_returns_bulk`, new `CreateReturnLineInput` / `CreateReturnsBulkInput` |
| `src/features/returns/api.ts` | `useCreateReturnsBulk()`; shared `invalidateAfterReturn()` (same 5 keys as before); `line_total` dropped from the request type |
| `src/routes/_authenticated/admin.returns.tsx` | Loop replaced by one `bulkReturn.mutateAsync({ invoice_id, reason, lines })`; payload no longer carries `line_total`. Pre-validation unchanged |

### Blast radius (audited, all clear)

| Area | Verdict |
|---|---|
| Migration / schema / CHECK 015 | **untouched — no migration** |
| `create_return` command | signature unchanged; still works; existing callers/tests unaffected |
| `reports.rs` return-profit math, `returns.line_total` readers | reads stored value, which the server now computes — same formula |
| print / PDF / receipt / Excel / backup / restore / retention | untouched |
| **suppliers** | separate table, no purchase-return path — untouched |
| Realtime events | ~`3N` → **3**; frontend invalidations ~`11N` → **~11** (`route.tsx:14-34` listener) |
| SQLite locking | `N` write transactions → **1**; shorter exclusive window. `busy_timeout = 5000` kept (`connection.rs:61`) |
| Rollback | additive command ⇒ reinstalling 1.1.1 works; 1.1.1 also remains the rollback target for this release |

### Explicitly NOT done (deliberate)

- **`BEGIN IMMEDIATE`** — the pre-existing deferred-transaction TOCTOU on the over-return guard is unchanged. Safe in practice (guard reads its own tx), flagged as a possible follow-up only.
- **`unit_price` still comes from the client.** The server now derives `line_total` from it, but does not re-read `unit_price` from `invoice_items`. Stricter validation belongs in its own change.
- **`PROJECT_ARCHITECTURE.md` §3 schema listing for `returns`/`return_items` is stale** (pre-existing drift, unrelated to this release).
- Gates **7b** (UI smoke) and **7c** (backup→restore end-to-end) remain manual.

### New tests (5 — total now **55 / 0 / 1**; shared `test_support.rs` untouched)

| Test | Covers |
|---|---|
| `bulk_return_is_all_or_nothing` | line 2 over-returns after line 1 succeeded ⇒ **0 rows, invoice unchanged**; then a payload with `"line_total": 999999` is accepted by serde but the stored value is still `quantity * unit_price` |
| `bulk_return_matches_n_sequential_single_returns` | one bulk call vs two legacy calls on identical invoices ⇒ identical invoice rows, balances and normalized return rows; final `7000 / 637 / 6363` pinned by hand |
| `bulk_return_on_a_discounted_carried_invoice_propagates_once` | 9000 reduction propagated **once** — successor `previous_balance 9000 → 0`, balance `1000 == expected_leaf_balance`, and `reconcile_balances` does not move it |
| `bulk_return_with_duplicate_product_lines_respects_the_guard` | `2 + 2 > 3` ⇒ rejected **and rolled back**; `2 + 1 <= 3` ⇒ both rows land |
| `empty_bulk_return_is_rejected` | `[]`, `quantity 0`, `unit_price -100` all rejected **before** the transaction opens |

### Gates — all green

| Gate | Result | Baseline |
|---|---|---|
| `cargo check --all-targets` | 0 errors, 34 warnings | same |
| `cargo test --lib` | **55 passed / 0 failed / 1 ignored** | 50 / 0 / 1 |
| `cargo clippy --all-targets` | **39 warnings, 0 errors** | 39 |
| `npm run lint` | **0 errors / 6 warnings**, exit 0 | same |
| `npx tsc --noEmit` | **14 errors** (accepted; `admin.returns.tsx` clean) | 14 |
| `npx tauri build` | EXIT 0, 5m55s (vite ✓ 16.75s) | — |
| `signtool sign` ×2 | 0 errors | — |
| `signtool verify /pa` ×2 | **Successfully verified**, 0 errors / 0 warnings, `AZ Solutions`, DigiCert timestamp | — |

### Artifacts (both signed)

- `src-tauri/target/release/bundle/msi/Moon Pipe POS_1.1.2_x64_en-US.msi` (9,904,128 bytes)
- `src-tauri/target/release/bundle/nsis/Moon Pipe POS_1.1.2_x64-setup.exe` (7,312,120 bytes)

### Still to run by hand

1. **Backup** `C:\ProgramData\MoonPipe\moonpipe.db` first.
2. Install **1.1.1**, verify, then install **1.1.2** over it (publisher **AZ Solutions**, file version **1.1.2**).
3. Multi-line return on **≥ 2 products** in one submission → all lines appear in *Recent returns*, stock restored for both, sale total reduced by the sum, Dashboard == Reports.
4. **Negative test:** over-enter a quantity on one line → error toast, and *nothing* is recorded for *any* line (this is the behaviour the old loop could not deliver).
5. Multi-line return on a **discounted** invoice → no CHECK error, invoice ends `subtotal >= discount`, identity `total == subtotal - discount + previous_balance` holds.
6. Multi-line return on an invoice whose balance was **carried forward** → successor invoice shrinks, balance still equals `expected_leaf_balance`.
7. One backup → restore → `PRAGMA journal_mode = wal` + `integrity_check = ok`. Then **gates 7b / 7c** from `PRE_DEPLOY_FIX_PLAN.md` §D.

### Rollback

Reinstall `Moon Pipe POS_1.1.1_x64-setup.exe` (still in `target/release/bundle/`). Additive command + **no migration** ⇒ no data repair; rows written by 1.1.2 are ordinary rows and stay valid under 1.1.1.

---

## 🩹 Session 2026-10-08: v1.1.1 HOTFIX — returns crash on discounted invoices

**Status**: 🟢 **CODE + BUILD COMPLETE — awaiting install + live repro on the customer machine.** Uncommitted at time of writing: `src-tauri/src/repositories/returns.rs`, `tauri.conf.json`, `Cargo.toml`, `Cargo.lock` (version 1.1.1).

### The production bug

Customer ran a return from the Returns module on a discounted invoice and got:

```
Database error: error returned from database: (code:275) CHECK constraint failed: discount >= 0 AND discount <= subtotal
```

- `code:275` = `SQLITE_CONSTRAINT`. The **transaction rolled back** — no partial return, no stock restored, **no data corruption**.
- The rule comes from `src-tauri/src/database/migrations/015_add_invoice_constraints.sql:22` (`CHECK (discount <= subtotal)`).
- Root cause: `repositories/returns.rs` shrank `subtotal` and `total` on a return but **never touched `discount`**. Trigger condition: `line_total > subtotal - discount`.
- **On the live DB this is a 100% repro on any single-line invoice with a discount** — returning the only line drives `subtotal` to 0 while `discount` survives. Candidates: `INV-2026-0007`, `INV-2026-0008`, `INV-2026-0019` (each `subtotal 3500 / discount 300|500`, one Bottle Trap @3500). 5 invoices carry a discount; 0 rows violated the CHECK; 0 of the 11 existing returns sat on a discounted invoice.
- Second, **silent** bug behind it: `new_total = total - line_total` credited the **gross** line value, while the customer had only paid net-of-discount. `commands/reports.rs:125` already values returns at `line_total * (subtotal - discount) / subtotal` — ledger and Reports disagreed by the discount slice.

### The fix — floor proration, backend only

`src-tauri/src/repositories/returns.rs` (the only source file changed):

```rust
let sub = inv.subtotal;
let disc_back = if sub > 0 { inv.discount * input.line_total / sub } else { inv.discount }; // floor
let new_subtotal = (sub - input.line_total).max(0);
let new_discount = (inv.discount - disc_back).max(0).min(new_subtotal);   // hard backstop
let new_total    = new_subtotal - new_discount + inv.previous_balance;
```

`UPDATE invoices` now also writes `discount`. SELECT gained `discount` + `previous_balance`.

**Why it cannot regress anything:**
1. **`discount == 0` is byte-identical to the old arithmetic** — `disc_back = 0`, `new_total = (sub - line) + prev_bal`, and `total == sub + prev_bal` holds on **all 22 live rows** (verified). All 7 pre-existing return tests pass unchanged.
2. **CHECK holds by construction:** with `K = sub - discount ≥ 0`, `new_discount - new_subtotal = ceil(K·line/sub) - K ≤ 0`; `.min(new_subtotal)` is a belt-and-braces backstop.
3. **Reports now agree with the ledger:** the credit equals `line - floor(discount·line/sub)`, the same net figure `reports.rs` uses.

### Blast radius (audited, all clear)

| Area | Verdict |
|---|---|
| `commands/invoices.rs:84-95` insert validation | untouched |
| `invoices.rs:415 propagate_carry_forward` | reads successor subtotal/discount, **never writes them** — receives a now-correct net reduction |
| `services/reconciliation.rs` | only `SUM(total - amount_paid)` + `carried_to` relinking — **never rewrites subtotal/discount/total**, so startup reconcile and backup-restore cannot undo the fix |
| print / PDF / receipt / `admin.reports.tsx:107` / `admin.invoices.$invoiceId.tsx:196` | render current values → prorated discount is correct |
| backup / restore / retention | file-level, no column knowledge |
| **suppliers** | separate table + separate CHECK; no purchase-return path exists → untouched |
| `admin.returns.tsx` | **no change** — "Return value" is goods value, not credit issued |

### Explicitly NOT done (deliberate, for blast-radius control)
- **No migration** — the CHECK stays; it is a correct guard.
- **No frontend change.**
- **No `.sqlx` regeneration** — `src-tauri/.cargo/config.toml` sets `DATABASE_URL` to the live DB and `SQLX_OFFLINE` is unset, so macros validate against the real schema. The orphaned cache entry for the old UPDATE is inert. *If `SQLX_OFFLINE=true` is ever set, re-run `cargo sqlx prepare`.*
- **Multi-line returns are still not atomic** — `admin.returns.tsx:95-97` fires one `create_return` per line in a loop. This fix removes the CHECK failure that stranded partial returns, but a mid-loop auth/network failure still could. **Follow-up, out of hotfix scope.**

### New tests (3, all inside `returns.rs` `mod tests`; shared `test_support.rs` untouched)
| Test | Covers |
|---|---|
| `full_return_of_the_only_line_on_a_discounted_invoice_clears_it` | **the customer's exact bug** — 3500/300/3200 invoice, return the line → row must end `0/0/0` |
| `partial_return_prorates_the_discount_and_matches_reports` | 10000/1000 → return 3000 ⇒ `7000 / 700 / 6300`, identity holds, credit `2700 == 3000*9000/10000` |
| `returns_on_a_discounted_invoice_never_trip_the_check` | drains a 7-line invoice step by step; every step must be `Ok` |

Plus a local `discounted_invoice()` helper (test_support keeps its `discount: 0` contract for the other 40+ tests).

### Gates — all green

| Gate | Result | Baseline |
|---|---|---|
| `cargo check --all-targets` | 0 errors, 34 warnings | same |
| `cargo test --lib` | **50 passed / 0 failed / 1 ignored** | 47 / 0 / 1 |
| `cargo clippy --all-targets` | **39 warnings, 0 errors** | 39 |
| `npm run lint` | **0 errors / 6 warnings**, exit 0 | same |
| `npx tsc --noEmit` | **14 errors** (accepted) | 14 |
| `npx vite build --config vite.config.spa.ts` | ✓ 15.8s | ✓ |
| `npx tauri build` | EXIT 0, 5m38s | — |
| `signtool sign` ×2 | 0 errors | — |
| `signtool verify /pa` ×2 | **Successfully verified**, PS Status **Valid**, `CN=AZ Solutions` | — |

### Artifacts (both signed, `signtool verify /pa` = Valid)
- `src-tauri/target/release/bundle/msi/Moon Pipe POS_1.1.1_x64_en-US.msi` (9,887,744 bytes)
- `src-tauri/target/release/bundle/nsis/Moon Pipe POS_1.1.1_x64-setup.exe` (7,318,608 bytes)

### Still to run by hand
1. **Backup** `C:\ProgramData\MoonPipe\moonpipe.db` before installing.
2. Install `Moon Pipe POS_1.1.1_x64-setup.exe` (publisher shows **AZ Solutions**); confirm file version **1.1.1**.
3. **Live repro:** on `INV-2026-0007` (or `0008` / `0019`) return the single Bottle Trap → must succeed; invoice row must read `subtotal 0 / discount 0 / total 0`.
4. Regression sweep: create a sale **with a discount** → partial-return one line → Dashboard "Sales today" == Reports "Today" → Excel export has the **NET SALES** row → one backup → restore → `PRAGMA journal_mode = wal` + `integrity_check = ok`.
5. Still-open from 2026-10-06: **gate 7b** (5-point UI checklist) and **gate 7c** (backup→restore end-to-end) against an installed build.

### Rollback
Reinstall `Moon Pipe POS_1.1.0_x64-setup.exe` (still in `target/release/bundle/`). Backend-only + no migration means **no data repair**; rows written by 1.1.1 satisfy every CHECK, so they stay valid under 1.1.0.

---

## 🔥 Session 2026-10-06: Pre-Deployment Fix Plan — Phases 2 frontend/3/4, gates, and the v1.1.0 release build

**Status**: 🟢 **COMPLETE except install + gates 7b/7c.** Plan: `PRE_DEPLOY_FIX_PLAN.md`. Phase 2 frontend (2.2/2.3/2.6/2.7) ✅, Phase 3 ✅, Phase 4 ✅, gates 1–6 ✅, gate 7a (WAL) ✅, repo-wide lint ✅, commit `adf0a39` ✅, **v1.1.0 MSI + NSIS built and signed** ✅.

### Done

**Phase 2 frontend ✅ (2.2 / 2.3 / 2.6 / 2.7)**
| Item | Files |
|------|-------|
| `ListInvoicesParams.limit?: number`; new `useInvoicesInRange(fromIso)` (`RANGE_LIMIT = 50_000`, key `["invoices","range",fromIso]`) + `useCustomerInvoices(customerId, enabled)` (`CUSTOMER_LIMIT = 200`, key `["invoices","customer",id]`); `useInvoices()` untouched | `api-client.ts:157-162,549`, `features/invoices/api.ts` |
| Reports: merged imports, `useMemo([])`-stable `fromIso` = min(Jan 1, 11-months-back) local midnights as `+00:00` (DB stores `to_rfc3339()`), **NET SALES** row pushed after `TOTAL` in `exportRows` (`total − previous_balance`, everything else `0`/`""`, `as never`); dropped unused `useQuery` | `admin.reports.tsx` |
| Dashboard: Sales today → `total_sales_today`/`total_invoices_today`, Last 7 days → `sales_7d` (sub "rolling week"); `sumBetween` + `today` deleted; `useInvoices()` kept only for Recent invoices | `admin.index.tsx` |
| Customers: Orders column → `invoice_count ?? 0`; page-level `useCustomerInvoices(expanded, expanded !== null)`; `<>` → `<Fragment key={c.id}>`; loading + "Showing the most recent 200 invoices." note; `invoice_count?: number` on the TS `Customer` types | `admin.customers.tsx`, `features/customers/api.ts`, `features/pos/api.ts` |

**Phase 3 ✅**
| Item | Files |
|------|-------|
| Checkout wraps `createCustomer.mutateAsync` — non-`Conflict:` rethrows; on `Conflict:` refetch `queryClient.fetchQuery({queryKey:["pos-customers"], …})`, match lowercased name + trimmed phone → reuse + `toast.info`; no match → `customer = null` + `toast.warning`, sale proceeds | `admin.pos.tsx` |
| `["pos-customers"]` invalidated on `useCreateCustomer`/`useUpdateCustomer`/`useDeleteCustomer` **and** on checkout success (3 spots, 2 beyond plan — intentional) | `features/customers/api.ts`, `admin.pos.tsx` |
| `customers:changed` realtime now also invalidates `["pos-customers"]` (the listener is `GlobalRealtimeListener` in `route.tsx`, **not** `tauri-events.ts`) | `route.tsx` |

**Phase 4 ✅**
| Item | Files |
|------|-------|
| `open_pool` builds `SqliteConnectOptions` (`.busy_timeout(5000)`, `.foreign_keys(true)`, `.synchronous(Normal)`; **no** `.journal_mode()` — the WAL switch takes an exclusive lock that `busy_timeout` can't wait on) and issues `PRAGMA journal_mode=WAL` once on a lone `SqliteConnection`, non-fatal (`log::warn!` + fall back to rollback journal), then `SqlitePoolOptions::connect_with(opts)` | `database/connection.rs:51-98` |
| `remove_stale_wal_files(db_path)` called at all 3 points of `import_database` — after `old.close()` before `fs::copy`, in the copy-failure branch before the `open_pool` revive, and after the copy before `match open_pool` | `commands/backup.rs:35,143,148,157` |
| 4 tests: WAL + per-connection pragmas (`journal_mode=wal`, `foreign_keys=1`, `synchronous=1`, `busy_timeout=5000`), WAL survives close/reopen, stale side-files deleted (db survives), no-op when nothing exists | `connection.rs`, `backup.rs` |
| Also removed the pre-existing unused `tauri::Manager` import | `database/connection.rs` |

### Gates (2026-10-06)
| Gate | Result |
|------|--------|
| `cargo check --all-targets` | ✅ 0 errors |
| `cargo test --lib` | ✅ **47 passed / 0 failed / 1 ignored** (baseline was 43+1) |
| `cargo clippy --all-targets` | ✅ 39 warnings vs 40 baseline — no new (one removed, one relocated by the added code) |
| `cargo build` | ✅ links |
| `tsc --noEmit` / `npm run lint` / `npm run build` | ⚠️ 14 (baseline, **accepted**) / ✅ **0 errors, 6 warnings** (exit 0) / ✓ |
| Gate 6 — live-DB-copy migration merge | ✅ **20/20**: keeper = earliest `created_at` / lowest `rowid`, balance = SUM, invoices repointed, index created + rejects, re-run no-op, real rows preserved |
| Gate 7a — WAL smoke on the live DB | ✅ launched the debug binary: `journal_mode` `delete` → `wal`, `-wal`/`-shm` present while running, persists after exit, `integrity_check = ok`, data intact (2 customers / 19 invoices), relaunch works |

### Repo-wide lint (2026-10-06)
`npm run lint` is now **green: 0 errors / 6 warnings (exit 0)**. Two config fixes were needed first:
- `eslint.config.js` global ignores only covered `dist`/`.output`/`.vinxi`, so `eslint .` traversed `src-tauri/target` + `dist-spa` and blew past a 900 s timeout. Added `dist-spa`, `src-tauri/target`, `src-tauri/gen`, `.opencode`, `**/*.d.ts`.
- `.prettierrc` had no `endOfLine`, so prettier's `lf` default flagged **1027 of 1129** findings as CRLF artifacts under `core.autocrlf=true`. Added `"endOfLine": "auto"`, then `eslint --fix` cleared the remaining 105 genuine prettier issues across 24 files (verified `tsc` still 14 and `vite build` still ✓).
- The 6 warnings are stock shadcn `react-refresh/only-export-components` in `ui/*.tsx`.

### Release build (2026-10-06)

| Step | Result |
|------|--------|
| Commit | `adf0a39 feat: pre-deploy fix plan (phases 1-4) + release hardening` — 79 files. `.gitignore` now excludes `*.pfx` and `.opencode/` so the signing key cannot be committed. **Note:** this commit holds the code + lint output only; the version bump below is still uncommitted at time of writing |
| Version bump | **1.1.0** in both `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` (`package.json` has no version field). Both must match or Tauri warns |
| Build | `cargo tauri build` — 5m45s. `beforeBuildCommand` ran `vite build --config vite.config.spa.ts` → `dist-spa`, then `cargo build --release --features tauri/custom-protocol`, then WiX + makensis |
| Artifacts | `src-tauri/target/release/bundle/msi/Moon Pipe POS_1.1.0_x64_en-US.msi` (9.9 MB) and `.../nsis/Moon Pipe POS_1.1.0_x64-setup.exe` (7.3 MB) |
| Sign | `signtool sign /f signing-cert.pfx /p MoonPipe2026 /fd SHA256 /tr http://timestamp.digicert.com /td SHA256` — 0 errors on both. Signer `CN=AZ Solutions`, timestamp `CN=DigiCert SHA256 RSA4096 Timestamp Responder 2026 1` |
| Trust store | First `signtool verify /pa` failed with *"chain terminated in a root certificate which is not trusted"* — the pfx lived only in `CurrentUser\My`. Imported the **public** cert (private key deliberately not stored) into `LocalMachine\Root` + `LocalMachine\TrustedPublisher` via an elevated run; thumbprint `2D1CFE03D65E62BF992595D5E372BCDE5BC6E74A`. Both installers now report `signtool verify /pa` = **Successfully verified** and `Get-AuthenticodeSignature` = **Valid** |
| Prior install | `D:\Projects\pos\Moon Pipe POS\` holds **1.0.0** (built 2026-09-22), registered in the uninstall hive as `Moon Pipe POS 1.0.0`. 1.1.0 upgrades over it. Shared DB: `C:\ProgramData\MoonPipe\moonpipe.db` |

**Debug-vs-release trap (cost us an hour):** `tauri/build.rs` sets `dev = !custom_protocol`, and `tauri-2.11.5` `manager/mod.rs:356` does `#[cfg(dev)] let url = self.config.build.dev_url`. So any binary built by plain `cargo build` (feature off → `cfg(dev)` on) points the webview at `http://localhost:8080` and never falls back to `frontendDist` → `ERR_CONNECTION_REFUSED` unless a vite server is running. `cargo build --release` behaves the *same way*. Only `cargo tauri build` / `npx tauri build` flips `custom-protocol` on. The **1.1.0 release exe needs no server at all.**

**Debug console trap:** `src-tauri/src/main.rs:2` is `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`, so **debug** builds are console apps — closing their CMD window sends `CTRL_CLOSE_EVENT` and kills the POS. Release builds have no console. Logs always go to `AppData\Local\com.moonpipe.pos\logs\Moon Pipe POS.log` regardless.

### Still to run by hand
- **Step 11 — install:** owner runs `Moon Pipe POS_1.1.0_x64-setup.exe` interactively (UAC now shows publisher **AZ Solutions**), then launches it.
- **Gate 7b — UI smoke against the installed build:** dashboard "Sales today" == Reports "Today" card · rolling 7-day card · POS walk-in duplicate reuse · customer Orders count vs history · Excel export row count + **NET SALES** row. The owner did exercise the debug app during this session (live-DB invoice count went **19 → 21**), but the 5-point checklist was never formally signed off.
- **Gate 7c — restore end-to-end:** not run. `remove_stale_wal_files` call sites are unit-tested only.
- Verify the installed exe reports file version 1.1.0 and `PRAGMA journal_mode = wal` on first launch.

### Corrections vs the plan
- Current SQLite numbers `synchronous` as `0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA` — the new test originally asserted `2`.
- `db_url.parse()` needed a turbofish (`parse::<SqliteConnectOptions>()`); the `let opts: SqliteConnectOptions` annotation alone did not resolve the type parameter.
- Migrations **017 + 018 were already applied to the live DB** (the old note at the bottom of this file was stale), so gate 6 had to drop `ux_customers_name_phone` to reach the pre-018 state.
- Phase 4 queries are runtime `sqlx::query` only → zero `.sqlx` offline-cache churn.

---

## 📁 Session 2026-10-05: Pre-Deployment Fix Plan — Phase 1 done, Phase 2 backend done

**Status**: ✅ **HANDED OFF** — completed by the 2026-10-06 session. Plan: `PRE_DEPLOY_FIX_PLAN.md` (repo root, untracked).
Phase 1 ✅ complete (gates clean, owner-run). Phase 2 backend ✅ complete.
Remaining items from this session were delivered on 2026-10-06 (see above).

### Done

**Phase 1 — duplicate-customer protection ✅**
| Item | Files |
|------|-------|
| Migration `018_customer_unique.sql` — merge dupes keyed `lower(trim(name))` + `trim(COALESCE(phone,''))`, keeper = earliest `created_at`, invoices repointed, keeper balance = SUM, dupes deleted, `ux_customers_name_phone` unique index | `src-tauri/src/database/migrations/018_customer_unique.sql` |
| `find_duplicate()` guard in `create` **and** `update` → `AppError::Conflict`, plus `is_unique_violation()` catch as race backstop | `repositories/customers.rs:71,116,147` |
| 4 tests: distinct-create matrix, duplicate-create Conflict (case/whitespace/None-vs-blank phone), update-to-duplicate Conflict, migration-018 merge (keeper, balance sum, invoice repoint, index enforcement) | `customers.rs:183+` |
| **Gates run clean (owner-reported)** — Phase-1 test suite passes | — |

**Phase 2 backend ✅ (2.1 / 2.4 / 2.5 / 2.7-backend)**
| Item | Files |
|------|-------|
| `ListInvoicesInput` `from_date`/`to_date`/`customer_id` + repo 4-branch list (customer / range / search / default), `RANGE_CAP = 50_000`; default path byte-identical (limit 50, `SEARCH_CAP` 200). *Deviation:* `customer_id` branch capped at `SEARCH_CAP` — harmless, keep | `commands/invoices.rs:15-17,61-63`, `repositories/invoices.rs:79-173` |
| `get_dashboard` sargable **local-time** ranges (`chrono::Local` midnight → RFC3339, `created_at >= ? AND < ?`) for sales, counts **and** all 3 profit windows — fixes UTC 00:00–05:00 PKT skew; new `sales_7d`, `invoices_7d` fields | `commands/reports.rs:56-104,161-171` |
| TS `DashboardStats` + `sales_7d`/`invoices_7d`; `ListInvoicesParams` with all 3 filters; `Customer.invoice_count?` | `api-client.ts:152-157,263,355-356` |
| `invoice_count` LEFT JOIN aggregate in customer `list`/`get` (`#[serde(default)]`) | `repositories/customers.rs:18,43,59` |

### Left (in plan order) — all delivered 2026-10-06

> Kept for traceability only; see the 2026-10-06 session above for what actually shipped.

| Phase | Item |
|-------|------|
| **2.2** | ✅ `useInvoicesInRange(fromIso)` + `useCustomerInvoices(customerId, enabled)` → `features/invoices/api.ts` |
| **2.3** | ✅ `admin.reports.tsx` — `useInvoicesInRange(from)` + **NET SALES** row in `exportRows` below untouched `TOTAL` |
| **2.6** | ✅ `admin.index.tsx` — `sumBetween` deleted; Sales today / Last 7 days from `dashboard`; `useInvoices()` only for Recent invoices |
| **2.7** | ✅ `admin.customers.tsx` — Orders → `c.invoice_count`, page `useInvoices()` dropped, lazy history + "most recent 200" note, Fragment keyed; `invoice_count?: number` on both TS `Customer` types |
| **3** | ✅ POS `createCustomer` try/catch → re-lookup & reuse / anonymous walk-in + warning; `["pos-customers"]` invalidated on create + realtime event |
| **4** | ✅ `connection.rs` — one-time `PRAGMA journal_mode=WAL` + `busy_timeout=5000`, `synchronous=NORMAL`, `foreign_keys=ON`; `backup.rs::import_database` — `moonpipe.db-wal`/`-shm` removed at all 3 points |
| **Gates** | ✅ 1–6 clean, 7a clean, repo-wide lint clean; **7b (UI smoke) and 7c (restore end-to-end) still to run by hand against the 1.1.0 install** |
| **Note** | Migrations 017 + 018 **are** applied to the live DB (confirmed `versions=[1..18]`, `ux_customers_name_phone` present) — this note was stale |

---

## 🔥 Session 2026-10-03: Carry-Forward System — Hardening, Supplier Port, Reconcile Cleanup

**Status**: ✅ **COMPLETE.** Three workstreams delivered: (A) invoice carry-forward hardening — ledger-truth reconciliation, return cascade, net-revenue reports; (B) supplier purchase carry-forward — full port of the invoice chain (migration 017); (C) Customers-page Reconcile button removed (UI-only).

### A. Invoice carry-forward hardening — balance truth

| Area | Files | What changed |
|------|-------|--------------|
| **Reconciliation service** | `services/reconciliation.rs` | Recomputes `customers.outstanding_balance` as `SUM(total - amount_paid)` over **non-carried** open invoices (`carried_to_invoice_id IS NULL`) — a SUM, not "newest", because legacy data can hold several open invoices per customer. Runs at **startup** (`lib.rs` → `run_and_notify`), after **backup restore** (`commands/backup.rs`), and via manual command. Logs `broken_invariant_rows` when a recompute still disagrees |
| **Manual command** | `commands/reconciliation.rs` | `reconcile_balances` (cashier/admin) — returns row count, emits `customers:changed` |
| **Absorbed-source marking** | `repositories/invoices.rs` | `create()` marks **every** open invoice it absorbed (not just the newest) so legacy multi-open customers don't strand debt |
| **Return cascade** | `repositories/invoices.rs`, `repositories/returns.rs` | A return that shrinks invoice A pushes the reduction forward through `carried_to_invoice_id` (depth-capped walk): successors' `previous_balance`/`total`/`amount_paid` re-derived each hop; customer balance recomputed from **leaf** invoices — never nudged by `+=` (a carried invoice is not a leaf; deltas would double-count or vanish at next reconcile) |
| **Dead API removed** | `repositories/customers.rs` | `update_balance(id, delta)` deleted — mirror is derived, hand-nudging is banned |
| **Net-revenue reports** | `commands/reports.rs`, `admin.index.tsx`, `admin.reports.tsx` | Every sales/revenue aggregate now sums `total - previous_balance` (money already counted when it first changed hands); Excel export gains a **Previous Balance** column |

### B. Supplier purchase chain — full port (mirrors the invoice chain)

| Phase | Files | Summary |
|-------|-------|---------|
| **Migration** | `017_supplier_purchase_carry_forward.sql` | `previous_balance INTEGER NOT NULL DEFAULT 0`, `carried_to_purchase_id TEXT REFERENCES supplier_purchases(id)`, index `idx_supplier_purchases_carried_to`. Migrations 001–016 untouched |
| **Create** | `repositories/suppliers.rs` `create()` | Reads supplier balance → `previous_balance`; `total = subtotal - discount + previous_balance`; `amount_paid` clamped to grand total; supplier balance **SET** to `MAX(0, due)` (replaced `+=`); marks **every** absorbed open purchase when `previous_balance > 0`; new purchase inserted with `carried_to_purchase_id = NULL` (chain head) |
| **Guard** | `suppliers.rs` `mark_paid()` | Absorbed purchases (`carried_to_purchase_id` set) cannot be paid — validation error names the successor `purchase_no` |
| **Reconciliation** | `services/reconciliation.rs`, `commands/reconciliation.rs` | `SUPPLIER_RECONCILE_TEMPLATE` mirror (SUM over `carried_to_purchase_id IS NULL`); wired into the same startup / restore / manual-command / invariant-warning paths. `run_and_notify` runs both reconciles and emits both events |
| **Frontend** | `api-client.ts`, `admin.suppliers.tsx` | `SupplierPurchase.previous_balance` + `carried_to_purchase_id`; dialog shows carried row + grand total; cash/bank methods **auto-fill Amount Paid to grand total** (user-editable flag, resets on supplier/method change); Purchase History gains **Previous Balance** column + **Carried** badge; Record Payment hidden on absorbed POs; directory rows show `· carried` |
| **Tests** | `suppliers.rs`, `reconciliation.rs`, `test_support.rs` | 11 new tests (7 repo + 4 supplier-reconcile) + fixtures → **cargo test 39/39**. Suppliers repo stays runtime-query-only → **zero `.sqlx` churn** |
| **Plan** | `.opencode/plans/SUPPLIER_CARRY_FORWARD_PLAN.md` | Approved 7-step plan, done criteria, STOP conditions, maintenance notes |

**Naming rule**: supplier column is `carried_to_purchase_id` (never `carried_to_invoice_id`) — deliberate divergence from invoices; keep it if more ledgers get chains.

### C. Customers-page Reconcile button removed (UI-only)

Deleted the button, `useReconcileBalances` hook, and `api.customers.reconcile` (3 files). The Rust `reconcile_balances` command is **kept** — reconciliation already runs at startup and after every restore, so the button was redundant repair UI. Service, command, and auto-run paths untouched.

### Verification (all green)

| Gate | Result |
|------|--------|
| `cargo test` | **39 passed, 0 failed** |
| `cargo check` + `SQLX_OFFLINE=true cargo check` | exit 0 both |
| `bunx vite build` | ✓ (revert `routeTree.gen.ts` noise after every build) |
| `bunx tsc --noEmit` | exactly 14 baseline errors, 0 new |
| eslint (touched files vs `git show HEAD:` baselines) | no new errors; customers page 38 → **33** |
| Live DB | migration **017 not yet applied** — applies automatically on next launch; 001–016 untouched; hash delta was concurrent app usage (invoice INV-2026-0011), schema verified intact |

### Known open (flagged, NOT fixed)

- `commands/reports.rs::get_sales_report` sums `i.total - i.previous_balance` over a `LEFT JOIN invoice_items` with `GROUP BY date` → multi-item days count each invoice once per item. Profit math (per-item CASE) is unaffected.

---

## 🔥 Session 2026-09-22: System Lock Feature + Code Signing + Build

**Status**: ✅ **COMPLETE.** Three features delivered: (1) system lock via secret key combo, (2) self-signed cert as "AZ Solutions", (3) fresh signed installers built and tested.

### What was built

| Phase | Files | Summary |
|-------|-------|---------|
| **DB migration** | `016_add_app_settings.sql` | `app_settings` table (key TEXT PK, value TEXT, updated_at TEXT) with unique index |
| **HMAC + bcrypt** | `src-tauri/src/license/mod.rs` | `LicenseState` (in-memory RwLock), `verify_password()` (bcrypt), `sign_hmac()` / `verify_hmac()` (SHA-256), `is_system_locked()` / `set_system_locked()`, `require_unlocked()` middleware |
| **License commands** | `src-tauri/src/commands/license.rs` | `check_system_lock` (no auth), `set_system_lock` (no auth — combo IS auth), `unlock_with_password` |
| **Module registration** | `commands/mod.rs`, `lib.rs` | `pub mod license`, 3 commands registered in `generate_handler![]`, `LicenseState` managed, lock checked on startup |
| **Zustand store** | `src/lib/license-store.ts` | `{ status, isLoading, setStatus, setLoading }` |
| **License API** | `src/features/license/api.ts` | `checkSystemLock()`, `unlockWithPassword(password)` |
| **Key combo listener** | `src/components/license/KeyComboListener.tsx` | Detects `lockdownsystem` typed within 4 seconds (no modifiers); blocks on input/textarea/contenteditable; triggers `set_system_lock` via invoke |
| **Lock screen** | `src/components/license/LockScreen.tsx` | Full-screen overlay with password input, lock icon, AZ Solutions credit |
| **License gate** | `src/components/license/LicenseGate.tsx` | Top-level wrapper — loading → locked → unlocked states; fail-closed on error |
| **App entry** | `src/main.tsx` | `LicenseGate` wraps `RouterProvider` |
| **Backup hardening** | `commands/backup.rs` | Ensures `app_settings` + lock row exist after restore; `app_settings` added to `REQUIRED_TABLES` |
| **Code signing cert** | `signing-cert.pfx` | Self-signed `CN=AZ Solutions`, SHA-256, 5-year validity, password `MoonPipe2026` |
| **Installer build** | `cargo tauri build` | Both MSI + NSIS built and signed with AZ Solutions cert + DigiCert timestamp |

### Key combo evolution

| Version | Combo | Problem | Resolution |
|---------|-------|---------|------------|
| v1 | Ctrl+Shift+A-R-E-E-B | Ctrl+R refreshes the page, making the combo useless | Dropped Ctrl |
| v2 | Shift+A-R-E-E-B | Shift held during 'a' gives uppercase 'A'; complex modifier tracking | Dropped modifiers entirely |
| v3 | Just type `lockdownsystem` | — | Final. 14-char string typed within 4s. No false positives ("lockdownsystem" is not a word). |

### Bug fixed this session

| Bug | Root cause | Fix |
|-----|-----------|-----|
| `set_system_lock` silently fails from key combo | Command required `require_admin()` with JWT auth header, but `KeyComboListener` sends no auth → Rust returns auth error → swallowed by `.catch(() => {})` | Removed `require_admin` from `set_system_lock`. The secret combo IS the authentication — no JWT needed to lock. Only password needed to unlock. |

### Build artifacts (2026-09-22, v1.0.0)

| File | Signed | Publisher |
|------|--------|-----------|
| `src-tauri/target/release/bundle/msi/Moon Pipe POS_1.0.0_x64_en-US.msi` | ✓ SHA-256 + DigiCert timestamp | AZ Solutions |
| `src-tauri/target/release/bundle/nsis/Moon Pipe POS_1.0.0_x64-setup.exe` | ✓ SHA-256 + DigiCert timestamp | AZ Solutions |

Certificate: self-signed `CN=AZ Solutions`, valid to 2031, password `MoonPipe2026`, exported as `signing-cert.pfx` in project root.

### Installed app behavior

- Install location: `D:\Projects\pos\Moon Pipe POS\` (per NSIS default, user-selected)
- Database: `C:\ProgramData\MoonPipe\moonpipe.db` (shared with dev instance)
- First launch after install may show blank window momentarily — kill and relaunch resolves (WebView2 cold start)
- Window title: "Moon Pipe POS"
- `app_settings.system_lock` persists across launches; if locked during dev, installed app also reads locked state (same DB)

---

## 🔥 Session 2026-09-19: Invoice Chain — Unpaid Balances Carry Forward + Client B Rebrand

**Status**: ✅ **COMPLETE.** Two features implemented: (1) client-b fork "Moon Pipe and Sanitary Store" branding, (2) invoice chain where unpaid balances carry forward to the next invoice.

### What was built

| Phase | Files | Summary |
|-------|-------|---------|
| **Client B branding** | `HANDOFF.md`, `PROJECT_ARCHITECTURE.md`, `src/styles.css`, `admin.pos.tsx`, `admin.inventory.tsx`, `api.ts` files | Renamed to "Moon Pipe and Sanitary Store", categories changed to `sanitary \| hardware`, copper accent color scheme applied |
| **Phase 1: DB** | `009_invoice_previous_balance.sql` | `ALTER TABLE invoices ADD COLUMN previous_balance INTEGER NOT NULL DEFAULT 0;` |
| **Phase 2: Rust structs** | `repositories/invoices.rs` | Added `previous_balance: i64` to `Invoice` struct; updated all 3 SELECT queries (search, default list, get) |
| **Phase 3: Invoice creation** | `repositories/invoices.rs` `create()` | Reads `customers.outstanding_balance` → `previous_balance`; computes `total = subtotal - discount + previous_balance`; `amount_paid` clamped to new total; `outstanding_balance` SET to `MAX(0, total - amount_paid)` (not `+=`) |
| **Phase 4: Commands** | `commands/invoices.rs` | Removed client-side `total < 0` validation (backend recalculates total) |
| **Phase 5: TS types** | `features/invoices/api.ts`, `lib/api-client.ts` | Added `previous_balance: number` to `Invoice` type |
| **Phase 6: POS page** | `admin.pos.tsx` | Derives `previousBalance` from selected customer's `outstanding_balance`; `total = max(0, subtotal - discount + previousBalance)`; "Previous balance" row in summary; removed `Math.min(..., total)` clamp on `paidNow` |
| **Phase 7: Invoice detail** | `admin.invoices.$invoiceId.tsx` | Shows "Previous balance" row when > 0 |
| **Phase 8: Receipt bitmap** | `services/receipt_bitmap.rs` | Added "Previous balance" line before TOTAL; test `sample_invoice()` updated with `previous_balance: 0` |
| **Phase 9: ESC/POS receipt** | `services/print.rs` | Both `build_escpos_receipt` and `build_text_receipt` show `previous_balance` when > 0 |
| **Phase 10: A4 PDF** | `services/invoice_pdf.rs` | Shows "Previous balance" line when > 0 |

### Invoice chain design decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| `total` field handling | **Backend ignores frontend `total` and recalculates** (Option A) | Backward compat — old clients still send `total` but it's overwritten with `subtotal - discount + previous_balance` |
| `previous_balance` source | Read from `customers.outstanding_balance` at creation time within transaction | Ensures consistency even if another invoice is created concurrently |
| `outstanding_balance` update | **SET** to `MAX(0, total - amount_paid)` (not `+=`) | Avoids double-counting — previous invoice already set the balance; new invoice replaces it |
| `mark_paid()` logic | Unchanged: `outstanding_balance -= applied` | Works because payments always target the latest invoice |
| Frontend total calculation | `total = max(0, subtotal - discount + previousBalance)` | Client shows preview; backend overwrites with authoritative calculation |

### Build status

- Vite build: ✅ PASSED clean
- `cargo check`: ✅ Only pre-existing `sqlx` "unable to open database file" errors (zero new type/schema errors)
- No commits made yet — changes uncommitted

### Client B setup notes

- Database path: `C:\ProgramData\MoonPipe\moonpipe.db` (separate from City Tiles)
- Branch: `client-b-sanitary`
- Categories: `sanitary | hardware` (not `marble | tiles | chips`)
- Color scheme: cool blue-gray base + warm copper accent (`--color-primary: oklch(0.45 0.12 40)`)
- Tile-specific logic removed from POS (no perCarton, perTileArea, carton/tile UI)

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
| Client B | **Branch `client-b-sanitary`** — "Moon Pipe and Sanitary Store"; DB at `%PROGRAMDATA%\MoonPipe\moonpipe.db`; categories `sanitary \| hardware`; teal accent + rounded corners; tile logic removed from POS; installed at `D:\Projects\pos\Moon Pipe POS\` (2026-09-22) |
| Payment rule | Invoice number tracks dues universally; customer balance mirrors any attached-customer due |
| Code signing | Self-signed, publisher **"AZ Solutions"** — cert at `signing-cert.pfx` in project root, password `MoonPipe2026`, valid to 2031 (2026-09-22) |
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
| Invoice chain | **Unpaid balances carry forward** — `previous_balance` on invoices read from `customers.outstanding_balance` at creation; `total = subtotal - discount + previous_balance`; `outstanding_balance` SET to `MAX(0, total - amount_paid)` after creation; `mark_paid()` unchanged (2026-09-19). **Hardened 2026-10-03**: every absorbed source invoice marked; returns cascade the reduction through the whole carry chain (depth-capped) and balances recompute from leaf invoices; reports count net revenue (`total - previous_balance`) |
| Supplier chain | **Supplier purchases mirror the invoice chain** — migration 017 added `previous_balance` + `carried_to_purchase_id` on `supplier_purchases`; `create()` carries the supplier balance into the new bill (grand total), SET (never `+=`) `suppliers.outstanding_balance`, marks absorbed open purchases; `mark_paid()` rejects absorbed POs naming the successor. Column is `carried_to_purchase_id`, **not** `carried_to_invoice_id` (2026-10-03) |
| Balance truth | **Mirrors are derived, never hand-nudged** — `customers.outstanding_balance` / `suppliers.outstanding_balance` = SUM of due over non-carried open rows; recompute runs at startup + after backup restore (`services/reconciliation.rs`, warns on broken invariants); `customers::update_balance` delta API removed; manual `reconcile_balances` command has no UI button (removed 2026-10-03) |
| System lock | **Secret combo `lockdownsystem`** — type within 4 seconds (no modifiers); locks system via `set_system_lock` (no JWT auth — combo IS auth); unlock via password `Areeb@1234` (bcrypt-hashed); lock state in `app_settings` table with HMAC signature; `require_unlocked()` middleware on all sensitive commands; fail-closed on missing/corrupt DB (2026-09-22) |
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

**Status**: ~99% complete — carry-forward system complete (invoice chain hardened + supplier chain ported + reconcile cleanup), system lock implemented, signed installers built and tested on dev machine. Remaining: clean-machine install test → full manual matrix → staff docs.
