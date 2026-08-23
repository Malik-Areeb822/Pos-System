# Stone Flow POS — Offline Migration Plan (Tauri v2 + SQLite)

> **Goal**: Convert to single executable, auto-start, fully offline, loosely coupled
> **Constraint**: Zero functional changes to core POS behavior
> **Generated**: 2026-08-19

---

## 1. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        TAURI APP (single .exe)                  │
│  ┌──────────────────────┐  ┌────────────────────────────────┐  │
│  │   FRONTEND (SPA)     │  │      RUST BACKEND              │  │
│  │  ─────────────────   │  │  ────────────────────────      │  │
│  │  • React 19 + Vite   │  │  • SQLite (tauri-plugin-sql)   │  │
│  │  • TanStack Router   │◄─┼──►│  • Tauri Commands (IPC)      │  │
│  │  • TanStack Query    │  │  • Tauri Events (realtime)     │  │
│  │  • React Hook Form   │  │  • JWT Auth (jsonwebtoken)     │  │
│  │  • Radix UI + Tailwind     │  • bcrypt (password hash)    │  │
│  │  • Error Boundaries        │  • Module isolation          │  │
│  └──────────────────────┘  └────────────────────────────────┘  │
│           │                           │                          │
│           └─────────────┬─────────────┘                          │
│                         ▼                                        │
│              ┌─────────────────────┐                             │
│              │   LOCAL RESOURCES   │                             │
│              │  • SQLite DB        │                             │
│              │  • App Data (images)│                             │
│              │  • Config (JSON)    │                             │
│              └─────────────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Loose Coupling Strategy (Critical)

Every module communicates via **well-defined interfaces** with **error boundaries**:

### Frontend: Feature-Based Modules with Circuit Breakers
```typescript
// src/features/pos/api.ts — Abstract API client
interface ApiClient {
  products: ProductsApi;
  customers: CustomersApi;
  invoices: InvoicesApi;
  // ...
}

// Each feature gets its own API slice with:
// - Retry logic (3x exponential backoff)
// - Circuit breaker (open after 5 failures/30s)
// - Fallback to cached React Query data
// - Graceful degradation UI (toast "Working offline...")
```

### Backend: Command Isolation (Rust)
```rust
// Each Tauri command is independent — panic in one doesn't crash others
#[tauri::command]
async fn create_invoice(cmd: CreateInvoiceCmd) -> Result<InvoiceId, AppError> {
    // Isolated transaction, own error handling
    INVOICE_SERVICE.create(cmd).await
}

// Global panic hook logs but doesn't exit
std::panic::set_hook(Box::new(|info| {
    log::error!("Panic in command: {:?}", info);
    // App stays alive, frontend gets error response
}));
```

### Database: Repository Pattern + Connection Pool
```rust
// Each service gets its own repository — no cross-service DB coupling
struct InvoiceRepository { pool: SqlitePool }
struct ProductRepository { pool: SqlitePool }
// Services call repositories, never raw SQL
```

---

## 3. Frontend Migration (TanStack Start → SPA)

| Current | Target | Notes |
|---------|--------|-------|
| `src/server.ts` (SSR) | **Remove** | No SSR needed |
| `vite.config.ts` (Nitro) | Vite SPA config | Keep `@tanstack/router-plugin` |
| `src/router.tsx` | Keep | Works in SPA mode |
| `src/routes/__root.tsx` | Keep | Remove `QueryClientProvider` from shell, add to `main.tsx` |
| Supabase client | **Custom `ApiClient`** | Same interface, calls Tauri commands |
| `useInvoiceRealtime` hook | **Tauri Event listener** | Same React Query invalidation API |

**New Files**:
```
src/
├── main.tsx                    // SPA entry (was server.ts)
├── lib/
│   ├── api-client.ts           // Tauri invoke wrapper + circuit breaker
│   ├── tauri-events.ts         // Real-time event bridge
│   └── auth-store.ts           // Local JWT + user state (Zustand)
├── features/
│   ├── pos/api.ts              // POS-specific API calls
│   ├── inventory/api.ts
│   ├── customers/api.ts
│   ├── invoices/api.ts
│   ├── returns/api.ts
│   ├── reports/api.ts
│   └── cashiers/api.ts
```

