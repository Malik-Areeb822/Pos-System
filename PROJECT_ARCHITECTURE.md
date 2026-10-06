# Moon Pipe and Sanitary POS — Complete Project Architecture Blueprint

> **Purpose**: Authoritative single-source reference of system architecture, data models, IPC bridges, hardware interfaces, component topologies, and inter-module dependencies. Scan this file to gain full context without needing to read the entire codebase.
> **Constraint**: Codebase truth only. Do not edit database migrations once applied. Maintain React 18.3.1 pin and whole-rupee canonical currency format.

---

## 1. System Overview & Technology Stack

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           FRONTEND (React 18 / SPA)                             │
│  • React 18.3.1 (pinned) • TanStack Router (file-based) • TanStack Query v5    │
│  • Tailwind CSS 4 • Radix UI Primitives • Lucide React • Sonner Toasts          │
│  • System Lock Gate (LicenseGate + KeyComboListener)                            │
└────────────────────────────────────────┬────────────────────────────────────────┘
                                         │ Tauri IPC Bridge (`invoke` / events)
┌────────────────────────────────────────▼────────────────────────────────────────┐
│                            BACKEND (Tauri v2 / Rust)                            │
│  • Tauri v2 Runtime • Tokio Async Runtime • sqlx (SQLite driver)                │
│  • JWT (HS256) + bcrypt (cost 12) • ab_glyph (TTF receipt rasterizer)           │
│  • genpdf (A4 PDF renderer) • rusb (USB ESC/POS) • Windows Spooler API (RAW)    │
│  • HMAC (lock state signing) • bcrypt (password verification)                   │
└────────────────────────────────────────┬────────────────────────────────────────┘
                                         │ File I/O
┌────────────────────────────────────────▼────────────────────────────────────────┐
│                               LOCAL STORAGE                                     │
│  • SQLite DB: %PROGRAMDATA%\MoonPipe\moonpipe.db                               │
│  • JWT Secret: %PROGRAMDATA%\MoonPipe\jwt.key                                  │
│  • Auth Store: localStorage ('moonpipe-auth')                                   │
│  • Code Signing: CN=AZ Solutions (self-signed, valid to 2031)                   │
└─────────────────────────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Key File / Path |
|---|---|---|
| **Desktop Shell** | Tauri v2 (Rust backend + Webview2 frontend) | [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) |
| **Frontend Framework** | React 18.3.1 (exact pin) + TypeScript + Vite | [package.json](package.json) |
| **Routing** | TanStack Router (type-safe file routes) | [src/routes/](src/routes/) |
| **State & Cache** | TanStack Query v5 (React Query) + Zustand | [src/lib/api-client.ts](src/lib/api-client.ts) |
| **Backend Core** | Rust (Tokio, sqlx, serde, tauri) | [src-tauri/src/lib.rs](src-tauri/src/lib.rs) |
| **Database** | SQLite via `sqlx::SqlitePool` | `%PROGRAMDATA%\MoonPipe\moonpipe.db` |
| **System Lock** | HMAC-signed lock state + bcrypt password | [src-tauri/src/license/mod.rs](src-tauri/src/license/mod.rs) |
| **Receipt Rasterizer** | Monochrome Bitmap Generator (`ab_glyph` + `GS v 0`) | [src-tauri/src/services/receipt_bitmap.rs](src-tauri/src/services/receipt_bitmap.rs) |
| **Printer Transport** | Windows Spooler RAW API + Direct USB (`rusb`) | [src-tauri/src/services/print.rs](src-tauri/src/services/print.rs) |
| **PDF Engine** | `genpdf` with embedded DejaVuSans TTF fonts | [src-tauri/src/services/invoice_pdf.rs](src-tauri/src/services/invoice_pdf.rs) |
| **Release** | `cargo tauri build` (vite SPA → `dist-spa` → `custom-protocol` → WiX MSI + makensis NSIS), signed post-build — **v1.1.0** | [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) |

---

## 2. Directory Structure & Location Map

