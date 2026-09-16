# Stone Flow POS — Complete Project Architecture Blueprint

> **Purpose**: Authoritative single-source reference of system architecture, data models, IPC bridges, hardware interfaces, component topologies, and inter-module dependencies. Scan this file to gain full context without needing to read the entire codebase.
> **Constraint**: Codebase truth only. Do not edit database migrations once applied. Maintain React 18.3.1 pin and whole-rupee canonical currency format.

---

## 1. System Overview & Technology Stack

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           FRONTEND (React 18 / SPA)                             │
│  • React 18.3.1 (pinned) • TanStack Router (file-based) • TanStack Query v5    │
│  • Tailwind CSS 4 • Radix UI Primitives • Lucide React • Sonner Toasts          │
└────────────────────────────────────────┬────────────────────────────────────────┘
                                         │ Tauri IPC Bridge (`invoke` / events)
┌────────────────────────────────────────▼────────────────────────────────────────┐
│                            BACKEND (Tauri v2 / Rust)                            │
│  • Tauri v2 Runtime • Tokio Async Runtime • sqlx (SQLite driver)                │
│  • JWT (HS256) + bcrypt (cost 12) • ab_glyph (TTF receipt rasterizer)           │
│  • genpdf (A4 PDF renderer) • rusb (USB ESC/POS) • Windows Spooler API (RAW)    │
└────────────────────────────────────────┬────────────────────────────────────────┘
                                         │ File I/O
┌────────────────────────────────────────▼────────────────────────────────────────┐
│                               LOCAL STORAGE                                     │
│  • SQLite DB: %PROGRAMDATA%\CityTiles\citytiles.db                              │
│  • JWT Secret: %PROGRAMDATA%\CityTiles\jwt.key                                  │
│  • Auth Store: localStorage ('city-tiles-auth')                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

| Layer | Technology | Key File / Path |
|---|---|---|
| **Desktop Shell** | Tauri v2 (Rust backend + Webview2 frontend) | [src-tauri/tauri.conf.json](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/tauri.conf.json) |
| **Frontend Framework** | React 18.3.1 (exact pin) + TypeScript + Vite | [package.json](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/package.json) |
| **Routing** | TanStack Router (type-safe file routes) | [src/routes/](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src/routes/) |
| **State & Cache** | TanStack Query v5 (React Query) + Zustand | [src/lib/api-client.ts](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src/lib/api-client.ts) |
| **Backend Core** | Rust (Tokio, sqlx, serde, tauri) | [src-tauri/src/lib.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/lib.rs) |
| **Database** | SQLite via `sqlx::SqlitePool` | `%PROGRAMDATA%\CityTiles\citytiles.db` |
| **Receipt Rasterizer** | Monochrome Bitmap Generator (`ab_glyph` + `GS v 0`) | [src-tauri/src/services/receipt_bitmap.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/services/receipt_bitmap.rs) |
| **Printer Transport** | Windows Spooler RAW API + Direct USB (`rusb`) | [src-tauri/src/services/print.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/services/print.rs) |
| **PDF Engine** | `genpdf` with embedded DejaVuSans TTF fonts | [src-tauri/src/services/invoice_pdf.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/services/invoice_pdf.rs) |

---

## 2. Directory Structure & Location Map

