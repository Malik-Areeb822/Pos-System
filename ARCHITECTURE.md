# Stone Flow POS — Architecture Reference

> Generated from codebase analysis. Keep this updated as the project evolves.

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | TanStack Start (React 19 + SSR) |
| Router | TanStack Router (file-based, type-safe) |
| Data Fetching | TanStack Query (React Query v5) |
| Database | Supabase (PostgreSQL) |
| Auth | Supabase Auth + custom RBAC |
| UI | Radix UI primitives + Tailwind CSS 4 + shadcn/ui patterns |
| Real-time | Supabase Realtime (Postgres changes) |
| Forms | React Hook Form + Zod |
| Charts | Recharts |
| Export | XLSX (SheetJS) |
| Icons | Lucide React |
| Build | Vite 6 + Nitro (server) |
| Lint/Format | ESLint 9 + Prettier 3 |

---

## Route Map

```
/
├── (public) Staff login — sign in / sign up / first-admin setup
│
├── /website/* (TABLED — public marketing site)
│   ├── /website                  → Home (hero, trust, categories, factory, CTA)
│   ├── /website/catalog          → Filterable product grid
│   ├── /website/product/$id      → Product detail (not implemented)
│   ├── /website/about            → Our process/story
│   ├── /website/trade            → Bulk/trade inquiry form
│   └── /website/contact          → Location, phone, WhatsApp, map, inquiry
│
└── /_authenticated/* (PROTECTED — requires approved staff account)
    └── /admin/                     → AdminShell layout + nav
        ├── /                       → Dashboard (admin only; cashiers → /pos)
        ├── /pos                    → Point of Sale (staff)
        ├── /inventory              → Inventory CRUD + import/export
        ├── /customers              → Customer CRM + order history
        ├── /invoices               → Invoice list + print / mark paid
        │   └── /$invoiceId         → Invoice detail (print view)
        ├── /returns                → Returns processing (stock restore)
        ├── /reports                → Sales reports + Excel (admin only)
        └── /cashiers               → Cashier approval/management (admin only)
```

---

## Database Schema (Supabase)

### Enums
```sql
app_role          : 'admin' | 'cashier'
product_category  : 'marble' | 'tiles' | 'chips' | 'sanitary'
payment_method    : 'cash' | 'bank' | 'credit'
account_status    : 'pending' | 'approved' | 'rejected' | 'suspended'
```

### Tables

| Table | Key Columns | RLS Policy Summary |
|-------|-------------|---------------------|
| `profiles` | `id` (FK→auth.users), `full_name`, `email`, `phone`, `employee_id`, `status`, `approved_at`, `rejected_at` | Admin reads all; users read own; status changes only via RPC |
| `user_roles` | `user_id`, `role` (unique per user+role) | Admin manages; users read own; trigger enforces single admin |
| `products` | `id`, `name`, `sku` (unique), `category`, `color`, `size`, `finish`, `unit`, `price`, `stock_qty`, `low_stock_threshold`, `pieces_per_carton`, `image_url`, `is_published` | Public reads published; staff reads all; staff writes; admin deletes |
| `customers` | `id`, `name`, `phone`, `email`, `address`, `outstanding_balance` | Staff CRUD; admin deletes |
| `invoices` | `id`, `invoice_no` (BMT-SEQ), `customer_id`, `customer_name`, `subtotal`, `discount`, `total`, `amount_paid`, `payment_method`, `notes`, `delivery_date`, `created_by` | Staff read; staff insert (own); staff update; admin delete |
| `invoice_items` | `id`, `invoice_id`, `product_id`, `product_name`, `quantity`, `unit`, `unit_price`, `line_total` | Staff read/insert; admin delete |
| `returns` | `id`, `invoice_id`, `invoice_no`, `customer_id`, `customer_name`, `total`, `reason`, `created_by` | Staff read/insert |
| `return_items` | `id`, `return_id`, `product_id`, `product_name`, `quantity`, `unit`, `unit_price`, `line_total` | Staff read/insert |
| `inquiries` | `id`, `name`, `phone`, `email`, `company`, `inquiry_type`, `message` | Anon insert; staff read |