```
moonpipe-pos-main/
├── src/                                         # FRONTEND (React SPA)
│   ├── main.tsx                                 # SPA Root Entry Point
│   ├── tauri-entry.tsx                          # Tauri WebView Entry Mount
│   ├── components/                              # UI Components & Shells
│   │   ├── admin/
│   │   │   ├── AdminShell.tsx                   # Main Admin/Staff Navigation Shell
│   │   │   ├── AdminOnly.tsx                    # Route Guard: Admin Role Only
│   │   │   └── AccessGate.tsx                   # Route Guard: Profile Approval Check
│   │   ├── license/
│   │   │   ├── KeyComboListener.tsx             # Secret combo detector (lockdownsystem)
│   │   │   ├── LockScreen.tsx                   # Full-screen lock overlay
│   │   │   └── LicenseGate.tsx                  # Top-level lock state gate
│   │   └── ui/                                  # 40+ Radix UI Primitives (Button, Dialog, Table, etc.)
│   ├── features/                                # Feature API Modules
│   │   ├── auth/                                # Authentication API Hooks & Stores
│   │   ├── license/                             # System Lock API Hooks
│   │   │   └── api.ts                           # checkSystemLock(), unlockWithPassword()
│   │   └── suppliers/                           # Supplier & Purchase Ledger API Hooks
│   ├── lib/                                     # Frontend Infrastructure
│   │   ├── api-client.ts                        # IPC Wrapper (Tauri commands bridge)
│   │   ├── tauri-events.ts                      # Backend Realtime Event Listeners
│   │   ├── auth-store.ts                        # Zustand Auth Store (persisted to localStorage)
│   │   ├── license-store.ts                     # Zustand Lock State Store
│   │   ├── business.ts                          # Business Store Identity Constants
│   │   └── file-save.ts                         # File Export Utilities (Save Dialog)
│   ├── styles.css                               # Tailwind CSS + custom properties (teal accent)
│   └── routes/                                  # File-Based Route Tree
│       ├── __root.tsx                           # Root Provider Layout (QueryClient, Toaster)
│       ├── index.tsx                            # Login / Staff Sign-in / First-Admin Setup
│       └── _authenticated/                      # Guarded Staff Routes
│           ├── route.tsx                        # Layout Guard (AccessGate + Auth Check)
│           ├── admin.index.tsx                  # Admin Dashboard (Stats, Low Stock, Recent Invoices)
│           ├── admin.pos.tsx                    # POS Checkout Screen (Cart, Customer selection)
│           ├── admin.inventory.tsx              # Inventory CRUD & Excel Import/Export
│           ├── admin.customers.tsx              # Customer CRM & Outstanding Balance
│           ├── admin.invoices.index.tsx         # Invoice History (Recent 50 + Server Search)
│           ├── admin.invoices.$invoiceId.tsx    # Invoice Detail View (Receipt Print / PDF)
│           ├── admin.returns.tsx                # Returns Processing (Stock restoral)
│           ├── admin.reports.tsx                # Sales Reports & Excel Exports
│           ├── admin.cashiers.tsx               # Staff & Cashier Account Approvals
│           ├── admin.suppliers.tsx              # Supplier Directory & Purchase Ledger
│           └── admin.settings.tsx               # Database Backup, Restore, Retention Settings
├── signing-cert.pfx                             # Self-signed code signing cert (CN=AZ Solutions) — GITIGNORED, repo root only
└── src-tauri/                                   # BACKEND (Rust & Tauri Shell)
    ├── tauri.conf.json                          # Tauri App Config & Window Parameters
    ├── Cargo.toml                               # Rust Dependencies & Features
    └── src/
        ├── main.rs                              # Windows Binary Entry
        ├── lib.rs                               # Main Tauri Builder & Command Registry
        ├── error.rs                             # Unified AppError Types & Serialization
        ├── license/
        │   └── mod.rs                           # HMAC signing, bcrypt verification, lock state
        ├── commands/                            # IPC Command Handlers
        │   ├── auth.rs                          # Login, Register, Password Reset, Status Change
        │   ├── products.rs                      # Product CRUD & Stock Queries
        │   ├── customers.rs                     # Customer CRUD & Balance Queries
        │   ├── invoices.rs                      # Invoice Creation, Listing, Details, Search
        │   ├── returns.rs                       # Return Creation & Queries
        │   ├── reports.rs                       # Sales & Inventory Reporting Queries
        │   ├── print.rs                         # Thermal Receipt & PDF Invoice Commands
        │   ├── backup.rs                        # DB Snapshot, Restore & Purge Commands
        │   ├── cashiers.rs                      # Staff Approval & Cashier Admin Queries
        │   ├── suppliers.rs                     # Supplier CRUD & Purchase Ledger Commands
        │   ├── reconciliation.rs                # Balance Reconciliation Command
        │   └── license.rs                       # System Lock Check/Set/Unlock Commands
        ├── database/                            # Database Infrastructure
        │   ├── connection.rs                    # SqlitePool Connection & Path Resolver
        │   └── migrations/                      # Embedded SQL Migrations (IMMUTABLE!)
        │       ├── 001_initial_schema.sql       # Schema Tables, Enums, Indexes
        │       ├── 002_enum_constraints.sql     # Category & Status CHECK Constraints
        │       ├── 003_invoice_counter.sql      # Invoice Sequence Generator
        │       ├── 004_seed_data.sql            # 20 Sample Products
        │       ├── 005_admin_user.sql           # Default Admin User
        │       ├── 006_product_company.sql      # Company field for products
        │       ├── 007_suppliers.sql            # Supplier Directory & Purchase Ledger Tables
        │       ├── 008_tile_area.sql            # Tile Area Per Tile (REAL) + Invoice Item Total Area (REAL)
        │       ├── 009_invoice_previous_balance.sql  # Invoice Previous Balance (INTEGER) for carry-forward
        │       ├── 010_add_purchase_price.sql   # Purchase price for profit tracking
        │       ├── 011_seed_moonpipe.sql        # Moon Pipe seed products
        │       ├── 012_cleanup_citytiles_seed.sql  # Remove CityTiles seed data
        │       ├── 013_add_hardware_category.sql  # Add 'hardware' category
        │       ├── 014_add_carried_to_invoice_id.sql  # Invoice carry-forward tracking
        │       ├── 015_add_invoice_constraints.sql  # Discount/paid CHECK constraints
        │       ├── 016_add_app_settings.sql     # App Settings table (lock state)
        │       └── 017_supplier_purchase_carry_forward.sql  # Supplier purchase carry-forward (previous_balance + carried_to_purchase_id)
        ├── repositories/                        # SQL Abstraction & Business Logic
        │   ├── auth.rs                          # User Profile & Role Repository
        │   ├── products.rs                      # Product Repository & Stock Mutators
        │   ├── customers.rs                     # Customer Repository (balance is derived, not mutated — see services/reconciliation.rs)
        │   ├── invoices.rs                      # Invoice Repository (Stock validation, Purge)
        │   ├── returns.rs                       # Return Repository (Stock restoral, Balance adjust)
        │   ├── reports.rs                       # Report Aggregation Queries
        │   └── suppliers.rs                     # Supplier & Purchase Ledger Repository
        ├── services/                            # External Services & Document Generators
        │   ├── receipt_bitmap.rs                # 80mm Bitmap Rasterizer (ab_glyph)
        │   ├── print.rs                         # ESC/POS Spooler RAW & USB Transport
        │   ├── invoice_pdf.rs                   # A4 PDF Invoice Generator (genpdf)
        │   ├── csv_import.rs                    # Flexible Excel/CSV Importer
        │   ├── backup.rs                        # VACUUM INTO Backup & Restore Engine
        │   └── reconciliation.rs                # Balance Reconciliation Service
        └── assets/
            └── fonts/                           # Embedded Fonts (Karla, Cormorant Garamond, DejaVu)
```

