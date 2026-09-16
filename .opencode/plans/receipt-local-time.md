# Plan: Show Local Time (PKT) on Receipts and PDF Invoices

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving on.
> If anything in "STOP conditions" occurs, stop and write a handback —
> do not improvise.
>
> **Drift check (run first)**: `git diff -- src-tauri/src/services/receipt_bitmap.rs src-tauri/src/services/print.rs src-tauri/src/services/invoice_pdf.rs`
> If in-scope files have changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Planned at**: revision `HEAD`, 2026-09-06

## Why this matters

All timestamps in the database are stored as UTC (`chrono::Utc::now().to_rfc3339()`). The
receipt and PDF rendering functions parse the UTC timestamp and format it directly without
converting to local time. Since the business operates in Pakistan (UTC+5), receipts show
a time 5 hours behind the actual sale time. This confuses customers and staff.

## Current state

Three files display the invoice `created_at` timestamp:

1. **`src-tauri/src/services/receipt_bitmap.rs:277-280`** — raster receipt (80mm thermal):
   ```rust
   fn timestamp(created_at: &str) -> String {
       chrono::DateTime::parse_from_rfc3339(created_at)
           .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
           .unwrap_or_else(|_| created_at.split('T').next().unwrap_or(created_at).to_string())
   }
   ```
   Called at line 309: `draw_meta_row(&mut c, fonts, "Date", &timestamp(&inv.created_at), 170.0);`

2. **`src-tauri/src/services/print.rs:343-358`** — text-mode ESC/POS fallback receipt:
   ```rust
   fn format_timestamp(created_at: &str) -> String {
       let parsed = chrono::DateTime::parse_from_rfc3339(created_at)
           .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
           .or_else(|_| { ... naive fallbacks ... });
       match parsed {
           Ok(ts) => ts,
           Err(_) => created_at.split('T').next().unwrap_or(created_at).to_string(),
       }
   }
   ```
   Called at lines 389 and 459.

3. **`src-tauri/src/services/invoice_pdf.rs:77`** — A4 PDF invoice:
   ```rust
   doc.push(elements::Paragraph::new(format!("Date: {}", inv.created_at.split('T').next().unwrap_or(""))));
   ```
   This one doesn't even show the time — only the date portion.

**Convention**: `chrono` is already a dependency with `serde` feature. `chrono::Local` is
available by default. The `Utc` type is imported in most repository files but NOT in the
three service files above (they use `chrono::` paths inline).

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Rust check | `cd src-tauri && cargo check` | 0 errors (40 pre-existing warnings ok) |
| Rust tests | `cd src-tauri && cargo test --lib` | all pass |

## Scope

**In scope**:
- `src-tauri/src/services/receipt_bitmap.rs` — `timestamp()` function
- `src-tauri/src/services/print.rs` — `format_timestamp()` function + its test
- `src-tauri/src/services/invoice_pdf.rs` — date display line

**Out of scope**:
- `src-tauri/src/repositories/*.rs` — DB storage stays UTC (immutable migration rule)
- `src-tauri/src/services/retention.rs` — retention cutoff stays UTC (it compares against stored UTC timestamps)
- `src-tauri/src/commands/reports.rs` — report date filters stay UTC (they filter stored data)
- Frontend display of dates — not printed, not in scope

## Steps

### Step 1: Fix `receipt_bitmap.rs` — raster receipt shows local time

In `src-tauri/src/services/receipt_bitmap.rs`, modify the `timestamp()` function (line 277)
to convert the parsed UTC timestamp to local time before formatting.

Change the `.map()` chain from:
```rust
.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
```
to:
```rust
.map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
```

The `unwrap_or_else` fallback (for non-RFC3339 strings) stays unchanged — those are
already-local or date-only strings.

**Verify**: `cd src-tauri && cargo check` → 0 errors

### Step 2: Fix `print.rs` — text-mode receipt shows local time

In `src-tauri/src/services/print.rs`, modify the `format_timestamp()` function (line 343).

