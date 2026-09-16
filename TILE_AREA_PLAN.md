# Tile Area Calculation — Implementation Plan

> **Goal**: Display total area on bills/receipts for tile products.
> **Example**: 10 cartons × 8 tiles × 0.32 sqm/tile = 25.6 sqm shown on receipt.
> **Pricing**: Price remains per tile (piece). Area is for display only.
> **Status**: APPROVED — awaiting green light to execute.
> **Date**: 2026-09-16

---

## 1. Database Migration

**New file: `src-tauri/src/database/migrations/008_tile_area.sql`**

```sql
ALTER TABLE products ADD COLUMN area_per_tile INTEGER;
ALTER TABLE invoice_items ADD COLUMN total_area INTEGER;
```

- `area_per_tile`: area of a single tile in sqm (stored as integer, e.g., 3200 = 0.32 sqm × 10000 for precision)
- `total_area`: computed at invoice creation = `area_per_tile × quantity`, stored for historical accuracy
- Both nullable — existing rows get NULL, zero impact on current data
- Use INTEGER (whole rupees precision pattern) to match existing money fields

---

## 2. Rust Backend — Products

### `src-tauri/src/repositories/products.rs`

- Add `pub area_per_tile: Option<i64>` to `Product` struct (line ~21)
- Add `pub area_per_tile: Option<i64>` to `CreateProductInput` struct (line ~42)
- Add `area_per_tile` to SELECT queries (list at line ~61, get at line ~71)
- Add `area_per_tile` to INSERT query (line ~85) + bind (line ~88)
- Add `area_per_tile = ?` to UPDATE query (line ~102) + bind (line ~104)

### `src-tauri/src/commands/products.rs`

- Add `pub area_per_tile: Option<i64>` to `CreateProductInputCmd` (line ~26)
- Add `pub area_per_tile: Option<i64>` to `UpdateProductInput` (line ~46)
- Add mapping in `create_product` (line ~83): `area_per_tile: input.area_per_tile`
- Add mapping in `update_product` (line ~110): `area_per_tile: input.area_per_tile`

---

## 3. Rust Backend — Invoices

### `src-tauri/src/repositories/invoices.rs`

- Add `pub total_area: Option<i64>` to `InvoiceItem` struct (line ~25)
- Add `pub total_area: Option<i64>` to `CreateInvoiceItemInput` struct (line ~52)
- Add `total_area` to SELECT in `get_items` (line ~131)
- Add `total_area` to INSERT in `create` (line ~201) + bind

### `src-tauri/src/commands/invoices.rs`

- Add `pub total_area: Option<i64>` to `CreateInvoiceItemInputCmd` (line ~32)
- Add mapping in `create_invoice` (line ~115-122): `total_area: item.total_area`

---

## 4. Rust Services — Receipt & PDF Display

### `src-tauri/src/services/receipt_bitmap.rs`

- In `render_receipt` line item loop (line ~318-330):
  - After the `@ PKR {unit_price} / {unit}` sub-line
  - If `item.total_area.is_some()`: draw a third line `{total_area} sqm` indented at x=56
- Update test `sample_invoice()` (line ~468): add `total_area: None`

### `src-tauri/src/services/invoice_pdf.rs`

- In `build_invoice_content` line item loop (line ~86-97):
  - If `item.total_area.is_some()`: append ` [{total_area} sqm]` to item text

### `src-tauri/src/services/inventory.rs`

- Add `area_per_tile` CSV column parser (after index 9, before stock_qty)
- Shift subsequent column indexes by 1
- Add `area_per_tile` to `CreateProductInput` construction (line ~136)

---

## 5. TypeScript Types (4 files)

### `src/lib/api-client.ts`
- Add `area_per_tile: number | null` to `Product` interface (line ~218)
- Add `area_per_tile?: number | null` to `CreateProductInput` interface (line ~236)
- Add `total_area: number | null` to `InvoiceItem` interface (line ~280)
- Add `total_area?: number | null` to `CreateInvoiceItemInput` interface (line ~304)

### `src/features/pos/api.ts`
- Add `area_per_tile: number | null` to `Product` type (line ~16)

### `src/features/inventory/api.ts`
- Add `area_per_tile: number | null` to `Product` type (line ~18)
- Add `area_per_tile?: number | null` to `CreateProductInput` type (line ~36)

### `src/lib/catalog.ts`
- Add `area_per_tile: number | null` to `Product` type (line ~19)