---

## 3. Database Schema (SQLite)

Located at `%PROGRAMDATA%\MoonPipe\moonpipe.db`. All monetary amounts are stored in **whole rupees (PKR)** as `INTEGER` (`i64`).

```sql
-- 1. Profiles (Staff & Admin Accounts)
CREATE TABLE profiles (
    id TEXT PRIMARY KEY NOT NULL,
    full_name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    phone TEXT,
    employee_id TEXT,
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'approved' | 'rejected' | 'suspended'
    approved_at TEXT,
    rejected_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 2. User Roles
CREATE TABLE user_roles (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    role TEXT NOT NULL, -- 'admin' | 'cashier'
    created_at TEXT NOT NULL,
    UNIQUE(user_id, role)
);

-- 3. Products (Inventory Catalog)
CREATE TABLE products (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    sku TEXT UNIQUE,
    category TEXT NOT NULL, -- 'sanitary' | 'hardware' (client B) or 'marble' | 'tiles' | 'chips' (client A)
    description TEXT,
    color TEXT,
    size TEXT,
    finish TEXT,
    company TEXT,
    unit TEXT NOT NULL DEFAULT 'sqft',
    price INTEGER NOT NULL DEFAULT 0, -- Whole rupees (selling price)
    purchase_price INTEGER,          -- Cost price for profit tracking
    pieces_per_carton INTEGER,
    area_per_tile REAL,              -- Tile area in sqm (only for 'tiles' category)
    stock_qty INTEGER NOT NULL DEFAULT 0,
    low_stock_threshold INTEGER NOT NULL DEFAULT 10,
    image_url TEXT,
    is_published INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 4. Customers (CRM & Credit Ledger)
CREATE TABLE customers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    address TEXT,
    outstanding_balance INTEGER NOT NULL DEFAULT 0, -- Whole rupees
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 5. Invoices (Sales Master)
CREATE TABLE invoices (
    id TEXT PRIMARY KEY NOT NULL,
    invoice_no TEXT UNIQUE NOT NULL, -- Format: INV-YYYY-NNNN
    customer_id TEXT REFERENCES customers(id) ON DELETE SET NULL,
    customer_name TEXT NOT NULL,
    subtotal INTEGER NOT NULL DEFAULT 0,
    discount INTEGER NOT NULL DEFAULT 0,
    total INTEGER NOT NULL DEFAULT 0,
    amount_paid INTEGER NOT NULL DEFAULT 0,
    payment_method TEXT NOT NULL DEFAULT 'cash', -- 'cash' | 'bank' | 'credit'
    previous_balance INTEGER NOT NULL DEFAULT 0, -- Carried from customer's outstanding_balance at creation
    carried_to_invoice_id TEXT REFERENCES invoices(id), -- Invoice this balance was carried forward to
    notes TEXT,
    delivery_date TEXT,
    created_by TEXT REFERENCES profiles(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 6. Invoice Items (Sales Line Items)
CREATE TABLE invoice_items (
    id TEXT PRIMARY KEY NOT NULL,
    invoice_id TEXT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    product_id TEXT REFERENCES products(id) ON DELETE SET NULL,
    product_name TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    unit TEXT NOT NULL DEFAULT 'sqft',
    unit_price INTEGER NOT NULL DEFAULT 0,
    line_total INTEGER NOT NULL DEFAULT 0,
    total_area REAL,                  -- area_per_tile × qty (computed at invoice creation)
    created_at TEXT NOT NULL
);

-- 7. Returns (Return Master)
CREATE TABLE returns (
    id TEXT PRIMARY KEY NOT NULL,
    invoice_id TEXT NOT NULL REFERENCES invoices(id),
    invoice_no TEXT NOT NULL,
    customer_id TEXT REFERENCES customers(id),
    customer_name TEXT NOT NULL,
    total INTEGER NOT NULL DEFAULT 0,
    reason TEXT,
    created_by TEXT REFERENCES profiles(id),
    created_at TEXT NOT NULL
);

-- 8. Return Items (Return Line Items)
CREATE TABLE return_items (
    id TEXT PRIMARY KEY NOT NULL,
    return_id TEXT NOT NULL REFERENCES returns(id) ON DELETE CASCADE,
    product_id TEXT REFERENCES products(id),
    product_name TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    unit TEXT NOT NULL,
    unit_price INTEGER NOT NULL DEFAULT 0,
    line_total INTEGER NOT NULL DEFAULT 0
);

-- 9. Suppliers (Supplier Directory)
CREATE TABLE suppliers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    company TEXT,
    address TEXT,
    notes TEXT,
    outstanding_balance INTEGER NOT NULL DEFAULT 0, -- Whole rupees owed to supplier
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_suppliers_name ON suppliers(name);
CREATE INDEX idx_suppliers_company ON suppliers(company);

-- 10. Supplier Purchases (Purchase Ledger Master)
CREATE TABLE supplier_purchases (
    id TEXT PRIMARY KEY NOT NULL,
    purchase_no TEXT UNIQUE NOT NULL, -- Format: PUR-YYYY-NNNN
    supplier_id TEXT NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    supplier_name TEXT NOT NULL,
    subtotal INTEGER NOT NULL DEFAULT 0,
    discount INTEGER NOT NULL DEFAULT 0,
    total INTEGER NOT NULL DEFAULT 0, -- = subtotal - discount + previous_balance (backend-computed)
    previous_balance INTEGER NOT NULL DEFAULT 0, -- Carried from supplier's outstanding_balance at creation
    carried_to_purchase_id TEXT REFERENCES supplier_purchases(id), -- Purchase this balance was carried forward to (NULL = chain leaf)
    amount_paid INTEGER NOT NULL DEFAULT 0,
    payment_method TEXT NOT NULL DEFAULT 'cash',
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_supplier_purchases_carried_to ON supplier_purchases(carried_to_purchase_id);

-- 11. Supplier Purchase Items (Purchase Line Items)
CREATE TABLE supplier_purchase_items (
    id TEXT PRIMARY KEY NOT NULL,
    purchase_id TEXT NOT NULL REFERENCES supplier_purchases(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    unit TEXT NOT NULL DEFAULT 'pcs',
    unit_price INTEGER NOT NULL DEFAULT 0,
    line_total INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

-- 12. App Settings (Key-Value Store)
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX idx_app_settings_key ON app_settings(key);
```