---

## 4. Backend Architecture (Rust)

### Project Structure
```
src-tauri/
├── Cargo.toml
├── tauri.conf.json
├── src/
│   ├── main.rs                    // App entry, plugin registration
│   ├── lib.rs                     // Module declarations
│   ├── config.rs                  // App config (paths, JWT secret)
│   ├── error.rs                   // AppError + Tauri serialization
│   ├── database/
│   │   ├── mod.rs
│   │   ├── connection.rs          // SqlitePool + migrations
│   │   ├── migrations/            // .sql files (embedded)
│   │   └── seed.rs                // Initial data (20 products)
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── jwt.rs                 // Token create/validate
│   │   ├── password.rs            // bcrypt hash/verify
│   │   └── middleware.rs          // Command guard macro
│   ├── repositories/
│   │   ├── mod.rs
│   │   ├── products.rs
│   │   ├── customers.rs
│   │   ├── invoices.rs
│   │   ├── returns.rs
│   │   ├── cashiers.rs
│   │   └── users.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── pos.rs                 // Checkout logic (atomic)
│   │   ├── inventory.rs           // Import/export + stock
│   │   ├── returns.rs             // process_return logic
│   │   ├── reports.rs             // Aggregations
│   │   ├── print.rs               // ESC/POS receipt formatting + raw USB write
│   │   ├── invoice_pdf.rs         // PDF invoice generation (genpdf) → app data
│   │   └── backup.rs              // Export/import SQLite file (or .sql dump)
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── auth.rs                // login, register, me, logout
│   │   ├── products.rs
│   │   ├── customers.rs
│   │   ├── invoices.rs
│   │   ├── returns.rs
│   │   ├── reports.rs
│   │   ├── cashiers.rs
│   │   ├── print.rs               // print_receipt(invoice_id) command
│   │   └── backup.rs              // export_data(path), import_data(path) commands
│   ├── events/
│   │   ├── mod.rs
│   │   └── emitter.rs             // Tauri Event emission helpers
│   └── autostart.rs               // OS login item registration
```

### Key Dependencies (`Cargo.toml`)
```toml
[dependencies]
tauri = { version = "2", features = ["sql", "fs", "shell", "autostart", "single-instance"] }
tauri-plugin-sql = { version = "2", features = ["libsql"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "uuid", "chrono", "json"] }
tokio = { version = "1", features = ["full"] }
jsonwebtoken = "9"
bcrypt = "0.16"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
rusb = "0.9"              // Raw USB write for ESC/POS thermal receipt printer
escpos-rs = "0.13"        // ESC/POS command formatting (or hand-rolled if it doesn't fit)
genpdf = "0.2"            // PDF invoice generation (lighter than printpdf)
```

> **Note**: Barcode scanner support (`serialport` plugin) is **not needed** — scanner is
> keyboard-emulation (types + Enter), handled entirely in the existing frontend input logic.
> No Rust-side integration required.

---

## 5. Database Migration (Supabase → SQLite)

> **Confirmed: fresh installation, no existing production data.** There is no Supabase data
> to migrate — first app launch runs migrations 001-005 against an empty SQLite file, and
> `005_admin_user.sql`'s "first user = admin" flow becomes the real onboarding path (mirrors
> the existing `handle_new_user()` Postgres trigger, re-implemented in Rust). This removes any
> "run both systems in parallel" concern from the risk table. Open question: whether
> `004_seed_data.sql` ships the 20 example products or starts inventory empty — see section 13.