### RPC Functions

| Function | Purpose | Security |
|----------|---------|----------|
| `admin_exists()` | Returns `true` if admin role exists | `EXECUTE` granted to `anon`, `authenticated` |
| `handle_new_user()` | Trigger on `auth.users` insert: first user → approved admin; rest → pending cashier | `SECURITY DEFINER` |
| `set_account_status(uid, status)` | Admin-only: approve/reject/suspend/reactivate cashier; updates timestamps & auditors | `SECURITY DEFINER`; checks admin role + self-protection |
| `process_return(invoice_id, items_json, reason)` | Atomic: creates return, restores stock, adjusts invoice total, updates customer balance | `SECURITY DEFINER` |
| `private.has_role(uid, role)` | Returns true if user has role AND status=approved | `SECURITY DEFINER`; used in RLS |
| `private.is_staff(uid)` | Returns true if user has admin/cashier role AND status=approved | `SECURITY DEFINER`; used in RLS |

---

## Auth & Authorization Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. User visits "/"                                          │
│    → supabase.auth.getSession()                             │
│    → if session exists: navigate("/admin", {replace: true}) │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. Sign Up (supabase.auth.signUp)                           │
│    → handle_new_user() trigger fires:                       │
│       - INSERT profiles (status: admin→approved, else→pending) │
│       - INSERT user_roles (role: admin→admin, else→cashier) │
│    → If admin already exists: toast "wait for admin approval" │
│    → If first user: auto-sign-in → "/admin"                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. AccessGate wraps all /_authenticated routes              │
│    → useMyProfile() fetches profiles.status                 │
│    │                                                         │
│    ├── pending   → "Awaiting admin approval"                │
│    ├── rejected  → "Registration rejected"                  │
│    ├── suspended → "Account suspended"                      │
│    └── approved  → render children (AdminShell)             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. Role-based UI                                            │
│    → AdminShell.nav filters by isAdmin                      │
│    ├── adminOnly: Dashboard, Reports, Cashiers              │
│    └── staff: POS, Inventory, Customers, Invoices, Returns  │
│    → AdminOnly component blocks cashiers from admin routes  │
│    → DashboardGate redirects cashiers from "/" → "/pos"     │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Features Detail

### Point of Sale (`/admin/pos`)
- **Barcode/Code scanner**: Exact SKU match → auto-add; fuzzy fallback shows dropdown
- **Tile carton logic**: `pieces_per_carton` > 0 → shows "Cartons" + "Extra tiles" inputs
- **Walk-in customer**: Inline create (name, phone, save flag) → creates customer record
- **Payment**: cash / bank / credit; partial payment tracked via `amount_paid`
- **Delivery date**: Optional future date for marble/tile orders
- **Checkout mutation** (atomic via Supabase):
  1. Insert invoice
  2. Insert invoice_items
  3. Decrement product stock_qty
  4. Update customer outstanding_balance (if credit)
  5. Navigate to invoice detail + invalidate queries

### Inventory (`/admin/inventory`)
- **Dialog form**: Add/Edit with category-specific fields (tiles → pieces_per_carton)
- **Excel Import**: Flexible column mapping (case-insensitive, aliases)
- **Excel Export**: Fixed column order with headers
- **Tabs**: All / Marble / Tiles / Chips / Sanitary
- **Search**: name, sku, color, size, unit, description
- **Low stock highlight**: Red when `stock_qty <= low_stock_threshold`
- **Permissions**: Admin = full CRUD; Cashier = read-only

### Real-time Sync (`useInvoiceRealtime`)
```typescript
// Subscribes to postgres_changes on 'invoices' + 'customers'
// On any change: invalidates ['invoices'], ['customers'], ['dashboard']
// Optional invoiceId: also invalidates ['invoice', invoiceId]
```
Used in: Invoices list, Invoice detail, Reports, Dashboard