---

## 4. End-to-End Core Workflows & Connections

### A. Point of Sale (POS) Checkout & Inventory Decrement

```
[User clicks Checkout in /admin/pos]
        │
        ▼
[apiClient.invoices.create(input)] ──(Tauri IPC)──► [commands::invoices::create_invoice]
                                                            │
                                                            ▼
                                                [InvoiceRepository::create]
                                                            │
                                      ┌─────────────────────┴─────────────────────┐
                                      │ In-Transaction SQL Executions             │
                                      │ 1. Read customer's outstanding_balance    │
                                      │    → stored as previous_balance on invoice│
                                      │ 2. Compute total = subtotal - discount    │
                                      │    + previous_balance                     │
                                      │ 3. Validate Stock: stock_qty >= qty       │
                                      │    (Fails transaction if stock short)     │
                                      │ 4. Insert into `invoices`                 │
                                      │ 5. Insert into `invoice_items`            │
                                      │    └─ total_area = area_per_tile × qty    │
                                      │ 6. UPDATE products SET stock_qty          │
                                      │ 7. Mark absorbed source invoices:         │
                                      │    UPDATE invoices SET carried_to_        │
                                      │    invoice_id = <new id> WHERE customer   │
                                      │    AND open AND uncarried (every one)     │
                                      │ 8. UPDATE customers SET outstanding_      │
                                      │    balance = MAX(0, total - amount_paid)  │
                                      └─────────────────────┬─────────────────────┘
                                                            │
                                       ┌────────────────────┴────────────────────┐
                                       │ Realtime Event & Auto-Print Dispatch    │
                                       │ 1. Emit `invoices_changed` IPC event   │
                                       │ 2. Trigger `print_receipt` IPC command │
                                       └─────────────────────────────────────────┘
```