### Schema Mapping
| Supabase | SQLite | Notes |
|----------|--------|-------|
| `uuid` PK | `TEXT` PK (UUID v4) | `gen_random_uuid()` → `uuid::Uuid::new_v4()` |
| `timestamptz` | `TEXT` (ISO8601) | Store UTC, parse with `chrono` |
| `numeric(12,2)` | `REAL` | Store as integer paise (PKR * 100) to avoid float issues |
| Enums | `TEXT` + CHECK constraints | `CHECK (category IN (...))` |
| `gen_random_uuid()` | Application-side | Rust generates UUIDs |
| Sequences (`invoice_seq`) | SQLite `AUTOINCREMENT` table | Separate counter table |
| RLS Policies | **Application-level** | Enforced in Rust services |
| Triggers (`handle_new_user`) | Rust logic in `auth::register()` | First user = admin |

### Migration Files (embedded in binary)
```
src-tauri/src/database/migrations/
├── 001_initial_schema.sql      // All tables, indexes, constraints
├── 002_enum_constraints.sql    // CHECK constraints for enums
├── 003_invoice_counter.sql     // invoice_counter table
├── 004_seed_data.sql           // 20 products from Supabase migration
└── 005_admin_user.sql          // Created on first run
```

---

## 6. Auth System Replacement

| Supabase Auth | Local Implementation |
|---------------|---------------------|
| Email/password | Same UI, calls `auth::login` Tauri command |
| OAuth (Google) | **Remove** — offline only |
| Session (JWT in localStorage) | **JWT in secure httpOnly cookie equivalent** — Tauri `app-data` file |
| `supabase.auth.getUser()` | `auth::me` command (validates JWT, returns user + roles) |
| `user_roles` + `profiles.status` | Same tables, checked in Rust middleware |
| Admin setup flow | Same: first registration → admin, rest → pending |

**Token Flow**:
```
Login → Rust: verify password → issue JWT (24h expiry) → save to local file
      → Frontend: store in Zustand + React Query cache
Each command → Frontend: send JWT in invoke headers
            → Rust: validate JWT → check role/status → execute or 401
```

---

## 7. Real-Time Replacement (Supabase Realtime → Tauri Events)

**Current**: `useInvoiceRealtime` subscribes to Postgres changes → invalidates React Query

**New**: Rust emits Tauri Events on mutations → Frontend listens → same invalidation

```rust
// Rust: after any invoice/customer mutation
tauri::emit("invoices:changed", ()).unwrap();
tauri::emit("customers:changed", ()).unwrap();
```

```typescript
// Frontend: useInvoiceRealtime.ts replacement
import { listen } from '@tauri-apps/api/event';

export function useInvoiceRealtime() {
  const queryClient = useQueryClient();
  
  useEffect(() => {
    const unlistenInvoices = await listen('invoices:changed', () => {
      queryClient.invalidateQueries({ queryKey: ['invoices'] });
      queryClient.invalidateQueries({ queryKey: ['customers'] });
      queryClient.invalidateQueries({ queryKey: ['dashboard'] });
    });
    const unlistenCustomers = await listen('customers:changed', () => {
      queryClient.invalidateQueries({ queryKey: ['customers'] });
    });
    return () => { unlistenInvoices(); unlistenCustomers(); };
  }, [queryClient]);
}
```

**Result**: Identical behavior, zero network, works across all app windows/tabs.

---

## 7.5 Printing (Receipt + Invoice)

**Decision**: Two separate outputs, two separate mechanisms — do not try to unify them.

### 7.5.1 Customer Receipt — 80mm USB Thermal Printer (ESC/POS direct)

