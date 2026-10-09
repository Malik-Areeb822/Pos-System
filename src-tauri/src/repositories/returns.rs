// src-tauri/src/repositories/returns.rs
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::Utc;
use crate::error::AppError;

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Return {
    pub id: String,
    pub invoice_id: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
    pub reason: String,
    pub processed_by: String,
    pub created_at: String,
}

/// One returned line. `line_total` is deliberately absent: the server
/// computes `quantity * unit_price`, so a tampered client value can never
/// reach the invoice ledger or the profit queries.
#[derive(Debug, serde::Deserialize)]
pub struct CreateReturnLineInput {
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
}

/// One return submission: a single invoice, a single reason, N lines.
/// Everything is applied inside ONE transaction — all-or-nothing.
#[derive(Debug, serde::Deserialize)]
pub struct CreateReturnsInput {
    pub invoice_id: String,
    pub reason: String,
    pub processed_by: String,
    pub lines: Vec<CreateReturnLineInput>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateReturnInput {
    pub invoice_id: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub reason: String,
    pub processed_by: String,
}

pub struct ReturnRepository {
    pool: SqlitePool,
}

impl ReturnRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, invoice_id: Option<String>) -> Result<Vec<Return>, AppError> {
        let returns = if let Some(inv_id) = invoice_id {
            sqlx::query_as_unchecked!(
                Return,
                r#"SELECT id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, reason, processed_by, created_at FROM returns WHERE invoice_id = ? ORDER BY created_at DESC"#,
                inv_id
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as_unchecked!(
                Return,
                r#"SELECT id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, reason, processed_by, created_at FROM returns ORDER BY created_at DESC"#
            )
            .fetch_all(&self.pool)
            .await?
        };
        Ok(returns)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Return>, AppError> {
        let ret = sqlx::query_as_unchecked!(
            Return,
            r#"SELECT id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, reason, processed_by, created_at FROM returns WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(ret)
    }

    /// Single-line wrapper over [`Self::create_bulk`].
    ///
    /// Kept so the old `create_return` command and every existing caller keep
    /// working; single-line returns take the exact same code path as a
    /// multi-line submission with one line.
    pub async fn create(&self, input: CreateReturnInput) -> Result<Return, AppError> {
        let mut created = self
            .create_bulk(CreateReturnsInput {
                invoice_id: input.invoice_id,
                reason: input.reason,
                processed_by: input.processed_by,
                lines: vec![CreateReturnLineInput {
                    product_id: input.product_id,
                    product_name: input.product_name,
                    quantity: input.quantity,
                    unit: input.unit,
                    unit_price: input.unit_price,
                }],
            })
            .await?;
        created
            .pop()
            .ok_or(AppError::NotFound("Return not found after creation".into()))
    }

    /// Apply every line in ONE transaction.
    ///
    /// All-or-nothing: if any line fails its guard, the transaction is dropped
    /// and NOTHING is written — no return rows, no stock, no ledger change.
    /// (The old UI drove `create()` once per line from the frontend, so a
    /// failure on line 3 stranded lines 1–2 as a permanent partial return.)
    ///
    /// `propagate_carry_forward` and `reconcile_customer` run ONCE after the
    /// loop with the summed reduction. That is provably identical to calling
    /// them per line: `saturating_sub` and `min` are order-independent, and
    /// `propagate` telescopes (`reduction = old_due - new_due`), so
    /// `Σ(old−new) = due₀ − due_final`. It turns `N × chain_length` walks into
    /// one walk and `N` balance scans into one.
    pub async fn create_bulk(&self, input: CreateReturnsInput) -> Result<Vec<Return>, AppError> {
        if input.lines.is_empty() {
            return Err(AppError::Validation(
                "Enter at least one returned quantity".into(),
            ));
        }
        for line in &input.lines {
            if line.quantity <= 0 {
                return Err(AppError::Validation(format!(
                    "{}: quantity must be at least 1",
                    line.product_name
                )));
            }
            if line.unit_price < 0 {
                return Err(AppError::Validation(format!(
                    "{}: price cannot be negative",
                    line.product_name
                )));
            }
        }

        let mut tx = self.pool.begin().await?;
        let now = Utc::now().to_rfc3339();

        let mut created: Vec<Return> = Vec::with_capacity(input.lines.len());
        let mut total_reduction: i64 = 0;
        let mut customer_id: Option<String> = None;

        for line in &input.lines {
            let id = Uuid::new_v4().to_string();

            // Server is the source of truth for the credited value.
            let line_total = line.quantity * line.unit_price;

            // Over-return guard (ledger integrity): cumulative returned quantity
            // per product must never exceed what was originally invoiced.
            // Matched on product_id when present, falling back to product name.
            // The reads run on THIS transaction, so lines earlier in the loop
            // are already visible — two lines for the same product are safe.
            let sold: i64 = if let Some(pid) = &line.product_id {
                sqlx::query!(
                    "SELECT COALESCE(SUM(quantity), 0) AS q FROM invoice_items WHERE invoice_id = ? AND product_id = ?",
                    input.invoice_id, pid
                )
                .fetch_one(&mut *tx)
                .await?
                .q
            } else {
                sqlx::query!(
                    "SELECT COALESCE(SUM(quantity), 0) AS q FROM invoice_items WHERE invoice_id = ? AND product_name = ?",
                    input.invoice_id, line.product_name
                )
                .fetch_one(&mut *tx)
                .await?
                .q
            };
            let already: i64 = if let Some(pid) = &line.product_id {
                sqlx::query!(
                    "SELECT COALESCE(SUM(quantity), 0) AS q FROM returns WHERE invoice_id = ? AND product_id = ?",
                    input.invoice_id, pid
                )
                .fetch_one(&mut *tx)
                .await?
                .q
            } else {
                sqlx::query!(
                    "SELECT COALESCE(SUM(quantity), 0) AS q FROM returns WHERE invoice_id = ? AND product_name = ?",
                    input.invoice_id, line.product_name
                )
                .fetch_one(&mut *tx)
                .await?
                .q
            };
            if line.quantity > sold - already {
                return Err(AppError::Validation(format!(
                    "Cannot return {} {}: only {} remain returnable on this invoice",
                    line.quantity,
                    line.unit,
                    (sold - already).max(0)
                )));
            }

            sqlx::query!(
                r#"INSERT INTO returns (id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, reason, processed_by, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                id, input.invoice_id, line.product_id, line.product_name, line.quantity, line.unit, line.unit_price, line_total, input.reason, input.processed_by, now
            )
            .execute(&mut *tx)
            .await?;

            // Restore product stock
            if let Some(product_id) = &line.product_id {
                sqlx::query!("UPDATE products SET stock_qty = stock_qty + ? WHERE id = ?", line.quantity, product_id)
                    .execute(&mut *tx)
                    .await?;
            }

            // Adjust the invoice ledger (mirrors original process_return RPC):
            // shrink subtotal/total by returned value and clamp paid.
            let invoice = sqlx::query!(
                "SELECT customer_id, subtotal, discount, previous_balance, total, amount_paid FROM invoices WHERE id = ?",
                input.invoice_id
            )
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(inv) = invoice {
                // Prorate the invoice-level discount over the goods going back.
                //
                // Migration 015 enforces `CHECK (discount <= subtotal)`, so simply
                // subtracting line_total from subtotal used to blow up with
                // "CHECK constraint failed" the moment a return pushed the
                // remaining subtotal below the original discount (every single-line
                // invoice with a discount — returning the item always failed).
                // It also left the ledger crediting the *gross* line_total while
                // the customer had only paid net-of-discount.
                //
                // Reports already value a return at
                // `line_total * (subtotal - discount) / subtotal`
                // (commands/reports.rs), so flooring `discount * line / subtotal`
                // keeps the two in agreement. The ratio (subtotal-discount)/subtotal
                // is preserved exactly, and
                //   new_discount - new_subtotal = ceil(K*line/sub) - K <= 0
                // (K = sub - discount >= 0, line <= sub), so the CHECK holds by
                // construction; `.min(new_subtotal)` is a hard backstop anyway.
                // With discount == 0 this collapses to the previous arithmetic
                // byte-for-byte.
                //
                // Reading the invoice again each iteration is what makes the
                // chain self-correcting: line 2 sees the row line 1 wrote.
                let sub = inv.subtotal;
                let disc_back = if sub > 0 {
                    inv.discount * line_total / sub
                } else {
                    inv.discount
                };
                let new_subtotal = (sub - line_total).max(0);
                let new_discount = (inv.discount - disc_back).max(0).min(new_subtotal);
                let new_total = new_subtotal - new_discount + inv.previous_balance;
                let new_paid = inv.amount_paid.min(new_total);
                let old_due = (inv.total - inv.amount_paid).max(0);
                let new_due = (new_total - new_paid).max(0);
                let reduction = old_due - new_due;

                sqlx::query!(
                    r#"UPDATE invoices SET subtotal = ?, discount = ?, total = ?, amount_paid = ?, updated_at = ? WHERE id = ?"#,
                    new_subtotal, new_discount, new_total, new_paid, now, input.invoice_id
                )
                .execute(&mut *tx)
                .await?;

                total_reduction += reduction;
                if let Some(cid) = inv.customer_id {
                    customer_id = Some(cid);
                }
            }

            created.push(Return {
                id,
                invoice_id: input.invoice_id.clone(),
                product_id: line.product_id.clone(),
                product_name: line.product_name.clone(),
                quantity: line.quantity,
                unit: line.unit.clone(),
                unit_price: line.unit_price,
                line_total,
                reason: input.reason.clone(),
                processed_by: input.processed_by.clone(),
                created_at: now.clone(),
            });
        }

        if total_reduction != 0 {
            if let Some(cid) = &customer_id {
                // A return against an invoice whose balance was carried to a
                // newer invoice must shrink the whole successor chain too —
                // otherwise the next invoice still bills goods that went back.
                crate::repositories::invoices::propagate_carry_forward(
                    &mut tx,
                    &input.invoice_id,
                    total_reduction,
                )
                .await?;

                // Recompute the balance from the leaf invoices rather than
                // nudging it by hand: a carried invoice is not a leaf, so a
                // plain `+= due_delta` here would be double-counted (or
                // silently discarded) once reconciliation runs at startup.
                crate::services::reconciliation::reconcile_customer(&mut tx, cid).await?;
            }
        }

        tx.commit().await?;
        Ok(created)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::customers::Customer;
    use crate::repositories::invoices::{
        CreateInvoiceInput, CreateInvoiceItemInput, Invoice, InvoiceRepository,
    };
    use crate::services::reconciliation::reconcile_balances;
    use crate::test_support::*;

    async fn process_return(
        pool: &SqlitePool,
        user: &str,
        invoice_id: &str,
        product_name: &str,
        quantity: i64,
        unit_price: i64,
    ) -> Result<Return, AppError> {
        ReturnRepository::new(pool.clone())
            .create(CreateReturnInput {
                invoice_id: invoice_id.to_string(),
                product_id: None,
                product_name: product_name.to_string(),
                quantity,
                unit: "pcs".to_string(),
                unit_price,
                reason: "test return".to_string(),
                processed_by: user.to_string(),
            })
            .await
    }

    /// An invoice with an explicit discount and optional up-front payment.
    /// `test_support::credit_invoice` hardcodes `discount: 0` and nothing paid,
    /// so the discounted-invoice paths need their own builder. Local to this
    /// module on purpose: shared fixtures stay untouched for the other tests.
    async fn discounted_invoice(
        pool: &SqlitePool,
        customer: &Customer,
        items: Vec<CreateInvoiceItemInput>,
        discount: i64,
        amount_paid: Option<i64>,
    ) -> Invoice {
        let subtotal = items.iter().map(|i| i.line_total).sum();
        InvoiceRepository::new(pool.clone())
            .create(CreateInvoiceInput {
                customer_id: Some(customer.id.clone()),
                customer_name: customer.name.clone(),
                subtotal,
                discount,
                total: 0, // backend is source of truth; this is ignored
                amount_paid,
                payment_method: "credit".to_string(),
                notes: None,
                delivery_date: None,
                items,
            })
            .await
            .expect("create discounted invoice")
    }

    #[tokio::test]
    async fn return_on_a_leaf_invoice_reduces_the_balance() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Leaf").await;
        let inv = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;
        assert_eq!(balance(&pool, &c.id).await, 1000);

        process_return(&pool, &user, &inv.id, "Tile", 2, 100)
            .await
            .unwrap();

        let after = get_invoice(&pool, &inv.id).await;
        assert_eq!(after.subtotal, 800);
        assert_eq!(after.total, 800);
        assert_eq!(balance(&pool, &c.id).await, 800);
        assert_eq!(
            balance(&pool, &c.id).await,
            expected_leaf_balance(&pool, &c.id).await
        );
    }

    #[tokio::test]
    async fn return_on_a_carried_invoice_shrinks_its_successor() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Carried").await;

        let a = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;
        let b = credit_invoice(&pool, &c, vec![item("Mixer", 1, 500)]).await;
        assert_eq!(
            get_invoice(&pool, &a.id).await.carried_to_invoice_id,
            Some(b.id.clone())
        );
        assert_eq!(balance(&pool, &c.id).await, 1500);

        // Returning goods from A must travel to B: A is not the leaf, so a
        // plain balance nudge here would be wiped by the next reconcile.
        process_return(&pool, &user, &a.id, "Tile", 2, 100)
            .await
            .unwrap();

        assert_eq!(get_invoice(&pool, &a.id).await.total, 800);

        let b_after = get_invoice(&pool, &b.id).await;
        assert_eq!(b_after.previous_balance, 800); // was 1000
        assert_eq!(b_after.total, 1300); // 500 new + 800 carried
        assert_eq!(balance(&pool, &c.id).await, 1300);

        // Reconciling must not move it: ledger and cached balance already agree.
        reconcile_balances(&pool).await.unwrap();
        assert_eq!(balance(&pool, &c.id).await, 1300);
    }

    #[tokio::test]
    async fn return_propagates_through_a_three_invoice_chain() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "LongChain").await;