### B. Thermal Receipt Printing Pipeline

```
[Trigger: POS Checkout / Invoices List Print / Detail Page Print]
        │
        ▼
[IPC Call: print_receipt(invoice_id, business)]
        │
        ▼
[src-tauri/src/services/print.rs :: print_receipt()]
        │
        ├─► [Primary Route] ──► [src-tauri/src/services/receipt_bitmap.rs :: render_receipt()]
        │                             │
        │                             ├─► Load Karla & Cormorant Garamond TTF fonts
        │                             ├─► Render 576px wide raster monochrome canvas (203 DPI)
        │                             └─► Encode canvas rows into ESC/POS `GS v 0` commands
        │
        └─► [Fallback Route] ─► [src-tauri/src/services/print.rs :: build_escpos_receipt()]
                                      │ (Used only if font loading fails)
                                      └─► Format 48-column classic ESC/POS text stream
        │
        ▼
[Transport Selection]
        ├─► [Path 1 (Windows Default)]: Win32 Spooler API (`StartDocPrinterW`, `WritePrinter` with RAW mode)
        └─► [Path 2 (Fallback)]: Direct USB write via `rusb` (scans for printer class 0x07 or known VIDs)
```

### C. Return Processing & Stock Restoral Pipeline

```
[User submits Return in /admin/returns]
        │
        ▼
[IPC Call: create_return(input)] ──► [ReturnRepository::create]
                                           │
                     ┌─────────────────────┴─────────────────────┐
                     │ In-Transaction Execution                  │
                     │ 1. Create `returns` & `return_items`      │
                     │ 2. UPDATE products SET stock_qty + qty    │
                     │ 3. Adjust `invoices.total` & `amount_paid`│
                     │    + cascade the reduction forward through│
                     │    `carried_to_invoice_id` successors     │
                     │    (depth-capped walk)                    │
                     │ 4. Recompute customer outstanding_balance │
                     │    from LEAF invoices (never `+=` nudge)  │
                     └─────────────────────┬─────────────────────┘
                                           │
                     ┌─────────────────────┴─────────────────────┐
                     │ Emit Events                               │
                     │ 1. `invoices_changed`                     │
                     │ 2. `inventory_changed`                    │
                     └───────────────────────────────────────────┘
```

### D. Supplier Purchase Recording & Balance Tracking