### `src/features/invoices/api.ts`
- Add `total_area: number | null` to `InvoiceItem` type (line ~21)
- Add `total_area?: number | null` to `CreateInvoiceItemInput` type (line ~45)

### `src/lib/pos.ts`
- Add `total_area: number | null` to `InvoiceItem` type (line ~44)

---

## 6. Frontend — Product Form

### `src/routes/_authenticated/admin.inventory.tsx`

- Add `area_per_tile: string` to `Draft` type (line ~49)
- Add `area_per_tile: ""` to `emptyDraft` (line ~64)
- Add to `toDraft()`: `area_per_tile: p.area_per_tile ? String(p.area_per_tile) : ""` (line ~80)
- Add to save payload (line ~130): `area_per_tile: draft.category === "tiles" && Number(draft.area_per_tile) > 0 ? Number(draft.area_per_tile) : null`
- Add form field (after `pieces_per_carton`, line ~402):
  ```tsx
  {draft.category === "tiles" && (
    <div className="space-y-2">
      <Label>Area per tile (sqm)</Label>
      <NumberInput min={0} step={0.01} {...field("area_per_tile")} />
      <p className="text-xs text-muted-foreground">
        Used on bills to show total area (e.g. 0.32 for a 60×60cm tile).
      </p>
    </div>
  )}
  ```
- Add to Excel import `pick()` (line ~186): `pick(row, "Area per tile", "area_per_tile")`
- Add to import payload (line ~199): `area_per_tile`
- Add to export mapping (line ~237): `"Area per tile": p.area_per_tile ? Number(p.area_per_tile) : ""`

---

## 7. Frontend — POS Checkout

### `src/routes/_authenticated/admin.pos.tsx`

- In checkout item mapping (line ~165-172), add `total_area`:
  ```typescript
  total_area: l.product.area_per_tile && l.product.area_per_tile > 0
    ? l.product.area_per_tile * l.qty
    : null,
  ```

---

## 8. Frontend — Invoice Detail Page

### `src/routes/_authenticated/admin.invoices.$invoiceId.tsx`

- Add optional "Area" column to items table (line ~147-168):
  - Only render if any item has `total_area`
  - Header: `<th className="py-2 text-right">Area</th>`
  - Cell: `{item.total_area ? `${item.total_area} sqm` : "—"}`

---

## 9. sqlx Cache Regeneration

```bash
cd src-tauri && cargo sqlx prepare
```

Regenerates `.sqlx/query-*.json` files to match new schema.

---

## 10. Architecture Doc Update

### `PROJECT_ARCHITECTURE.md`

- Add `area_per_tile INTEGER` to products schema (line ~174)
- Add `total_area INTEGER` to invoice_items schema (line ~213)
- Add area workflow to Section 4 (POS checkout area calculation)

---

## Execution Order

1. Create migration file `008_tile_area.sql`
2. Update Rust structs + SQL in `repositories/products.rs`
3. Update Rust structs + SQL in `repositories/invoices.rs`
4. Update command structs + mappings in `commands/products.rs`
5. Update command structs + mappings in `commands/invoices.rs`
6. Update receipt display in `receipt_bitmap.rs` + fix test
7. Update PDF display in `invoice_pdf.rs`
8. Update CSV importer in `inventory.rs` (services)
9. Run `cargo sqlx prepare` + `cargo check`
10. Update all 6 TypeScript type files
11. Update product form in `admin.inventory.tsx`
12. Update POS checkout in `admin.pos.tsx`
13. Update invoice detail in `admin.invoices.$invoiceId.tsx`
14. Run `npx tsc --noEmit` + `npm run build`
15. Update `PROJECT_ARCHITECTURE.md`

---

## Verification

- [ ] `cargo check --all-targets` — 0 errors
- [ ] `npx tsc --noEmit` — no new errors
- [ ] `npm run build` — clean
- [ ] Add a tile product with `area_per_tile = 0.32` and `pieces_per_carton = 8`
- [ ] Create invoice: buy 10 cartons → qty = 80 tiles
- [ ] Receipt shows: `80 pcs TileName` + `@ PKR X / pcs` + `25.6 sqm`
- [ ] PDF shows: `80 pcs TileName @ PKR X = PKR Y [25.6 sqm]`
- [ ] Invoice detail page shows area column
- [ ] Non-tile products show no area (NULL fields)
- [ ] Excel export includes "Area per tile" column
- [ ] Excel import round-trips area_per_tile correctly