```
stone-flow-pos-main/
├── src/                                         # FRONTEND (React SPA)
│   ├── main.tsx                                 # SPA Root Entry Point
│   ├── tauri-entry.tsx                          # Tauri WebView Entry Mount
│   ├── components/                              # UI Components & Shells
│   │   ├── admin/
│   │   │   ├── AdminShell.tsx                   # Main Admin/Staff Navigation Shell
│   │   │   ├── AdminOnly.tsx                    # Route Guard: Admin Role Only
│   │   │   └── AccessGate.tsx                   # Route Guard: Profile Approval Check
│   │   └── ui/                                  # 40+ Radix UI Primitives (Button, Dialog, Table, etc.)
│   ├── features/                                # Feature API Modules
│   │   ├── auth/                                # Authentication API Hooks & Stores
│   │   └── suppliers/                           # Supplier & Purchase Ledger API Hooks
│   ├── lib/                                     # Frontend Infrastructure
│   │   ├── api-client.ts                        # IPC Wrapper (Tauri commands bridge)
│   │   ├── tauri-events.ts                      # Backend Realtime Event Listeners
│   │   ├── auth-store.ts                        # Zustand Auth Store (persisted to localStorage)
│   │   ├── business.ts                          # Business Store Identity Constants
│   │   └── file-save.ts                         # File Export Utilities (Save Dialog)
│   └── routes/                                  # File-Based Route Tree
│       ├── __root.tsx                           # Root Provider Layout (QueryClient, Toaster)
│       ├── index.tsx                            # Login / Staff Sign-in / First-Admin Setup
│       └── _authenticated/                      # Guarded Staff Routes
│           ├── route.tsx                        # Layout Guard (AccessGate + Auth Check)
│           ├── admin.index.tsx                  # Admin Dashboard (Stats, Low Stock, Recent Invoices)
│           ├── admin.pos.tsx                    # POS Checkout Screen (Tile calculations, Cart)
│           ├── admin.inventory.tsx              # Inventory CRUD & Excel Import/Export
│           ├── admin.customers.tsx              # Customer CRM & Outstanding Balance
│           ├── admin.invoices.index.tsx         # Invoice History (Recent 50 + Server Search)
│           ├── admin.invoices.$invoiceId.tsx    # Invoice Detail View (Receipt Print / PDF)
│           ├── admin.returns.tsx                # Returns Processing (Stock restoral)
│           ├── admin.reports.tsx                # Sales Reports & Excel Exports
│           ├── admin.cashiers.tsx               # Staff & Cashier Account Approvals
│           ├── admin.suppliers.tsx              # Supplier Directory & Purchase Ledger
│           └── admin.settings.tsx               # Database Backup, Restore, Retention Settings
│
└── src-tauri/                                   # BACKEND (Rust & Tauri Shell)
    ├── tauri.conf.json                          # Tauri App Config & Window Parameters
    ├── Cargo.toml                               # Rust Dependencies & Features
    └── src/
        ├── main.rs                              # Windows Binary Entry
        ├── lib.rs                               # Main Tauri Builder & Command Registry
        ├── error.rs                             # Unified AppError Types & Serialization
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
        │   └── suppliers.rs                     # Supplier CRUD & Purchase Ledger Commands
        ├── database/                            # Database Infrastructure
        │   ├── connection.rs                    # SqlitePool Connection & Path Resolver
        │   └── migrations/                      # Embedded SQL Migrations (IMMUTABLE!)
        │       ├── 001_initial_schema.sql       # Schema Tables, Enums, Indexes
        │       ├── 002_sequences.sql            # Invoice Sequence Generator
        │       ├── 003_defaults.sql             # Initial Default Data
        │       ├── 004_seed_products.sql        # 20 Sample Products
        │       ├── 005_sales_retention.sql      # Sales Auto-Retention Indexes
    │   ├── 007_suppliers.sql            # Supplier Directory & Purchase Ledger Tables
    │   └── 008_tile_area.sql            # Tile Area Per Tile (REAL) + Invoice Item Total Area (REAL)
        ├── repositories/                        # SQL Abstraction & Business Logic
        │   ├── auth.rs                          # User Profile & Role Repository
        │   ├── products.rs                      # Product Repository & Stock Mutators
        │   ├── customers.rs                     # Customer Repository & Balance Mutators
        │   ├── invoices.rs                      # Invoice Repository (Stock validation, Purge)
        │   ├── returns.rs                       # Return Repository (Stock restoral, Balance adjust)
        │   ├── reports.rs                       # Report Aggregation Queries
        │   └── suppliers.rs                     # Supplier & Purchase Ledger Repository
        ├── services/                            # External Services & Document Generators
        │   ├── receipt_bitmap.rs                # 80mm Bitmap Rasterizer (ab_glyph)
        │   ├── print.rs                         # ESC/POS Spooler RAW & USB Transport
        │   ├── invoice_pdf.rs                   # A4 PDF Invoice Generator (genpdf)
        │   ├── csv_import.rs                    # Flexible Excel/CSV Importer
        │   └── backup.rs                        # VACUUM INTO Backup & Restore Engine
        └── assets/
            └── fonts/                           # Embedded Fonts (Karla, Cormorant Garamond, DejaVu)
```