### Reports (`/admin/reports`)
- **Period cards**: Today / 7d / Month / 6mo / Year → each with Excel export
- **Monthly breakdown**: Last 12 months table with visual bar (sales share)
- **Export all**: Full invoice dump with totals row

### Returns (`/admin/returns`)
- Search invoice → load items + already-returned quantities
- Enter return qty per item (max = remaining)
- Reason (optional)
- `process_return` RPC: creates return record, restores stock, reduces invoice total, adjusts customer balance

---

## Component Architecture

```
src/
├── components/
│   ├── admin/
│   │   ├── AdminShell.tsx       # Layout: sidebar nav + header + sign out
│   │   ├── AdminOnly.tsx        # Wrapper: blocks non-admin
│   │   └── AccessGate.tsx       # Wrapper: checks profile.status
│   ├── ui/                      # 40+ Radix-based primitives (Button, Dialog, Table, etc.)
│   └── website/
│       ├── SiteShell.tsx        # Public layout: header, footer, nav, Wordmark
│       ├── ProductCard.tsx      # Catalog product card
│       └── InquiryForm.tsx      # Contact/trade inquiry form
├── hooks/
│   └── useInvoiceRealtime.ts    # Realtime subscription hook
├── lib/
│   ├── pos.ts                   # Types + Supabase queries + auth hooks
│   ├── catalog.ts               # Product types, categories, currency formatter, fetchProducts
│   └── business.ts              # Business constants (name, phone, address, hours)
├── integrations/supabase/
│   ├── client.ts                # Lazy singleton Supabase client (client + SSR)
│   ├── client.server.ts         # Server-only client
│   ├── types.ts                 # Generated Database types (562 lines)
│   ├── auth-middleware.ts       # SSR auth helpers
│   └── auth-attacher.ts         # Cookie/session attachment
├── routes/
│   ├── __root.tsx               # Root layout: QueryClientProvider, Toaster, Error/NotFound
│   ├── index.tsx                # Login page
│   ├── _authenticated/
│   │   ├── route.tsx            # Auth guard + AccessGate
│   │   ├── admin.index.tsx      # Dashboard (stats, recent invoices, low stock)
│   │   ├── admin.pos.tsx        # POS screen (517 lines)
│   │   ├── admin.inventory.tsx  # Inventory (480 lines)
│   │   ├── admin.customers.tsx  # Customers (159 lines)
│   │   ├── admin.invoices.index.tsx
│   │   ├── admin.invoices.$invoiceId.tsx (MISSING — print view)
│   │   ├── admin.returns.tsx    # Returns (257 lines)
│   │   ├── admin.reports.tsx    # Reports (222 lines)
│   │   └── admin.cashiers.tsx   # Cashier mgmt (155 lines)
│   └── website/                 # Public site (tabled)
├── server.ts                    # SSR entry: error capture + h3 swallowed error normalization
└── router.tsx                   # Router factory with QueryClient context
```

---

## Key Business Rules (Enforced at DB Level)

1. **Single Admin**: `enforce_single_admin()` trigger on `user_roles` insert/update
2. **Approved Staff Only**: All RLS policies use `private.is_staff(uid)` which requires `profiles.status = 'approved'`
3. **Self-Protection**: `protect_profile_status()` trigger prevents users changing their own status
4. **Invoice Ownership**: `created_by = auth.uid()` enforced on insert
5. **Stock Atomicity**: Decrement on checkout, restore on return — both via direct SQL in mutations/RPC
6. **Outstanding Balance**: Auto-updated on invoice create (credit) and payment cleared / return

---

## Environment Variables

| Variable | Required | Context |
|----------|----------|---------|
| `VITE_SUPABASE_URL` | Yes | Client + SSR |
| `VITE_SUPABASE_PUBLISHABLE_KEY` | Yes | Client + SSR |
| `SUPABASE_URL` | Fallback | SSR only |
| `SUPABASE_PUBLISHABLE_KEY` | Fallback | SSR only |

---

## Commands

