# Suppliers Module — Detailed Implementation Plan

> **Goal**: Add a full Suppliers module with supplier CRUD and a partial-payment purchase ledger that mirrors the existing customer/invoice billing patterns — **zero conflicts** with existing system.
>
> **Constraint**: Follow every convention from [PROJECT_ARCHITECTURE.md](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/PROJECT_ARCHITECTURE.md) and [HANDOFF.md](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/HANDOFF.md). All money in whole PKR integers. Migration immutability respected. React 18.3.1 pin untouched.

---

## 1. Concept & Domain Model

### What is the Suppliers module?

A **parallel ledger system** for tracking money the business **owes to suppliers** (the existing system tracks money **customers owe to the business**). It consists of two sub-features:

| Sub-feature | Analogous to | Purpose |
|---|---|---|
| **Supplier CRUD** | Customers CRUD | Store supplier contact info + track total outstanding balance owed TO them |
| **Supplier Purchases** | Invoices + `mark_paid` | Record purchase orders from suppliers with partial payment tracking |

### Key difference from customer invoices

| Aspect | Customer Invoices (existing) | Supplier Purchases (new) |
|---|---|---|
| Direction | Customer owes business | Business owes supplier |
| Balance field | `customers.outstanding_balance` (receivable) | `suppliers.outstanding_balance` (payable) |
| Stock effect | Decrements stock on creation | **No stock auto-increment** (inventory is managed via Inventory CRUD/Import — keeping it decoupled avoids double-counting risk) |
| Numbering | `INV-YYYY-NNNN` | `PO-YYYY-NNNN` (separate counter) |
| Retention | 12-month auto-purge of settled invoices | **No auto-purge** (supplier payment history is needed indefinitely for accounting) |

> [!IMPORTANT]
> **Stock is NOT auto-incremented** when recording a supplier purchase. The owner manages stock through the Inventory page. This is a deliberate decoupling decision — purchase records track money flow, not inventory state. This prevents double-counting if the owner also updates stock manually.

---

## 2. Database Schema — New Migration `006_suppliers.sql`

> [!CAUTION]
> This goes in a **new file** `src-tauri/src/database/migrations/006_suppliers.sql`. Existing migrations are **never** edited.

```sql
-- 006_suppliers.sql — Supplier ledger tables

-- 1. Suppliers (contact directory + payables balance)
CREATE TABLE IF NOT EXISTS suppliers (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    phone           TEXT,
    email           TEXT,
    company         TEXT,
    address         TEXT,
    notes           TEXT,
    outstanding_balance INTEGER NOT NULL DEFAULT 0,  -- Whole rupees (what WE owe THEM)
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_suppliers_name ON suppliers(name);
CREATE INDEX IF NOT EXISTS idx_suppliers_company ON suppliers(company);

-- 2. Supplier Purchases (purchase order master — analogous to invoices)
CREATE TABLE IF NOT EXISTS supplier_purchases (
    id              TEXT PRIMARY KEY NOT NULL,
    purchase_no     TEXT UNIQUE NOT NULL,               -- Format: PO-YYYY-NNNN
    supplier_id     TEXT NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    supplier_name   TEXT NOT NULL,                       -- Denormalized for display
    subtotal        INTEGER NOT NULL DEFAULT 0,          -- Whole rupees
    discount        INTEGER NOT NULL DEFAULT 0,          -- Whole rupees
    total           INTEGER NOT NULL DEFAULT 0,          -- Whole rupees
    amount_paid     INTEGER NOT NULL DEFAULT 0,          -- Whole rupees (cumulative)
    payment_method  TEXT NOT NULL DEFAULT 'cash'
                    CHECK (payment_method IN ('cash', 'bank', 'credit')),
    notes           TEXT,
    created_by      TEXT REFERENCES profiles(id),
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_supplier_purchases_supplier
    ON supplier_purchases(supplier_id);
CREATE INDEX IF NOT EXISTS idx_supplier_purchases_created
    ON supplier_purchases(created_at DESC);

-- 3. Supplier Purchase Items (line items — analogous to invoice_items)
CREATE TABLE IF NOT EXISTS supplier_purchase_items (
    id              TEXT PRIMARY KEY NOT NULL,
    purchase_id     TEXT NOT NULL REFERENCES supplier_purchases(id) ON DELETE CASCADE,
    description     TEXT NOT NULL,                       -- Free-text item description
    quantity        INTEGER NOT NULL DEFAULT 1,
    unit            TEXT NOT NULL DEFAULT 'pcs',
    unit_price      INTEGER NOT NULL DEFAULT 0,          -- Whole rupees
    line_total      INTEGER NOT NULL DEFAULT 0,          -- Whole rupees
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_supplier_purchase_items_purchase
    ON supplier_purchase_items(purchase_id);

-- 4. Purchase number sequence (analogous to invoice_counter)
CREATE TABLE IF NOT EXISTS purchase_counter (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    year     INTEGER NOT NULL,
    sequence INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO purchase_counter (id, year, sequence) VALUES (1, 2026, 0);
```