```
[User records purchase in /admin/suppliers]
        │
        ▼
[IPC Call: create_supplier_purchase(input)] ──► [create_supplier_purchase]
                                                     │
                       ┌─────────────────────────────┴─────────────────────────────┐
                       │ In-Transaction Execution                                  │
                       │ 1. Validate supplier_id exists                            │
                       │ 2. Read supplier outstanding_balance → previous_balance   │
                       │ 3. Compute total = subtotal - discount + previous_balance │
                       │    (Amount Paid clamped to grand total)                   │
                       │ 4. Insert into `supplier_purchases` (PUR-YYYY-NNNN)       │
                       │    with carried_to_purchase_id = NULL (chain head)        │
                       │ 5. Insert into `supplier_purchase_items` (line items)     │
                       │ 6. If previous_balance > 0: mark EVERY open uncarried     │
                       │    purchase carried_to_purchase_id = <new id>             │
                       │ 7. UPDATE suppliers SET outstanding_balance =             │
                       │    MAX(0, total - amount_paid)   (SET, never `+=`)        │
                       └─────────────────────────────┬─────────────────────────────┘
                                                     │
                       ┌─────────────────────────────┴─────────────────────────────┐
                       │ Emit Event: `suppliers_changed`                           │
                       └───────────────────────────────────────────────────────────┘

[User clicks "Mark as Paid" on a purchase]
        │
        ▼
[IPC Call: mark_supplier_purchase_paid(input)] ──► [mark_supplier_purchase_paid]
                                                       │
                     ┌─────────────────────────────────┴─────────────────────────────┐
                     │ In-Transaction Execution                                      │
                     │ 0. GUARD: if carried_to_purchase_id IS SET → reject with      │
                     │    validation error naming the successor purchase_no          │
                     │ 1. UPDATE supplier_purchases SET amount_paid = total          │
                     │ 2. Recompute supplier outstanding_balance from the ledger     │
                     │    (leaf purchases only)                                      │
                     └─────────────────────────────────┬─────────────────────────────┘
                                                       │
                     ┌─────────────────────────────────┴─────────────────────────────┐
                     │ Emit Event: `suppliers_changed`                               │
                     └───────────────────────────────────────────────────────────────┘
```

---

## 5. IPC Command Bridge Reference

The frontend interacts with Rust backend commands exclusively through `apiInvoke` defined in [src/lib/api-client.ts](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src/lib/api-client.ts).

| Domain | Tauri Command Name | Rust File | Frontend Wrapper | Description |
|---|---|---|---|---|
| **Auth** | `login` | `commands/auth.rs` | `apiClient.auth.login` | Authenticates staff, returns JWT token & user profile |
| **Auth** | `register` | `commands/auth.rs` | `apiClient.auth.register` | Registers staff account (First user → Admin; Rest → Pending) |
| **Auth** | `check_admin_exists` | `commands/auth.rs` | `apiClient.auth.checkAdminExists` | Returns `true` if an admin profile exists |
| **Products** | `list_products` | `commands/products.rs` | `apiClient.inventory.list` | Retrieves catalog products with category/search filters |
| **Products** | `create_product` | `commands/products.rs` | `apiClient.inventory.create` | Inserts new product into inventory |
| **Products** | `update_product` | `commands/products.rs` | `apiClient.inventory.update` | Updates product details, prices, or stock threshold |
| **Invoices** | `create_invoice` | `commands/invoices.rs` | `apiClient.invoices.create` | Atomic invoice checkout; reads customer balance → `previous_balance`; computes total; validates stock; updates balance |
| **Invoices** | `list_invoices` | `commands/invoices.rs` | `apiClient.invoices.list` | Fetches newest 50 invoices or runs server search (cap 200) |
| **Invoices** | `get_invoice` | `commands/invoices.rs` | `apiClient.invoices.get` | Returns invoice master details + line items |
| **Print** | `print_receipt` | `commands/print.rs` | `apiClient.invoices.printReceipt` | Generates raster receipt bitmap and prints via Spooler/USB |
| **Print** | `print_invoice_pdf` | `commands/print.rs` | `apiClient.invoices.printInvoicePdf` | Generates A4 PDF invoice file and returns file path |
| **Returns** | `create_return` | `commands/returns.rs` | `apiClient.returns.create` | Processes return, restores stock, adjusts customer balance |
| **Backup** | `create_backup` | `commands/backup.rs` | `apiClient.backup.create` | Runs `VACUUM INTO` live SQLite snapshot |
| **Backup** | `restore_backup` | `commands/backup.rs` | `apiClient.backup.restore` | Validates DB, creates pre-restore snapshot, restores DB |
| **Backup** | `purge_old_invoices` | `commands/backup.rs` | `apiClient.backup.purge` | Purges settled invoices older than 12 months with snapshot |
| **Suppliers** | `list_suppliers` | `commands/suppliers.rs` | `apiClient.suppliers.list` | Lists all suppliers ordered by name |
| **Suppliers** | `get_supplier` | `commands/suppliers.rs` | `apiClient.suppliers.get` | Returns single supplier by ID |
| **Suppliers** | `create_supplier` | `commands/suppliers.rs` | `apiClient.suppliers.create` | Creates new supplier with outstanding_balance=0 |
| **Suppliers** | `update_supplier` | `commands/suppliers.rs` | `apiClient.suppliers.update` | Updates supplier details (name, phone, email, company, address, notes) |
| **Suppliers** | `delete_supplier` | `commands/suppliers.rs` | `apiClient.suppliers.delete` | Deletes supplier (blocked if purchases exist due to FK RESTRICT) |
| **Suppliers** | `list_supplier_purchases` | `commands/suppliers.rs` | `apiClient.suppliers.listPurchases` | Lists purchases for a supplier (newest first) |
| **Suppliers** | `get_supplier_purchase` | `commands/suppliers.rs` | `apiClient.suppliers.getPurchase` | Returns single purchase by ID |
| **Suppliers** | `get_supplier_purchase_with_items` | `commands/suppliers.rs` | `apiClient.suppliers.getPurchaseWithItems` | Returns purchase + line items tuple → `{ purchase, items }` |
| **Suppliers** | `create_supplier_purchase` | `commands/suppliers.rs` | `apiClient.suppliers.createPurchase` | Carries supplier balance into purchase (`previous_balance`), total = subtotal − discount + previous_balance, marks absorbed open purchases, SETs supplier outstanding_balance |
| **Suppliers** | `mark_supplier_purchase_paid` | `commands/suppliers.rs` | `apiClient.suppliers.markPurchasePaid` | Marks purchase as paid — **rejected** if the purchase was carried forward (names the successor); recomputes supplier balance from the ledger |
| **Reconciliation** | `reconcile_balances` | `commands/reconciliation.rs` | — (no UI since 2026-10-03) | Recomputes **customer + supplier** balances from non-carried open rows; runs automatically at startup and after backup restore |
| **License** | `check_system_lock` | `commands/license.rs` | `checkSystemLock()` | Returns lock status from app_settings (no auth required) |
| **License** | `set_system_lock` | `commands/license.rs` | via invoke directly | Sets lock state (no JWT — combo IS the authentication) |
| **License** | `unlock_with_password` | `commands/license.rs` | `unlockWithPassword()` | Verifies bcrypt password hash, unlocks system |

