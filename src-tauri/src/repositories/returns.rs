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

#[derive(Debug, serde::Deserialize)]
pub struct CreateReturnInput {
    pub invoice_id: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
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

    pub async fn create(&self, input: CreateReturnInput) -> Result<Return, AppError> {
        let mut tx = self.pool.begin().await?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Over-return guard (ledger integrity): cumulative returned quantity
        // per product must never exceed what was originally invoiced.
        // Matched on product_id when present, falling back to product name.
        let sold: i64 = if let Some(pid) = &input.product_id {
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
                input.invoice_id, input.product_name
            )
            .fetch_one(&mut *tx)
            .await?
            .q
        };
        let already: i64 = if let Some(pid) = &input.product_id {
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
                input.invoice_id, input.product_name
            )
            .fetch_one(&mut *tx)
            .await?
            .q
        };
        if input.quantity > sold - already {
            return Err(AppError::Validation(format!(
                "Cannot return {} {}: only {} remain returnable on this invoice",
                input.quantity,
                input.unit,
                (sold - already).max(0)
            )));
        }

        sqlx::query!(
            r#"INSERT INTO returns (id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, reason, processed_by, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            id, input.invoice_id, input.product_id, input.product_name, input.quantity, input.unit, input.unit_price, input.line_total, input.reason, input.processed_by, now
        )
        .execute(&mut *tx)
        .await?;

        // Restore product stock
        if let Some(product_id) = &input.product_id {
            sqlx::query!("UPDATE products SET stock_qty = stock_qty + ? WHERE id = ?", input.quantity, product_id)
                .execute(&mut *tx)
                .await?;
        }

        // Adjust the invoice ledger (mirrors original process_return RPC):
        // shrink subtotal/total by returned value and clamp paid.
        let invoice_id = input.invoice_id.clone();
        let invoice_query = sqlx::query!("SELECT customer_id, subtotal, total, amount_paid FROM invoices WHERE id = ?", invoice_id);
        let invoice = invoice_query.fetch_optional(&mut *tx).await?;

        if let Some(inv) = invoice {
            let new_subtotal = (inv.subtotal - input.line_total).max(0);
            let new_total = (inv.total - input.line_total).max(0);
            let new_paid = inv.amount_paid.min(new_total);
            let old_due = (inv.total - inv.amount_paid).max(0);
            let new_due = (new_total - new_paid).max(0);
            let reduction = old_due - new_due;

            sqlx::query!(
                r#"UPDATE invoices SET subtotal = ?, total = ?, amount_paid = ?, updated_at = ? WHERE id = ?"#,
                new_subtotal, new_total, new_paid, now, invoice_id
            )
            .execute(&mut *tx)
            .await?;

            if reduction != 0 {
                if let Some(customer_id) = &inv.customer_id {
                    // A return against an invoice whose balance was carried to a
                    // newer invoice must shrink the whole successor chain too —
                    // otherwise the next invoice still bills goods that went back.
                    crate::repositories::invoices::propagate_carry_forward(
                        &mut tx,
                        &invoice_id,
                        reduction,
                    )
                    .await?;

                    // Recompute the balance from the leaf invoices rather than
                    // nudging it by hand: a carried invoice is not a leaf, so a
                    // plain `+= due_delta` here would be double-counted (or
                    // silently discarded) once reconciliation runs at startup.
                    crate::services::reconciliation::reconcile_customer(&mut tx, customer_id).await?;
                }
            }
        }

        tx.commit().await?;

        self.get(&id).await?.ok_or(AppError::NotFound("Return not found after creation".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::invoices::{CreateInvoiceInput, InvoiceRepository};
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
                line_total: quantity * unit_price,
                reason: "test return".to_string(),
                processed_by: user.to_string(),
            })
            .await
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
}