```bash
npm run dev       # Start dev server (Vite + Nitro)
npm run build     # Production build
npm run build:dev # Dev-mode build
npm run preview   # Preview production build
npm run lint      # ESLint
npm run format    # Prettier --write
```

---

## Missing / Incomplete

- `/admin/invoices/$invoiceId.tsx` — Print view for invoices (referenced in invoices list but file not found)
- `/website/product/$productId.tsx` — Product detail page (route defined in router but component missing)
- Website section marked "TABLED" — not wired into main nav from login

---

## Supabase Project

- **Project ID**: `zsbxrkyvhfgorrcfvymk`
- **Realtime publication**: `invoices`, `customers` (added via migration `20260808202142`)

---

## Data Flow: POS Checkout

```
User Action: "Complete sale"
    │
    ▼
checkout.mutate()
    │
    ├─► Find/create customer (walk-in + save flag)
    │
    ├─► INSERT invoices {
    │       customer_id, customer_name,
    │       subtotal, discount, total, amount_paid,
    │       payment_method, notes, delivery_date, created_by
    │   }
    │
    ├─► INSERT invoice_items[] {
    │       invoice_id, product_id, product_name,
    │       quantity, unit, unit_price, line_total
    │   }
    │
    ├─► UPDATE products SET stock_qty = stock_qty - qty WHERE id = product_id
    │
    ├─► IF credit (amount_paid < total):
    │       UPDATE customers SET outstanding_balance += (total - amount_paid)
    │
    ▼
Returns invoice_id → navigate("/admin/invoices/$invoiceId")
    │
    ▼
queryClient.invalidateQueries() → useInvoiceRealtime pushes to all sessions
```

---

## File: `src/lib/pos.ts` — Key Exports

```typescript
// Types
type Role = 'admin' | 'cashier'
type AccountStatus = 'pending' | 'approved' | 'rejected' | 'suspended'
type StaffProfile = { id, full_name, email, phone, employee_id, status, created_at, approved_at, rejected_at }
type PaymentMethod = 'cash' | 'bank' | 'credit'
type Customer = { id, name, phone, email, address, outstanding_balance, created_at }
type Invoice = { id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, payment_method, notes, delivery_date, created_at }
type InvoiceItem = { id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total }

// Hooks
useSession()           → current auth user
useRoles()             → string[] of roles (requires approved profile)
useMyProfile()         → StaffProfile | null
useIsAdmin()           → boolean

// Queries
fetchStaff()           → StaffProfile[]
fetchCustomers()       → Customer[]
fetchInvoices()        → Invoice[]
formatDate(iso)        → "DD MMM YYYY"

// Mutations
setAccountStatus(uid, status)  → RPC call
markInvoicePaid(invoice)       → updates amount_paid + customer balance
```

---

## File: `src/lib/catalog.ts` — Key Exports

```typescript
type Category = 'marble' | 'tiles' | 'chips' | 'sanitary'
type Product = { id, name, sku, category, description, color, size, finish, unit, price, pieces_per_carton, stock_qty, low_stock_threshold, image_url, is_published }

CATEGORIES: { key, label, blurb, image }[]  // 4 categories with marketing copy

currency(value)        → "PKR 1,234" (Intl.NumberFormat en-PK, 0 decimals)
productImage(product)  → product.image_url || category fallback image
fetchProducts(category?) → Product[]
fetchProduct(id)       → Product | null
```

---

## Styling Notes

- **CSS Variables** (from `styles.css` + Tailwind):
  - `--marble-black` — deep background
  - `--brass` — accent gold
  - `--ivory` — light text on dark
  - `--stone-surface` — subtle texture background
- **Fonts**: Cormorant Garamond (display) + Karla (body) via Google Fonts preconnect
- **Admin UI**: Clean, data-dense, functional (`bg-muted/40`, `bg-card`, `border-border`)
- **Website UI**: Premium, spacious, editorial (`bg-marble-black`, `text-ivory`, large imagery)

---

*Last updated: 2026-08-19*