---

## 3. Database Schema (SQLite)

Located at `%PROGRAMDATA%\CityTiles\citytiles.db`. All monetary amounts are stored in **whole rupees (PKR)** as `INTEGER` (`i64`).

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
    category TEXT NOT NULL, -- 'marble' | 'tiles' | 'chips' | 'sanitary'
    description TEXT,
    color TEXT,
    size TEXT,
    finish TEXT,
    company TEXT,
    unit TEXT NOT NULL DEFAULT 'sqft',
    price INTEGER NOT NULL DEFAULT 0, -- Whole rupees
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
    total INTEGER NOT NULL DEFAULT 0,
    amount_paid INTEGER NOT NULL DEFAULT 0,
    payment_method TEXT NOT NULL DEFAULT 'cash',
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

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
                                      │ 1. Validate Stock: Ensure stock_qty >= qty│
                                      │    (Fails transaction if stock short)     │
                                      │ 2. Insert into `invoices`                 │
                                      │ 3. Insert into `invoice_items`            │
                                      │    └─ total_area = area_per_tile × qty    │
                                      │ 4. UPDATE products SET stock_qty          │
                                      │ 5. UPDATE customers SET balance           │
                                      └─────────────────────┬─────────────────────┘
                                                            │
                                       ┌────────────────────┴────────────────────┐
                                       │ Realtime Event & Auto-Print Dispatch    │
                                       │ 1. Emit `invoices_changed` IPC event    │
                                       │ 2. Trigger `print_receipt` IPC command  │
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
                     │ 4. Decrement customer outstanding_balance │
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
                       │ 2. Insert into `supplier_purchases` (PUR-YYYY-NNNN)       │
                       │ 3. Insert into `supplier_purchase_items` (line items)      │
                       │ 4. UPDATE suppliers SET outstanding_balance += (total-paid) │
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
                     │ 1. UPDATE supplier_purchases SET amount_paid = total          │
                     │ 2. UPDATE suppliers SET outstanding_balance = MAX(0, bal-total)│
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
| **Invoices** | `create_invoice` | `commands/invoices.rs` | `apiClient.invoices.create` | Atomic invoice checkout, stock validation & decrement |
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
| **Suppliers** | `create_supplier_purchase` | `commands/suppliers.rs` | `apiClient.suppliers.createPurchase` | Creates purchase with line items, updates supplier outstanding_balance |
| **Suppliers** | `mark_supplier_purchase_paid` | `commands/suppliers.rs` | `apiClient.suppliers.markPurchasePaid` | Marks purchase as paid, decrements supplier outstanding_balance |

---

## 6. Critical Invariants & Rules (DO NOT BREAK)

> [!CAUTION]
> **1. React Version Pin (18.3.1)**
> `react` and `react-dom` are pinned to **`18.3.1` exact**. Do not upgrade to React 19. React 19 production builds contain an event-dispatch walker loop (`findInstanceBlockingEvent`) that wedges the webview on initial input focus.

> [!IMPORTANT]
> **2. Migration Immutability**
> Never modify existing SQL files in [src-tauri/src/database/migrations/](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/database/migrations/). `sqlx` validates checksums on startup. Editing existing migrations will crash every installed application. All future schema alterations must go in new numbered migration files (e.g. `006_feature_name.sql`).

> [!IMPORTANT]
> **3. Whole Rupees Canonical Format**
> All monetary values across the SQLite database, Rust struct models, IPC payloads, and UI screens are stored and displayed in **whole Pakistani Rupees (PKR)**. Never divide by 100 or introduce decimal paise logic.

> [!WARNING]
> **4. Hard Stock Block on Oversell**
> Invoice checkout validates per-product stock availability within the database transaction. If cart quantity exceeds available `stock_qty`, the transaction rolls back with a stock violation error.

> [!NOTE]
> **5. Default Printer Targeting**
> 80mm thermal receipt printing routes directly to the **Windows Default Printer** via the Win32 Print Spooler RAW API. Ensure the thermal receipt printer is set as the Windows default printer on the host machine.