        let a = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await; // 1000
        let b = credit_invoice(&pool, &c, vec![item("Mixer", 1, 500)]).await; // 1500
        let d = credit_invoice(&pool, &c, vec![item("Tap", 1, 200)]).await; // 1700
        assert_eq!(balance(&pool, &c.id).await, 1700);

        process_return(&pool, &user, &a.id, "Tile", 2, 100)
            .await
            .unwrap();

        let b_after = get_invoice(&pool, &b.id).await;
        assert_eq!(b_after.previous_balance, 800);
        assert_eq!(b_after.total, 1300);

        let d_after = get_invoice(&pool, &d.id).await;
        assert_eq!(d_after.previous_balance, 1300);
        assert_eq!(d_after.total, 1500);

        assert_eq!(balance(&pool, &c.id).await, 1500);
        assert_eq!(
            balance(&pool, &c.id).await,
            expected_leaf_balance(&pool, &c.id).await
        );
    }

    #[tokio::test]
    async fn return_on_a_middle_invoice_leaves_its_upstream_alone() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Middle").await;

        let a = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await; // 1000
        let b = credit_invoice(&pool, &c, vec![item("Mixer", 1, 500)]).await; // 1500
        let d = credit_invoice(&pool, &c, vec![item("Tap", 1, 200)]).await; // 1700

        process_return(&pool, &user, &b.id, "Mixer", 1, 500)
            .await
            .unwrap();

        // Upstream (A) is already absorbed and must not change.
        assert_eq!(get_invoice(&pool, &a.id).await.total, 1000);
        // Downstream (D) absorbs B's shrink: 1500 -> 1000 carried.
        let d_after = get_invoice(&pool, &d.id).await;
        assert_eq!(d_after.previous_balance, 1000);
        assert_eq!(d_after.total, 1200);
        assert_eq!(balance(&pool, &c.id).await, 1200);
    }

    #[tokio::test]
    async fn return_with_no_successor_ends_the_walk_cleanly() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Lonely").await;
        let inv = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;

        process_return(&pool, &user, &inv.id, "Tile", 1, 100)
            .await
            .unwrap();

        assert_eq!(get_invoice(&pool, &inv.id).await.total, 900);
        assert_eq!(balance(&pool, &c.id).await, 900);
    }

    #[tokio::test]
    async fn walkin_returns_touch_no_customer_balance() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;

        let inv = InvoiceRepository::new(pool.clone())
            .create(CreateInvoiceInput {
                customer_id: None,
                customer_name: "Walk-in".into(),
                subtotal: 1000,
                discount: 0,
                total: 0,
                amount_paid: Some(1000),
                payment_method: "cash".into(),
                notes: None,
                delivery_date: None,
                items: vec![item("Tile", 10, 100)],
            })
            .await
            .unwrap();

        process_return(&pool, &user, &inv.id, "Tile", 2, 100)
            .await
            .unwrap();

        let after = get_invoice(&pool, &inv.id).await;
        assert_eq!(after.total, 800);
        assert_eq!(after.amount_paid, 800); // paid clamps down with the total

        let customers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM customers")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(customers, 0);
    }

    #[tokio::test]
    async fn over_return_is_still_rejected() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Greedy").await;
        let inv = credit_invoice(&pool, &c, vec![item("Tile", 3, 100)]).await;

        match process_return(&pool, &user, &inv.id, "Tile", 4, 100).await {
            Err(AppError::Validation(msg)) => {
                assert!(msg.contains("remain returnable"), "unexpected: {}", msg)
            }
            Err(other) => panic!("expected Validation error, got {:?}", other),
            Ok(_) => panic!("expected Validation error, got Ok"),
        }
        assert_eq!(get_invoice(&pool, &inv.id).await.total, 300);
        assert_eq!(balance(&pool, &c.id).await, 300);
    }

    // ==================================================================
    // Discounted invoices — regression coverage for the 1.1.1 hotfix.
    // Before the proration fix these all died with
    //   "CHECK constraint failed: discount >= 0 AND discount <= subtotal"
    // because subtotal was shrunk while discount was left behind.
    // ==================================================================

    #[tokio::test]
    async fn full_return_of_the_only_line_on_a_discounted_invoice_clears_it() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Discounted").await;

        // subtotal 3500, discount 300, total 3200 — a single-line invoice with
        // a discount. Returning its only line drove subtotal to 0 while
        // discount stayed at 300, tripping the migration-015 CHECK.
        let inv = discounted_invoice(&pool, &c, vec![item("Bottle Trap", 1, 3500)], 300, Some(3200))
            .await;
        assert_eq!(inv.subtotal, 3500);
        assert_eq!(inv.discount, 300);
        assert_eq!(inv.total, 3200);

        process_return(&pool, &user, &inv.id, "Bottle Trap", 1, 3500)
            .await
            .expect("return on a discounted invoice must not trip the discount CHECK");

        let after = get_invoice(&pool, &inv.id).await;
        assert_eq!(after.subtotal, 0);
        assert_eq!(after.discount, 0);
        assert_eq!(after.total, 0);
        assert_eq!(after.amount_paid, 0);
        assert_eq!(balance(&pool, &c.id).await, 0);
        assert_eq!(
            balance(&pool, &c.id).await,
            expected_leaf_balance(&pool, &c.id).await
        );
    }

    #[tokio::test]
    async fn partial_return_prorates_the_discount_and_matches_reports() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Prorate").await;

        let inv = discounted_invoice(
            &pool,
            &c,
            vec![item("Tap", 1, 3000), item("Mixer", 1, 7000)],
            1000,
            None,
        )
        .await;
        assert_eq!(inv.subtotal, 10000);
        assert_eq!(inv.discount, 1000);
        assert_eq!(inv.total, 9000);

        process_return(&pool, &user, &inv.id, "Tap", 1, 3000).await.unwrap();

        let after = get_invoice(&pool, &inv.id).await;
        // discount comes back in the same proportion the goods did: 3000/10000
        assert_eq!(after.subtotal, 7000);
        assert_eq!(after.discount, 700);
        assert_eq!(after.total, 6300);
        // the identity propagate_carry_forward and the UI both rely on
        assert_eq!(
            after.total,
            after.subtotal - after.discount + after.previous_balance
        );
        // credit issued == what commands/reports.rs values a return at:
        //   line_total * (subtotal - discount) / subtotal = 3000 * 9000 / 10000
        assert_eq!(inv.total - after.total, 2700);
        assert_eq!(balance(&pool, &c.id).await, 6300);
    }

    #[tokio::test]
    async fn returns_on_a_discounted_invoice_never_trip_the_check() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Drain").await;

        // Seven lines at awkward prices so the integer floor in
        // `discount * line / subtotal` is actually exercised on every step.
        let items: Vec<CreateInvoiceItemInput> = (1i64..=7)
            .map(|n| item(&format!("Line {}", n), 1, n * 1111))
            .collect();
        let inv = discounted_invoice(&pool, &c, items, 777, None).await;
        assert_eq!(inv.subtotal, 31108);

        for n in 1i64..=7 {
            let name = format!("Line {}", n);
            process_return(&pool, &user, &inv.id, &name, 1, n * 1111)
                .await
                .unwrap_or_else(|e| panic!("return {} tripped a constraint: {:?}", n, e));
        }

        let after = get_invoice(&pool, &inv.id).await;
        assert_eq!(after.subtotal, 0);
        assert_eq!(after.discount, 0);
        assert_eq!(after.total, 0);
        assert_eq!(balance(&pool, &c.id).await, 0);
    }

    // ==================================================================
    // Bulk (all-or-nothing) returns — the v1.1.2 change. The old UI drove
    // `create()` once per line from the frontend, so a guard failure on
    // line 3 stranded lines 1-2 as a permanent partial return. Every test
    // here asserts either "everything landed" or "nothing landed".
    // ==================================================================

    fn line(product_name: &str, quantity: i64, unit_price: i64) -> CreateReturnLineInput {
        CreateReturnLineInput {
            product_id: None,
            product_name: product_name.to_string(),
            quantity,
            unit: "pcs".to_string(),
            unit_price,
        }
    }

    async fn bulk_return(
        pool: &SqlitePool,
        user: &str,
        invoice_id: &str,
        lines: Vec<CreateReturnLineInput>,
    ) -> Result<Vec<Return>, AppError> {
        ReturnRepository::new(pool.clone())
            .create_bulk(CreateReturnsInput {
                invoice_id: invoice_id.to_string(),
                reason: "test return".to_string(),
                processed_by: user.to_string(),
                lines,
            })
            .await
    }

    async fn count_returns(pool: &SqlitePool, invoice_id: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM returns WHERE invoice_id = ?")
            .bind(invoice_id)
            .fetch_one(pool)
            .await
            .expect("count return rows")
    }

    #[tokio::test]
    async fn bulk_return_is_all_or_nothing() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "Atomic").await;
        let inv = credit_invoice(&pool, &c, vec![item("Tile", 3, 100), item("Mixer", 1, 500)]).await;
        assert_eq!(inv.total, 800);
        assert_eq!(balance(&pool, &c.id).await, 800);

        // Line 2 over-returns, AFTER line 1 already succeeded in the same tx.
        let bad = bulk_return(
            &pool,
            &user,
            &inv.id,
            vec![line("Tile", 2, 100), line("Mixer", 5, 500)],
        )
        .await;
        match bad {
            Err(AppError::Validation(msg)) => {
                assert!(msg.contains("remain returnable"), "unexpected: {}", msg)
            }
            Err(other) => panic!("expected Validation error, got {:?}", other),
            Ok(_) => panic!("expected Validation error, got Ok"),
        }

        // Nothing from the failed submission may survive: no rows, no ledger.
        assert_eq!(count_returns(&pool, &inv.id).await, 0);
        let after_fail = get_invoice(&pool, &inv.id).await;
        assert_eq!(after_fail.subtotal, 800);
        assert_eq!(after_fail.total, 800);
        assert_eq!(balance(&pool, &c.id).await, 800);

        // The whole submission is JSON from the browser, so prove a tampered
        // `line_total` can never reach the ledger: the field is not part of
        // the input type, and what lands in `returns.line_total` is always
        // `quantity * unit_price` computed here.
        let payload = format!(
            r#"{{"invoice_id":"{}","reason":"test return","processed_by":"{}","lines":[
                 {{"product_name":"Tile","quantity":3,"unit":"pcs","unit_price":100,"line_total":999999}},
                 {{"product_name":"Mixer","quantity":1,"unit":"pcs","unit_price":500,"line_total":1}}]}}"#,
            inv.id, user
        );
        let input: CreateReturnsInput = serde_json::from_str(&payload).expect("parse payload");
        let created = ReturnRepository::new(pool.clone())
            .create_bulk(input)
            .await
            .expect("valid bulk return");

        assert_eq!(created.len(), 2);
        for r in &created {
            assert_eq!(r.line_total, r.quantity * r.unit_price);
        }
        assert_eq!(
            created.iter().map(|r| r.line_total).sum::<i64>(),
            300 + 500
        );
        assert_eq!(count_returns(&pool, &inv.id).await, 2);
        let after = get_invoice(&pool, &inv.id).await;
        assert_eq!(after.subtotal, 0);
        assert_eq!(after.total, 0);
        assert_eq!(balance(&pool, &c.id).await, 0);
    }

    #[tokio::test]
    async fn bulk_return_matches_n_sequential_single_returns() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let cb = make_customer(&pool, "BulkSide").await;
        let cs = make_customer(&pool, "SeqSide").await;

        let shape = || vec![item("Tap", 1, 3000), item("Mixer", 1, 7000), item("Tile", 10, 100)];
        let inv_b = discounted_invoice(&pool, &cb, shape(), 1000, None).await;
        let inv_s = discounted_invoice(&pool, &cs, shape(), 1000, None).await;
        assert_eq!(inv_b.subtotal, 11000);
        assert_eq!(inv_b.total, 10000);

        // Two independent submissions of the SAME shape: one primitive call,
        // two legacy calls. The legacy path re-runs propagate + reconcile
        // after every line; the bulk path runs them once with the summed
        // reduction. If the telescoping claim were wrong these diverge.
        bulk_return(
            &pool,
            &user,
            &inv_b.id,
            vec![line("Tap", 1, 3000), line("Tile", 10, 100)],
        )
        .await
        .unwrap();
        process_return(&pool, &user, &inv_s.id, "Tap", 1, 3000).await.unwrap();
        process_return(&pool, &user, &inv_s.id, "Tile", 10, 100).await.unwrap();

        let b = get_invoice(&pool, &inv_b.id).await;
        let s = get_invoice(&pool, &inv_s.id).await;
        assert_eq!(
            (b.subtotal, b.discount, b.total, b.previous_balance, b.amount_paid),
            (s.subtotal, s.discount, s.total, s.previous_balance, s.amount_paid)
        );

        // Pinned by hand so an "identically wrong" pair cannot pass:
        //   Tap:  disc_back = 1000*3000/11000 = 272 -> sub 8000 disc 728
        //   Tile: disc_back = 728*1000/8000   = 91  -> sub 7000 disc 637
        assert_eq!(b.subtotal, 7000);
        assert_eq!(b.discount, 637);
        assert_eq!(b.total, 6363);
        assert_eq!(
            b.total,
            b.subtotal - b.discount + b.previous_balance
        );

        assert_eq!(balance(&pool, &cb.id).await, balance(&pool, &cs.id).await);
        assert_eq!(balance(&pool, &cb.id).await, 6363);
        assert_eq!(
            balance(&pool, &cb.id).await,
            expected_leaf_balance(&pool, &cb.id).await
        );
        assert_eq!(
            balance(&pool, &cs.id).await,
            expected_leaf_balance(&pool, &cs.id).await
        );

        // Row-for-row identical (ids and timestamps excluded: one submission
        // stamps a single created_at, two calls naturally stamp two).
        let norm = |rows: Vec<Return>| {
            let mut v: Vec<(String, i64, i64, i64, String)> = rows
                .into_iter()
                .map(|r| (r.product_name, r.quantity, r.unit_price, r.line_total, r.reason))
                .collect();
            v.sort();
            v
        };
        let repo = ReturnRepository::new(pool.clone());
        let rb = repo.list(Some(inv_b.id.clone())).await.unwrap();
        let rs = repo.list(Some(inv_s.id.clone())).await.unwrap();
        assert_eq!(rb.len(), 2);
        assert_eq!(norm(rb), norm(rs));
    }

    #[tokio::test]
    async fn bulk_return_on_a_discounted_carried_invoice_propagates_once() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "CarriedDiscount").await;

        // A carries a discount AND is unpaid, so its whole 9000 sits on B.
        let a = discounted_invoice(
            &pool,
            &c,
            vec![item("Tap", 1, 3000), item("Mixer", 1, 7000)],
            1000,
            None,
        )
        .await;
        assert_eq!(a.total, 9000);
        let b = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;
        assert_eq!(b.previous_balance, 9000);
        assert_eq!(b.total, 10000);
        assert_eq!(balance(&pool, &c.id).await, 10000);

        // Full return of both lines in ONE submission: reduction 2700 + 6300.
        bulk_return(
            &pool,
            &user,
            &a.id,
            vec![line("Tap", 1, 3000), line("Mixer", 1, 7000)],
        )
        .await
        .unwrap();

        let a_after = get_invoice(&pool, &a.id).await;
        assert_eq!(a_after.subtotal, 0);
        assert_eq!(a_after.discount, 0);
        assert_eq!(a_after.total, 0);
        assert_eq!(a_after.amount_paid, 0);

        // propagate ran once with 9000 — not zero times (B keeps billing the
        // goods) and not twice (B would go negative).
        let b_after = get_invoice(&pool, &b.id).await;
        assert_eq!(b_after.previous_balance, 0);
        assert_eq!(b_after.total, 1000);
        assert_eq!(balance(&pool, &c.id).await, 1000);
        assert_eq!(
            balance(&pool, &c.id).await,
            expected_leaf_balance(&pool, &c.id).await
        );

        reconcile_balances(&pool).await.unwrap();
        assert_eq!(balance(&pool, &c.id).await, 1000);
    }

    #[tokio::test]
    async fn bulk_return_with_duplicate_product_lines_respects_the_guard() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "DupLines").await;
        let inv = credit_invoice(&pool, &c, vec![item("Tile", 3, 100)]).await;
        assert_eq!(inv.total, 300);

        // 2 + 2 = 4 > 3 sold. The guard reads through THIS tx, so line 2 sees
        // line 1's insert — and the whole submission must be discarded.
        let bad = bulk_return(
            &pool,
            &user,
            &inv.id,
            vec![line("Tile", 2, 100), line("Tile", 2, 100)],
        )
        .await;
        match bad {
            Err(AppError::Validation(msg)) => {
                assert!(msg.contains("remain returnable"), "unexpected: {}", msg)
            }
            Err(other) => panic!("expected Validation error, got {:?}", other),
            Ok(_) => panic!("expected Validation error, got Ok"),
        }
        assert_eq!(count_returns(&pool, &inv.id).await, 0);
        assert_eq!(get_invoice(&pool, &inv.id).await.total, 300);
        assert_eq!(balance(&pool, &c.id).await, 300);

        // The same two lines within budget DO land together.
        bulk_return(
            &pool,
            &user,
            &inv.id,
            vec![line("Tile", 2, 100), line("Tile", 1, 100)],
        )
        .await
        .unwrap();
        assert_eq!(count_returns(&pool, &inv.id).await, 2);
        assert_eq!(get_invoice(&pool, &inv.id).await.total, 0);
        assert_eq!(balance(&pool, &c.id).await, 0);
    }

    #[tokio::test]
    async fn empty_bulk_return_is_rejected() {
        let pool = test_pool().await;
        let user = seed_user(&pool).await;
        let c = make_customer(&pool, "NothingToSend").await;
        let inv = credit_invoice(&pool, &c, vec![item("Tile", 3, 100)]).await;

        let empty = bulk_return(&pool, &user, &inv.id, vec![]).await;
        assert!(matches!(empty, Err(AppError::Validation(_))), "empty lines accepted");

        let zero_qty = bulk_return(&pool, &user, &inv.id, vec![line("Tile", 0, 100)]).await;
        assert!(matches!(zero_qty, Err(AppError::Validation(_))), "zero quantity accepted");

        let negative = bulk_return(&pool, &user, &inv.id, vec![line("Tile", 1, -100)]).await;
        assert!(matches!(negative, Err(AppError::Validation(_))), "negative price accepted");

        // Validation runs before the transaction opens: nothing was written.
        assert_eq!(count_returns(&pool, &inv.id).await, 0);
        assert_eq!(get_invoice(&pool, &inv.id).await.total, 300);
        assert_eq!(balance(&pool, &c.id).await, 300);
    }
}