In the first `.map()` branch (RFC3339 parse, line 344-345), change:
```rust
.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
```
to:
```rust
.map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
```

The two `NaiveDateTime` fallback branches (lines 347-352) should stay unchanged — naive
datetimes have no timezone info to convert from; they're already the intended display value.

**Verify**: `cd src-tauri && cargo check` → 0 errors

### Step 3: Fix `invoice_pdf.rs` — A4 PDF shows local date+time

In `src-tauri/src/services/invoice_pdf.rs`, modify line 77 from:
```rust
doc.push(elements::Paragraph::new(format!("Date: {}", inv.created_at.split('T').next().unwrap_or(""))));
```
to a proper parse-and-convert, matching the receipt pattern:
```rust
let date_str = chrono::DateTime::parse_from_rfc3339(&inv.created_at)
    .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
    .unwrap_or_else(|_| inv.created_at.split('T').next().unwrap_or("").to_string());
doc.push(elements::Paragraph::new(format!("Date: {}", date_str)));
```

**Verify**: `cd src-tauri && cargo check` → 0 errors

### Step 4: Update the test in `print.rs`

The existing test at line 576 asserts UTC output:
```rust
assert_eq!(format_timestamp("2026-08-23T14:35:00.123456+00:00"), "2026-08-23 14:35");
```

After the fix, this will produce `2026-08-23 19:35` (UTC+5) when run on a machine in
PKT timezone. Update the test to account for local conversion. The simplest approach:
compute the expected local string at test time:

```rust
let expected = chrono::DateTime::parse_from_rfc3339("2026-08-23T14:35:00.123456+00:00")
    .unwrap()
    .with_timezone(&chrono::Local)
    .format("%Y-%m-%d %H:%M")
    .to_string();
assert_eq!(format_timestamp("2026-08-23T14:35:00.123456+00:00"), expected);
```

The two other assertions (naive string `"2026-08-23T14:35:00"` and bare date `"2026-08-23"`)
stay unchanged — those go through fallback paths.

**Verify**: `cd src-tauri && cargo test --lib` → all pass

## Test plan

- Existing `timestamp_parses_iso_and_keeps_fallback` test in `print.rs` — update per Step 4.
- Existing `full_receipt_renders_to_reasonable_bitmap` test in `receipt_bitmap.rs` — uses
  `created_at: "2026-08-23T14:35:00+00:00"` which will now render as local time. The test
  doesn't assert on the date string content (it checks bitmap dimensions), so it should
  still pass without changes.
- No new test files needed — the conversion is a one-line change in each function.

**Verify**: `cd src-tauri && cargo test --lib` → all pass

## Done criteria

Machine-checkable. ALL must hold:

- [ ] `cd src-tauri && cargo check` → 0 errors
- [ ] `cd src-tauri && cargo test --lib` → all pass
- [ ] No files outside the in-scope list are modified
- [ ] `receipt_bitmap.rs:timestamp()` converts RFC3339 timestamps to `chrono::Local` before formatting
- [ ] `print.rs:format_timestamp()` converts RFC3339 timestamps to `chrono::Local` before formatting
- [ ] `invoice_pdf.rs:77` parses and converts to local time instead of raw string split

## STOP conditions

Stop if:

- The code at the "Current state" locations doesn't match the excerpts.
- A step's verification fails twice after a reasonable fix attempt.
- The work appears to require touching an out-of-scope file.
- `chrono::Local` is not available (it is — default chrono feature set includes it).

On stopping, write a **handback** for the planning agent instead of improvising.

## Maintenance notes

- Database timestamps remain UTC — this is intentional. The conversion happens only at
  display time. If the app is ever deployed to a different timezone, the receipts will
  automatically show that timezone's time (since `chrono::Local` reads the OS timezone).
- The `retention.rs` and `reports.rs` files use `Utc::now()` for filtering stored data —
  these must stay UTC because they compare against UTC-stored `created_at` columns.
- The `NaiveDateTime` fallback branches in `format_timestamp()` are untouched because
  naive timestamps have no offset to convert from. These are legacy edge cases.