### Schema design decisions

| Decision | Rationale |
|---|---|
| `supplier_id` uses `ON DELETE RESTRICT` (not `SET NULL`) | Unlike customers, deleting a supplier with unpaid balances would orphan financial records. Restrict forces settling/deleting purchases first. |
| Items use `description` (free text) not `product_id` FK | Supplier purchases may include freight, services, materials not in our product catalog. Decouples from inventory. |
| Separate `purchase_counter` table | Avoids conflict with `invoice_counter`. Same singleton pattern. |
| No `delivery_date` | Purchase tracking is financial, not logistics. Can be added later in a `007_` migration if needed. |

---

## 3. Backend — Rust Implementation

### 3A. Files to Create (NEW)

| File | Purpose |
|---|---|
| `src-tauri/src/repositories/suppliers.rs` | `SupplierRepository` + `SupplierPurchaseRepository` — all SQL logic |
| `src-tauri/src/commands/suppliers.rs` | IPC command handlers (`#[tauri::command]`) |

### 3B. Files to Modify (EXISTING — minimal, additive only)

| File | Change | Risk |
|---|---|---|
| [repositories/mod.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/repositories/mod.rs) | Add `pub mod suppliers;` + re-exports | Zero — additive |
| [commands/mod.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/commands/mod.rs) | Add `pub mod suppliers;` + `pub use suppliers::*;` | Zero — additive |
| [events/emitter.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/events/emitter.rs) | Add `emit_suppliers_changed()` | Zero — additive |
| [events/mod.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/events/mod.rs) | Re-export `emit_suppliers_changed` | Zero — additive |
| [lib.rs](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src-tauri/src/lib.rs) | Register new commands in `tauri::generate_handler![...]` | Zero — additive append |

### 3C. Repository Layer — `repositories/suppliers.rs`

#### Structs

```rust
// --- Supplier CRUD ---
#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub outstanding_balance: i64,  // What WE owe THEM (whole PKR)
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateSupplierInput {
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

// --- Supplier Purchase ---
#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct SupplierPurchase {
    pub id: String,
    pub purchase_no: String,
    pub supplier_id: String,
    pub supplier_name: String,
    pub subtotal: i64,
    pub discount: i64,
    pub total: i64,
    pub amount_paid: i64,
    pub payment_method: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct SupplierPurchaseItem {
    pub id: String,
    pub purchase_id: String,
    pub description: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
    pub created_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreatePurchaseInput {
    pub supplier_id: String,
    pub supplier_name: String,
    pub subtotal: i64,
    pub discount: i64,
    pub total: i64,
    pub amount_paid: Option<i64>,
    pub payment_method: String,
    pub notes: Option<String>,
    pub items: Vec<CreatePurchaseItemInput>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreatePurchaseItemInput {
    pub description: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
}
```

#### Repository functions (mirror existing patterns exactly)