- **Connection**: USB. Exact brand/model TBD (buyer hasn't purchased yet), but nearly all
  cheap 80mm thermal printers speak the same **standard ESC/POS command subset** — brand
  mostly affects driver quirks, not the protocol itself. Build against generic ESC/POS now;
  expect at most a small tweak once the real unit is in hand.
- **Approach**: Raw ESC/POS bytes over USB via `rusb`, bypassing the OS print spooler/dialog
  entirely. This gives instant, dialog-free printing on checkout — critical for real POS use
  where a print-dialog popup on every sale is a dealbreaker.
  - Fallback path if raw USB enumeration proves difficult on a given machine: install the
    printer as a "Generic / Text Only" Windows printer and write raw ESC/POS bytes through the
    Windows spooler API instead. Same command payload either way.
- **When it fires**: Automatically as part of the checkout mutation, fire-and-forget with retry
  (consistent with the circuit-breaker pattern in section 2) — a failed/offline printer must
  never block the sale from completing.
- **Content**: Header (business name/address from `lib/business.ts` equivalent), line items,
  totals, payment method, footer. Reuses the same data shape as `invoice_items`.
- **New module**: `services/print.rs` (formatting + raw write) + `commands/print.rs`
  (`print_receipt(invoice_id)` command, called right after invoice insert succeeds).

### 7.5.2 System Invoice — PDF (saved locally, not printed)

- **Approach**: Generate a proper PDF (A4/Letter) via `genpdf`, saved to app data as
  `invoices/{invoice_no}.pdf`. This is the permanent, always-succeeds record — independent of
  printer state, drivers, or connection.
- **New module**: `services/invoice_pdf.rs` (`generate_invoice_pdf(invoice)`), called in the
  same checkout flow as the receipt print, but never allowed to fail the sale either.

### Updated Checkout Flow (replaces the single-line "Insert invoice" step)

```
checkout.mutate()
    │
    ├─► Find/create customer (walk-in + save flag)
    ├─► INSERT invoice
    ├─► INSERT invoice_items[]
    ├─► UPDATE products.stock_qty -= qty
    ├─► IF credit: UPDATE customers.outstanding_balance
    ├─► tauri::emit("invoices:changed") / ("customers:changed")
    │
    ├─► print_receipt_escpos(invoice)     // fire-and-forget, retry via circuit breaker
    └─► generate_invoice_pdf(invoice)     // always succeeds, saved to app data
```

### Barcode Scanner — Not Needed

Confirmed out of scope. Scanner input is keyboard-emulation (types SKU + Enter), already
handled by existing frontend input logic in the POS screen — no Rust/serial integration
required. `serialport` plugin dropped from dependencies.

---

## 7.6 Backup & Restore (Local File, No Cloud Dependency)

Treated as **Phase 3-4 core work, not optional polish** — a single SQLite file on one machine
with no cloud backup is a real business-continuity risk for a POS handling live invoices and
customer balances.

- **Export**: One-click copy of the SQLite database file (or a `.sql` dump) to a user-chosen
  location — USB drive, or a cloud-synced folder (Google Drive/OneDrive) the user has set up
  themselves. Implemented as `services/backup.rs::export_data(dest_path)`; trivial — file copy
  or `sqlite3 .dump` equivalent via `sqlx`.
- **Import**: Restore from a previously exported file. **Admin-only**, gated behind an explicit
  confirmation step in the UI since it overwrites all current data. Implemented as
  `services/backup.rs::import_data(src_path)`.
- **UI**: New section in Admin settings — "Export Data" / "Import Data" buttons, matching the
  original section 13 Q4 proposal.
- **Not in scope for now**: scheduled/automatic background backups. Can be added later as a
  simple "backup on app close to a fixed local folder" if desired, but manual export/import
  covers the core risk.

---

## 8. Module Isolation Details

### Frontend Error Boundaries
```tsx
// Each major route wrapped
<ErrorBoundary fallback={<PosFallback />}>
  <PosScreen />
</ErrorBoundary>

<ErrorBoundary fallback={<InventoryFallback />}>
  <InventoryScreen />
</ErrorBoundary>
```

### API Circuit Breaker (per feature)
```typescript
class CircuitBreaker {
  private failures = 0;
  private lastFailure = 0;
  private state: 'closed' | 'open' | 'half-open' = 'closed';
  
  async call<T>(fn: () => Promise<T>): Promise<T> {
    if (this.state === 'open') {
      if (Date.now() - this.lastFailure > 30000) this.state = 'half-open';
      else throw new Error('Circuit open');
    }
    try {
      const result = await fn();
      this.onSuccess();
      return result;
    } catch (e) {
      this.onFailure();
      throw e;
    }
  }
}
```

### Backend: Command-Level Isolation
- Each `#[tauri::command]` runs in own Tokio task
- Panic hook catches, logs, returns `AppError::Internal`
- No shared mutable state between commands
- Database pool cloned per request (cheap with `sqlx`)

---

## 9. Auto-Start Implementation

```rust
// src/autostart.rs
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

pub fn setup_autostart(app: &tauri::App) {
    let autostart = app.autolaunch();
    autostart.enable().ok(); // Idempotent
    
    // Also register single-instance (prevent double launch)
    app.handle().plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        let _ = app.get_webview_window("main").map(|w| {
            w.show().ok();
            w.set_focus().ok();
        });
    })).ok();
}
```

**Windows**: Creates `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` entry
**macOS**: LaunchAgent plist
**Linux**: `.config/autostart/*.desktop`

---

## 10. Build & Packaging

```json
// tauri.conf.json (key parts)
{
  "productName": "City Tiles POS",
  "version": "1.0.0",
  "identifier": "com.citytiles.pos",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [{
      "title": "City Tiles POS",
      "width": 1400,
      "height": 900,
      "minWidth": 1200,
      "minHeight": 800
    }],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data:;"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "icon": ["icons/icon.ico", "icons/icon.png"],
    "windows": {
      "nsis": { "installerIcon": "icons/icon.ico" }
    }
  },
  "plugins": {
    "sql": { "preload": ["sqlite:citytiles.db"] },
    "autostart": { "macos": { "useLaunchAgent": true } },
    "singleInstance": {}
  }
}
```

---

## 11. Migration Sequence (Zero-Downtime for User)

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| **1. Frontend SPA + API Abstraction** | 3-4 days | Working SPA calling mock API |
| **2. Rust Backend Core (DB, Auth, Commands)** | 5-7 days | Tauri app with all CRUD working |
| **3. Business Logic Port (POS, Returns, Reports)** | 4-5 days | Feature parity with Supabase |
| **4. Real-time + Auto-start + Polish** | 2-3 days | Production executable |
| **5. Testing + Installer** | 2 days | Signed MSI/EXE |

**Total: ~3-4 weeks** (can parallelize frontend/backend after Phase 1)

---

## 12. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| SQLite corruption | WAL mode + periodic `PRAGMA integrity_check`; mitigated further by export/import backup (section 7.6) |
| Large DB (>1GB) | Pagination in all list queries; archive old invoices |
| Forgot password | Admin can reset via "Cashiers" screen (no email needed) |
| No local backup | Manual export to USB/cloud-synced folder + admin-gated import (section 7.6) — no automatic backup yet |
| Thermal printer not detected / offline | Fire-and-forget with circuit breaker retry; checkout must never block on printer failure; PDF invoice (section 7.5.2) always succeeds as system record regardless of printer state |
| ESC/POS quirks on unknown printer brand | Build against generic ESC/POS command subset now; fallback to spooler-passthrough (Generic/Text Only driver) if raw USB (`rusb`) enumeration fails on a given unit |

---

## 13. Questions Before Implementation

**Resolved:**

1. ~~Print requirement~~ — **Decided**: 80mm USB thermal printer via ESC/POS direct (raw USB,
   `rusb`) for customer receipts, auto-fired on checkout, no print dialog. Separately, a PDF
   invoice is generated via `genpdf` and saved to local app data as the permanent system
   record. See section 7.5.
2. ~~Barcode scanner~~ — **Not needed.** Keyboard-emulation only, no Rust/serial integration.
   `serialport` dropped from dependencies.
4. ~~Backup/restore UI~~ — **Decided**: Yes, add it, treated as core (not optional). One-click
   SQLite file/`.sql` dump export to a user-chosen location (USB, cloud-synced folder);
   admin-only import with confirmation step (overwrites current data). See section 7.6.

**Still open:**

3. **Multi-user on same machine**: Current design supports multiple OS users (separate
   app-data). OK to keep as-is?
5. **Code signing**: Windows EV cert available, or self-signed (users see SmartScreen warning)?
6. **Seed data**: Ship the 20 example products (marble/tiles/chips/sanitary) pre-loaded on
   first launch, or start inventory empty and let the store owner add their own catalog from
   scratch? (Relevant since this is confirmed as a fresh install with no existing data to
   migrate — see section 5.)

---

## 14. File Changes Summary

| Category | Files to Modify | Files to Create | Files to Delete |
|----------|-----------------|-----------------|-----------------|
| Config | `package.json`, `vite.config.ts`, `tsconfig.json` | `tauri.conf.json`, `Cargo.toml` | `src/server.ts`, `src/start.ts` |
| Frontend Core | `src/main.tsx` (new), `src/router.tsx`, `src/routes/__root.tsx` | `src/lib/api-client.ts`, `src/lib/tauri-events.ts`, `src/lib/auth-store.ts` | `src/integrations/supabase/*` |
| Features | Update imports in all route files | `src/features/*/api.ts` (7 files) | — |
| Backend | — | `src-tauri/src/**/*.rs` (~25 files, incl. `services/print.rs`, `services/invoice_pdf.rs`, `services/backup.rs`, `commands/print.rs`, `commands/backup.rs`) | — |
| Database | — | `src-tauri/src/database/migrations/*.sql` (5 files) | `supabase/migrations/*` (keep for reference) |
| Build | — | `.github/workflows/release.yml` (optional) | — |

---

## 15. Implementation Order (Recommended)

### Phase 1: Frontend SPA + API Abstraction Layer
1. Create `src/main.tsx` (SPA entry point)
2. Update `vite.config.ts` for SPA mode (remove Nitro)
3. Create `src/lib/api-client.ts` with Tauri invoke wrapper + circuit breaker
4. Create `src/lib/tauri-events.ts` for realtime bridge
5. Create `src/lib/auth-store.ts` (Zustand) for JWT + user state
6. Create `src/features/*/api.ts` for each domain
7. Update all route files to import from feature APIs instead of Supabase
8. Test with mock API (no backend yet)

### Phase 2: Rust Backend Core
1. Initialize Tauri project (`cargo tauri init`)
2. Set up `tauri-plugin-sql` with SQLite
3. Write migrations (5 files matching Supabase schema)
4. Implement repositories (CRUD for each table)
5. Implement auth (JWT + bcrypt + middleware)
6. Implement Tauri commands for all CRUD operations
6. Wire frontend to real backend

### Phase 3: Business Logic Port
1. Port POS checkout logic (atomic transaction)
2. Port returns processing (`process_return` RPC)
3. Port inventory import/export (XLSX via Rust `calamine`)
4. Port reports aggregations
5. Port cashier management
6. Implement `services/print.rs` — ESC/POS receipt formatting + raw USB write (`rusb`);
   wire into checkout as fire-and-forget with circuit breaker retry
7. Implement `services/invoice_pdf.rs` — PDF invoice generation (`genpdf`), saved to app data
8. Implement `services/backup.rs` — export/import SQLite file/dump; admin-only import UI with
   confirmation step

### Phase 4: Real-time + Auto-start + Polish
1. Implement Tauri Event emission on mutations
2. Replace `useInvoiceRealtime` hook
3. Add auto-start plugin
4. Add single-instance plugin
5. Configure CSP, icons, installer
6. Test on clean Windows machine
7. Test thermal receipt printing on actual USB printer once purchased — confirm raw USB
   (`rusb`) enumeration works, fall back to spooler-passthrough if not

### Phase 5: Testing + Release
1. End-to-end testing (POS flow, returns, reports)
2. Print testing
3. Build signed MSI/EXE
4. Documentation for user

---

## 16. Loose Coupling Checklist (Enforce Throughout)

- [ ] Every feature has its own API module (`src/features/x/api.ts`)
- [ ] Every API call goes through circuit breaker
- [ ] Every route wrapped in React Error Boundary
- [ ] Every Tauri command is independent (no shared mutable state)
- [ ] Every service uses its own repository
- [ ] Panic hook installed globally in Rust
- [ ] Database errors mapped to typed `AppError` variants
- [ ] Frontend shows graceful degradation (not crashes)
- [ ] Config externalized (not hardcoded)
- [ ] Logging structured (tracing) for debugging

---

*End of Migration Plan*