---

## 6. Critical Invariants & Rules (DO NOT BREAK)

> [!CAUTION]
> **1. React Version Pin (18.3.1)**
> `react` and `react-dom` are pinned to **`18.3.1` exact**. Do not upgrade to React 19. React 19 production builds contain an event-dispatch walker loop (`findInstanceBlockingEvent`) that wedges the webview on initial input focus.

> [!IMPORTANT]
> **2. Migration Immutability**
> Never modify existing SQL files in [src-tauri/src/database/migrations/](src-tauri/src/database/migrations/). `sqlx` validates checksums on startup. Editing existing migrations will crash every installed application. All future schema alterations must go in new numbered migration files (e.g. `017_feature_name.sql`).

> [!IMPORTANT]
> **3. Whole Rupees Canonical Format**
> All monetary values across the SQLite database, Rust struct models, IPC payloads, and UI screens are stored and displayed in **whole Pakistani Rupees (PKR)**. Never divide by 100 or introduce decimal paise logic.

> [!WARNING]
> **4. Hard Stock Block on Oversell**
> Invoice checkout validates per-product stock availability within the database transaction. If cart quantity exceeds available `stock_qty`, the transaction rolls back with a stock violation error.

> [!NOTE]
> **5. Default Printer Targeting**
> 80mm thermal receipt printing routes directly to the **Windows Default Printer** via the Win32 Print Spooler RAW API. Ensure the thermal receipt printer is set as the Windows default printer on the host machine.

> [!IMPORTANT]
> **6. Invoice Chain — Unpaid Balances Carry Forward**
> When creating an invoice for a named customer with an outstanding balance, the balance is read from `customers.outstanding_balance` and stored as `previous_balance` on the new invoice. The invoice `total` is computed as `subtotal - discount + previous_balance`. After payment, `outstanding_balance` is SET to `MAX(0, total - amount_paid)` — not accumulated. Every open invoice absorbed by the carry is marked `carried_to_invoice_id = <new id>`. Returns cascade the reduction through the whole `carried_to_invoice_id` chain (depth-capped) and balances are recomputed from **leaf** invoices only. `customers.outstanding_balance` is a **mirror**: it equals `SUM(total - amount_paid)` over non-carried open invoices and is re-derived at startup and after every backup restore (`services/reconciliation.rs`) — never hand-edited. All sales/revenue aggregates use `total - previous_balance` (net revenue — `previous_balance` was already counted when it first changed hands).