```rust
pub struct SupplierRepository { pool: SqlitePool }

impl SupplierRepository {
    pub fn new(pool: SqlitePool) -> Self;
    pub async fn list(&self) -> Result<Vec<Supplier>, AppError>;
    pub async fn get(&self, id: &str) -> Result<Option<Supplier>, AppError>;
    pub async fn create(&self, input: CreateSupplierInput) -> Result<Supplier, AppError>;
    pub async fn update(&self, id: &str, input: CreateSupplierInput) -> Result<Supplier, AppError>;
    pub async fn delete(&self, id: &str) -> Result<(), AppError>;
    // Delete guard: reject if outstanding_balance != 0
}

pub struct SupplierPurchaseRepository { pool: SqlitePool }

impl SupplierPurchaseRepository {
    pub fn new(pool: SqlitePool) -> Self;
    pub async fn list(&self, limit: i64, offset: i64, query: Option<&str>)
        -> Result<Vec<SupplierPurchase>, AppError>;
    pub async fn get(&self, id: &str) -> Result<Option<SupplierPurchase>, AppError>;
    pub async fn get_with_items(&self, id: &str)
        -> Result<Option<(SupplierPurchase, Vec<SupplierPurchaseItem>)>, AppError>;
    pub async fn create(&self, input: CreatePurchaseInput)
        -> Result<SupplierPurchase, AppError>;
    pub async fn mark_paid(&self, id: &str, amount: i64, method: &str)
        -> Result<SupplierPurchase, AppError>;
    pub async fn get_next_purchase_no(&self) -> Result<String, AppError>;
}
```

#### Payment logic in `create()` (mirrors `InvoiceRepository::create`)

```
1. Begin transaction
2. Get next purchase_no (PO-YYYY-NNNN) from purchase_counter
3. Calculate: amount_paid = input.amount_paid.unwrap_or(0).clamp(0, total)
              due = total - amount_paid
4. INSERT INTO supplier_purchases (...)
5. For each item: INSERT INTO supplier_purchase_items (...)
6. If due > 0:
     UPDATE suppliers SET outstanding_balance = outstanding_balance + due WHERE id = ?
7. Commit transaction
```

#### Payment logic in `mark_paid()` (mirrors `InvoiceRepository::mark_paid`)

```
1. Begin transaction
2. Fetch purchase, compute due_before = total - amount_paid
3. Validate: amount <= due_before (reject overpayment)
4. applied = amount.clamp(0, due_before)
5. UPDATE supplier_purchases SET amount_paid = old_paid + applied
6. UPDATE suppliers SET outstanding_balance = MAX(0, outstanding_balance - applied)
7. Commit transaction
```

### 3D. Commands Layer — `commands/suppliers.rs`

| Tauri Command Name | Auth | Emits |
|---|---|---|
| `list_suppliers` | any staff | — |
| `get_supplier` | any staff | — |
| `create_supplier` | `require_cashier_or_admin` | `suppliers:changed` |
| `update_supplier` | `require_cashier_or_admin` | `suppliers:changed` |
| `delete_supplier` | `require_cashier_or_admin` | `suppliers:changed` |
| `list_supplier_purchases` | any staff | — |
| `get_supplier_purchase` | any staff | — |
| `get_supplier_purchase_with_items` | any staff | — |
| `create_supplier_purchase` | `require_cashier_or_admin` | `suppliers:changed` |
| `mark_supplier_purchase_paid` | `require_cashier_or_admin` | `suppliers:changed` |

#### Validation in `create_supplier_purchase` (mirrors `create_invoice`)

```
- discount >= 0
- subtotal >= 0, total >= 0
- amount_paid >= 0 (if provided)
- each item: quantity >= 1, unit_price >= 0, line_total >= 0
```

### 3E. Events — `events/emitter.rs`

Add one new function:

```rust
pub async fn emit_suppliers_changed(app: &AppHandle) {
    let _ = app.emit("suppliers:changed", ());
}
```

### 3F. Command Registration — `lib.rs`

Append to `tauri::generate_handler![...]`:

```rust
// Suppliers
commands::suppliers::list_suppliers,
commands::suppliers::get_supplier,
commands::suppliers::create_supplier,
commands::suppliers::update_supplier,
commands::suppliers::delete_supplier,
commands::suppliers::list_supplier_purchases,
commands::suppliers::get_supplier_purchase,
commands::suppliers::get_supplier_purchase_with_items,
commands::suppliers::create_supplier_purchase,
commands::suppliers::mark_supplier_purchase_paid,
```

---

## 4. Frontend — TypeScript Implementation

### 4A. Files to Create (NEW)

| File | Purpose |
|---|---|
| `src/features/suppliers/api.ts` | React Query hooks + mutation hooks |
| `src/routes/_authenticated/admin.suppliers.tsx` | Suppliers page (CRUD + purchases + payments) |

### 4B. Files to Modify (EXISTING — minimal, additive only)

| File | Change | Risk |
|---|---|---|
| [api-client.ts](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src/lib/api-client.ts) | Add `api.suppliers` namespace (8-10 methods) | Zero — additive, new namespace |
| [tauri-events.ts](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src/lib/tauri-events.ts) | Add `"suppliers:changed"` to `TauriEventName` union + add `useSuppliersRealtime()` hook | Zero — additive |
| [AdminShell.tsx](file:///C:/Users/Malik%20Areeb%20Ahmed/OneDrive/Desktop/stone-flow-pos-main/src/components/admin/AdminShell.tsx) | Add nav entry `{ to: "/admin/suppliers", label: "Suppliers", icon: Truck, adminOnly: true }` | Zero — additive to `NAV[]` array |

### 4C. API Client — `api-client.ts` additions

```typescript
// New namespace within the api object
suppliers: {
  list:              ()           => apiInvoke<Supplier[]>("list_suppliers", {}, { feature: "suppliers" }),
  get:               (id: string) => apiInvoke<Supplier>("get_supplier", { id }, { feature: "suppliers" }),
  create:            (input)      => apiInvoke<Supplier>("create_supplier", { input }, { feature: "suppliers" }),
  update:            (id, input)  => apiInvoke<Supplier>("update_supplier", { input: { ...input, id } }, { feature: "suppliers" }),
  delete:            (id: string) => apiInvoke<void>("delete_supplier", { id }, { feature: "suppliers" }),
  listPurchases:     (params?)    => apiInvoke<SupplierPurchase[]>("list_supplier_purchases", { input: params ?? {} }, { feature: "suppliers" }),
  getPurchase:       (id: string) => apiInvoke<SupplierPurchase>("get_supplier_purchase", { id }, { feature: "suppliers" }),
  getPurchaseWithItems: (id)      => apiInvoke<[SupplierPurchase, SupplierPurchaseItem[]]>("get_supplier_purchase_with_items", { id }, { feature: "suppliers" })
                                       .then(([purchase, items]) => ({ purchase, items })),
  createPurchase:    (input)      => apiInvoke<SupplierPurchase>("create_supplier_purchase", { input }, { feature: "suppliers" }),
  markPurchasePaid:  (input)      => apiInvoke<SupplierPurchase>("mark_supplier_purchase_paid", { input }, { feature: "suppliers" }),
},
```

### 4D. TypeScript Types — `api-client.ts`

```typescript
interface Supplier {
  id: string;
  name: string;
  phone: string | null;
  email: string | null;
  company: string | null;
  address: string | null;
  notes: string | null;
  outstanding_balance: number;
  created_at: string;
  updated_at: string;
}

interface SupplierPurchase {
  id: string;
  purchase_no: string;
  supplier_id: string;
  supplier_name: string;
  subtotal: number;
  discount: number;
  total: number;
  amount_paid: number;
  payment_method: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

interface SupplierPurchaseItem {
  id: string;
  purchase_id: string;
  description: string;
  quantity: number;
  unit: string;
  unit_price: number;
  line_total: number;
  created_at: string;
}
```

### 4E. Feature Hooks — `features/suppliers/api.ts`

```typescript
// Query hooks
useSuppliers()                          // queryKey: ["suppliers"]
useSupplier(id)                         // queryKey: ["supplier", id]
useSupplierPurchases(params?)           // queryKey: ["supplier-purchases"]
useSupplierPurchase(id)                 // queryKey: ["supplier-purchase", id]

// Mutation hooks
useCreateSupplier()
useUpdateSupplier()
useDeleteSupplier()
useCreateSupplierPurchase()
useMarkSupplierPurchasePaid()
```

### 4F. Realtime Hook — `tauri-events.ts`

```typescript
// New hook
export function useSuppliersRealtime() {
  useEffect(() => {
    const sub = subscribeToEvent("suppliers:changed", () => {
      queryClient.invalidateQueries({ queryKey: ["suppliers"] });
      queryClient.invalidateQueries({ queryKey: ["supplier-purchases"] });
    });
    return () => sub.unlisten();
  }, []);
}
```

### 4G. Navigation Entry — `AdminShell.tsx`

```typescript
// Insert after the "Returns" entry in NAV array:
{ to: "/admin/suppliers", label: "Suppliers", icon: Truck, adminOnly: true },
```

Import: `import { Truck } from "lucide-react";` (already available in lucide-react)

### 4H. Suppliers Page — `admin.suppliers.tsx`

The page has **3 sections** arranged as tabs or stacked panels:

#### Section 1: Supplier Directory (top)
- **Table columns**: Name | Company | Phone | Email | Balance Owed | Actions
- **Add Supplier** dialog in header actions (mirrors customer dialog)
  - Fields: Name (required), Phone, Email, Company, Address, Notes
- **Edit/Delete** inline actions per row
- **Balance** column: red text if `outstanding_balance > 0`, formatted via `currency()`
- **Row click** → expands to show that supplier's purchase history (same accordion pattern as customers page)

#### Section 2: Record Purchase (middle — dialog triggered by button)
- **"Record Purchase" button** in header actions
- Dialog form:
  - Supplier dropdown (searchable, populated from `useSuppliers()`)
  - Items list (dynamic add/remove rows):
    - Description (free text), Quantity, Unit, Unit Price → auto-calculated Line Total
  - Subtotal (auto-calculated sum of line totals)
  - Discount (manual input, validated >= 0)
  - Total (auto = subtotal - discount)
  - Amount Paid (defaults to 0 for credit, total for cash)
  - Payment Method: cash / bank / credit radio
  - Notes (optional)
- On submit → `api.suppliers.createPurchase(input)` → toast success with PO number

#### Section 3: Purchase History (bottom or expanded from supplier row)
- **Table columns**: PO # | Supplier | Total | Paid | Balance | Date | Actions
- **Balance** = `total - amount_paid`, red if > 0
- **"Record Payment" action** on unpaid purchases:
  - Same UI pattern as invoice detail page payment box
  - NumberInput with min=1, max=balance, placeholder=balance
  - Button: "Clear full balance" / "Record partial payment" (dynamic label)
  - On submit → `api.suppliers.markPurchasePaid({ id, amount, payment_method })`
  - Toast: "Payment recorded — PKR X still due" or "Purchase fully paid"

---

## 5. Conflict Analysis — Zero-Impact Guarantee

| Existing System Area | Impact from Suppliers Module | Why Zero Conflict |
|---|---|---|
| **Customer invoices** | None | Completely separate tables, counters, events, query keys, API namespace |
| **Products / Inventory** | None | Supplier purchases do NOT touch `products.stock_qty` — deliberate decoupling |
| **Invoice counter** | None | Supplier uses separate `purchase_counter` table |
| **Event system** | New event added, existing untouched | `"suppliers:changed"` is a new channel; no existing listener sees it |
| **Dashboard / Reports** | None | Dashboard queries only reference `invoices`, `products`, `customers` tables. Supplier data is invisible to existing reports |
| **Returns** | None | Returns reference `invoices`/`invoice_items` only |
| **Backup / Restore** | Automatic | `VACUUM INTO` captures all tables; restore swaps the whole DB file; new tables come along for free |
| **Retention purge** | None | Retention only targets `invoices` with `amount_paid >= total`; supplier tables are not queried |
| **Receipt printing** | None | Receipts only reference invoice data |
| **PDF generation** | None | PDF only references invoice data |
| **Auth / Roles** | None | Uses existing `require_cashier_or_admin` middleware |
| **React Query cache** | None | All new query keys (`["suppliers"]`, `["supplier-purchases"]`) are unique |
| **TanStack Router** | None | New route file auto-discovered by file-based routing |
| **Migration checksums** | None | New `006_` file; existing migrations untouched |

---

## 6. Implementation Order (Recommended)

### Phase 1: Database (5 min)
1. Create `src-tauri/src/database/migrations/006_suppliers.sql`

### Phase 2: Backend core (30 min)
2. Create `src-tauri/src/repositories/suppliers.rs` (structs + all repo functions)
3. Update `src-tauri/src/repositories/mod.rs` (add module + re-exports)
4. Create `src-tauri/src/commands/suppliers.rs` (all 10 IPC commands)
5. Update `src-tauri/src/commands/mod.rs` (add module + re-exports)
6. Update `src-tauri/src/events/emitter.rs` (add `emit_suppliers_changed`)
7. Update `src-tauri/src/events/mod.rs` (re-export)
8. Update `src-tauri/src/lib.rs` (register 10 commands in handler)

### Phase 3: Frontend wiring (20 min)
9. Add TypeScript types + `api.suppliers` namespace to `api-client.ts`
10. Add `"suppliers:changed"` event + `useSuppliersRealtime()` to `tauri-events.ts`
11. Create `src/features/suppliers/api.ts` (query + mutation hooks)
12. Add nav entry to `AdminShell.tsx`

### Phase 4: UI page (40 min)
13. Create `src/routes/_authenticated/admin.suppliers.tsx` (full page with all 3 sections)

### Phase 5: Verification (15 min)
14. `cargo check --all-targets` — 0 new errors
15. `npx tsc --noEmit` — 0 new errors vs baseline
16. `npm run build` — clean
17. `cargo tauri dev` — manual smoke test:
    - Add supplier → appears in list
    - Record purchase → PO number generated, supplier balance increases
    - Record partial payment → balance decreases correctly
    - Record full payment → balance goes to 0
    - Delete supplier with balance → blocked
    - Delete supplier with 0 balance → succeeds
    - Verify existing flows (login, POS, invoices, customers, returns) are unaffected

---

## 7. Future Extensions (NOT in scope now)

These can be added in later migrations/sessions without touching the core:

- **Link purchases to products** for auto stock-in (would need a `product_id` FK on `supplier_purchase_items` + stock increment in the `create` transaction)
- **Supplier reports** on the Reports page (total payables, payment history by period)
- **Purchase receipt printing** (reuse `receipt_bitmap.rs` with a purchase variant)
- **Dashboard integration** (total payables card, recent purchases widget)
- **Purchase search** (same pattern as invoice full-history search)

---

## 8. Files Changed Summary

### New Files (6)
| # | File | Layer |
|---|---|---|
| 1 | `src-tauri/src/database/migrations/006_suppliers.sql` | Database |
| 2 | `src-tauri/src/repositories/suppliers.rs` | Backend |
| 3 | `src-tauri/src/commands/suppliers.rs` | Backend |
| 4 | `src/features/suppliers/api.ts` | Frontend |
| 5 | `src/routes/_authenticated/admin.suppliers.tsx` | Frontend |
| 6 | *(routeTree.gen.ts — auto-regenerated by TanStack Router)* | Auto |

### Modified Files (6, all additive-only)
| # | File | Change Size |
|---|---|---|
| 1 | `src-tauri/src/repositories/mod.rs` | +2 lines |
| 2 | `src-tauri/src/commands/mod.rs` | +2 lines |
| 3 | `src-tauri/src/events/emitter.rs` | +4 lines |
| 4 | `src-tauri/src/events/mod.rs` | +1 re-export |
| 5 | `src-tauri/src/lib.rs` | +10 lines in handler array |
| 6 | `src/lib/api-client.ts` | +30 lines (types + namespace) |
| 7 | `src/lib/tauri-events.ts` | +10 lines (event + hook) |
| 8 | `src/components/admin/AdminShell.tsx` | +1 nav entry + 1 import |

> **Total existing-file modifications**: ~50 lines across 8 files, all purely additive. Zero line deletions. Zero behavioral changes to existing code paths.