> [!IMPORTANT]
> **7. System Lock — Secret Combo**
> Type `lockdownsystem` within 4 seconds (no modifiers) to lock the system. Lock state persists in `app_settings` table with HMAC signature. Only the password (`Areeb@1234`, bcrypt-hashed) can unlock. The key combo listener is active in all states (locked and unlocked). It does NOT trigger on input/textarea/contenteditable elements. The `set_system_lock` command has NO JWT authentication — the combo itself IS the authentication. `require_unlocked()` middleware blocks all sensitive commands when locked.

> [!IMPORTANT]
> **8. Code Signing**
> Self-signed certificate `CN=AZ Solutions` (SHA-256, valid to 2031). Password: `MoonPipe2026`. Certificate file: `signing-cert.pfx` in project root — **gitignored, never commit it.** Both MSI and NSIS installers are signed after `cargo tauri build` with a separate `signtool` pass (Tauri itself does not sign):
> `signtool sign /f signing-cert.pfx /p MoonPipe2026 /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 <artifact>`
> Signer thumbprint `2D1CFE03D65E62BF992595D5E372BCDE5BC6E74A`. The **public** cert must be imported into `LocalMachine\Root` **and** `LocalMachine\TrustedPublisher` (never the private key) or `signtool verify /pa` fails with *"chain terminated in a root certificate which is not trusted"* and installers show **Unknown publisher**. With it installed, `signtool verify /pa` = Successfully verified and `Get-AuthenticodeSignature` = Valid. SmartScreen reputation warnings are still expected — self-signed certs never accumulate reputation.

> [!IMPORTANT]
> **9. Supplier Purchase Chain — Mirror of the Invoice Chain**
> `supplier_purchases.previous_balance` is read from `suppliers.outstanding_balance` at creation; `total = subtotal - discount + previous_balance`; supplier balance is SET to `MAX(0, total - amount_paid)` (never `+=`). Absorbed open purchases are marked `carried_to_purchase_id = <new id>` (all of them). `mark_supplier_purchase_paid` rejects purchases with a non-NULL `carried_to_purchase_id`, naming the successor. `suppliers.outstanding_balance` is derived from non-carried open purchases and re-computed by the same reconciliation pass that fixes customers. Column name is `carried_to_purchase_id` — **never** `carried_to_invoice_id` (migration 017).

> [!IMPORTANT]
> **10. Release Build — `cargo tauri build`, never bare `cargo build`**
> The shippable artifacts come from `cargo tauri build` (or `npx tauri build`), which runs `beforeBuildCommand` (`npx vite build --config vite.config.spa.ts` → `dist-spa/`), then compiles with the **`custom-protocol` feature**, then bundles WiX (MSI) + makensis (NSIS). Version is declared in **two** places that must match: `src-tauri/tauri.conf.json` (drives the installer filename and app version) and `src-tauri/Cargo.toml` (drives the exe/file version). Current release: **1.1.0** → `src-tauri/target/release/bundle/msi/Moon Pipe POS_1.1.0_x64_en-US.msi` and `.../nsis/Moon Pipe POS_1.1.0_x64-setup.exe`. Test/lint gates before any build: `cargo test --lib`, `npm run lint`, `npx tsc --noEmit`.

> [!CAUTION]
> **11. Debug Build Traps — localhost:8080 and the vanishing console**
> **(a) URL:** `tauri/build.rs` computes `dev = !custom_protocol`, and `tauri-2.11.5` `manager/mod.rs` does `#[cfg(dev)] let url = self.config.build.dev_url`. A binary from a bare `cargo build` **or `cargo build --release`** has `custom-protocol` off → `cfg(dev)` on → the webview loads `devUrl` (`http://localhost:8080`) and **never falls back to `frontendDist`**, producing `ERR_CONNECTION_REFUSED` unless `npx vite --config vite.config.spa.ts --port 8080` (the `beforeDevCommand`) is running. Only `cargo tauri build` flips the feature; the shipped exe needs no server.
> **(b) Console:** `src-tauri/src/main.rs:2` is `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`, so **debug** builds are console apps — closing their CMD window sends `CTRL_CLOSE_EVENT` and terminates the POS. Release builds allocate no console. Logs always land in `AppData\Local\com.moonpipe.pos\logs\Moon Pipe POS.log`.
