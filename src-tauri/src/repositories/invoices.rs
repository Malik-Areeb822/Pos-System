// src-tauri/src/repositories/invoices.rs
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::{Utc, Datelike};
use crate::error::AppError;

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Invoice {
    pub id: String,
    pub invoice_no: String,
    pub customer_id: Option<String>,
    pub customer_name: String,
    pub subtotal: i64,
    pub discount: i64,
    pub total: i64,
    pub previous_balance: i64,
    pub amount_paid: i64,
    pub payment_method: String,
    pub notes: Option<String>,
    pub delivery_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub carried_to_invoice_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct InvoiceItem {
    pub id: String,
    pub invoice_id: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
    pub purchase_price: i64,
    pub total_area: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateInvoiceInput {
    pub customer_id: Option<String>,
    pub customer_name: String,
    pub subtotal: i64,
    pub discount: i64,
    pub total: i64,
    pub amount_paid: Option<i64>,
    pub payment_method: String,
    pub notes: Option<String>,
    pub delivery_date: Option<String>,
    pub items: Vec<CreateInvoiceItemInput>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateInvoiceItemInput {
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
    pub purchase_price: i64,
    pub total_area: Option<f64>,
}

pub struct InvoiceRepository {
    pool: SqlitePool,
}

impl InvoiceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Cap for full-history searches — keeps the webview table light while
    /// still reaching invoices far older than the default page.
    pub const SEARCH_CAP: i64 = 200;
    pub const RANGE_CAP: i64 = 50_000;

    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
        query: Option<&str>,
        from_date: Option<&str>,
        to_date: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Vec<Invoice>, AppError> {
        let invoices = if let Some(cid) = customer_id.filter(|s| !s.trim().is_empty()) {
            let cap = if limit > 0 && limit <= Self::SEARCH_CAP { limit } else { Self::SEARCH_CAP };
            sqlx::query_as_unchecked!(
                Invoice,
                r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id
                   FROM invoices
                   WHERE customer_id = ?
                   ORDER BY created_at DESC LIMIT ?"#,
                cid,
                cap
            )
            .fetch_all(&self.pool)
            .await?
        } else if from_date.is_some() || to_date.is_some() {
            let cap = if limit > 0 && limit <= Self::RANGE_CAP { limit } else { Self::RANGE_CAP };
            match (from_date, to_date) {
                (Some(from), Some(to)) => {
                    sqlx::query_as_unchecked!(
                        Invoice,
                        r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id
                           FROM invoices
                           WHERE created_at >= ? AND created_at < ?
                           ORDER BY created_at DESC LIMIT ?"#,
                        from,
                        to,
                        cap
                    )
                    .fetch_all(&self.pool)
                    .await?
                }
                (Some(from), None) => {
                    sqlx::query_as_unchecked!(
                        Invoice,
                        r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id
                           FROM invoices
                           WHERE created_at >= ?
                           ORDER BY created_at DESC LIMIT ?"#,
                        from,
                        cap
                    )
                    .fetch_all(&self.pool)
                    .await?
                }
                (None, Some(to)) => {
                    sqlx::query_as_unchecked!(
                        Invoice,
                        r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id
                           FROM invoices
                           WHERE created_at < ?
                           ORDER BY created_at DESC LIMIT ?"#,
                        to,
                        cap
                    )
                    .fetch_all(&self.pool)
                    .await?
                }
                (None, None) => unreachable!(),
            }
        } else if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
            // Full-history search across invoice number and customer name
            // (case-insensitive), newest first, capped.
            sqlx::query_as_unchecked!(
                Invoice,
                r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id FROM invoices
                   WHERE LOWER(invoice_no) LIKE '%' || LOWER(?) || '%'
                      OR LOWER(customer_name) LIKE '%' || LOWER(?) || '%'
                   ORDER BY created_at DESC LIMIT ?"#,
                q,
                q,
                Self::SEARCH_CAP
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as_unchecked!(
                Invoice,
                r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id FROM invoices ORDER BY created_at DESC LIMIT ? OFFSET ?"#,
                limit, offset
            )
            .fetch_all(&self.pool)
            .await?
        };
        Ok(invoices)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Invoice>, AppError> {
        let invoice = sqlx::query_as_unchecked!(
            Invoice,
            r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id FROM invoices WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(invoice)
    }

    pub async fn get_with_items(&self, id: &str) -> Result<Option<(Invoice, Vec<InvoiceItem>)>, AppError> {
        let invoice = self.get(id).await?;
        if let Some(inv) = invoice {
            let items = self.get_items(id).await?;
            Ok(Some((inv, items)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_items(&self, invoice_id: &str) -> Result<Vec<InvoiceItem>, AppError> {
        let items = sqlx::query_as_unchecked!(
            InvoiceItem,
            r#"SELECT id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, purchase_price, total_area, created_at FROM invoice_items WHERE invoice_id = ?"#,
            invoice_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(items)
    }

    pub async fn create(&self, input: CreateInvoiceInput) -> Result<Invoice, AppError> {
        let mut tx = self.pool.begin().await?;

        // Get next invoice number
        let invoice_no = get_next_invoice_no(&mut tx).await?;
        
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Read the customer's current outstanding balance and carry it forward.
        let previous_balance: i64 = if let Some(cid) = &input.customer_id {
            sqlx::query_scalar!("SELECT outstanding_balance FROM customers WHERE id = ?", cid)
                .fetch_optional(&mut *tx)
                .await?
                .unwrap_or(0)
        } else {
            0
        };

        // Total = new items + carried balance.  The frontend may also send
        // `input.total`, but we ignore it — the backend is source of truth.
        let total = input.subtotal.saturating_sub(input.discount) + previous_balance;
        let default_paid = if input.payment_method == "credit" { 0 } else { total };
        let amount_paid = input.amount_paid.unwrap_or(default_paid).clamp(0, total);
        let due = total - amount_paid;

        sqlx::query!(
            r#"INSERT INTO invoices (id, invoice_no, customer_id, customer_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, delivery_date, created_at, updated_at, carried_to_invoice_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)"#,
            id, invoice_no, input.customer_id, input.customer_name, input.subtotal, input.discount, total, previous_balance, amount_paid, input.payment_method, input.notes, input.delivery_date, now, now
        )
        .execute(&mut *tx)
        .await?;

        for item in input.items {
            let item_id = Uuid::new_v4().to_string();
            sqlx::query!(
                r#"INSERT INTO invoice_items (id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, purchase_price, total_area, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                item_id, id, item.product_id, item.product_name, item.quantity, item.unit, item.unit_price, item.line_total, item.purchase_price, item.total_area, now
            )
            .execute(&mut *tx)
            .await?;

            // Update product stock
            if let Some(product_id) = &item.product_id {
                sqlx::query!("UPDATE products SET stock_qty = stock_qty - ? WHERE id = ?", item.quantity, product_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        // Set the customer's outstanding balance to the new due amount
        // (not += — each invoice carries the full chain, so the latest
        // invoice's due IS the customer's total outstanding).
        if let Some(customer_id) = &input.customer_id {
            sqlx::query!("UPDATE customers SET outstanding_balance = MAX(0, ?) WHERE id = ?", due, customer_id)
                .execute(&mut *tx)
                .await?;
        }

        // When this invoice absorbs a previous balance, mark EVERY source
        // invoice it absorbed so the UI can show "Balance carried to this
        // invoice" on each one. A customer may hold more than one open
        // invoice when this feature first runs against pre-existing data —
        // marking only the newest would strand the rest as open-but-invisible
        // debt (they would be excluded from reconciliation while their money
        // is already inside this invoice's total).
        if previous_balance > 0 {
            if let Some(customer_id) = &input.customer_id {
                sqlx::query!(
                    r#"UPDATE invoices SET carried_to_invoice_id = ?
                       WHERE customer_id = ? AND id != ?
                         AND (total - amount_paid) > 0
                         AND carried_to_invoice_id IS NULL"#,
                    id, customer_id, id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.get(&id).await?.ok_or(AppError::NotFound("Invoice not found after creation".into()))
    }

    pub async fn mark_paid(&self, id: &str, amount: i64, method: &str) -> Result<Invoice, AppError> {
        let mut tx = self.pool.begin().await?;

        let invoice = self.get(id).await?.ok_or(AppError::NotFound("Invoice not found".into()))?;

        // Block payment on an invoice whose balance was carried to a newer one.
        if let Some(carried_to) = &invoice.carried_to_invoice_id {
            let linked = self.get(carried_to).await?;
            let linked_no = linked.map(|i| i.invoice_no).unwrap_or_default();
            return Err(AppError::Validation(format!(
                "This invoice's balance was carried to {}. Pay that invoice instead.",
                linked_no
            )));
        }

        let old_paid = invoice.amount_paid;
        let due_before = (invoice.total - old_paid).max(0);
        if amount > due_before {
            return Err(AppError::Validation(format!(
                "Payment {} exceeds remaining balance {}", amount, due_before
            )));
        }
        let applied = amount.clamp(0, due_before);
        let new_paid = old_paid + applied;
        let now = Utc::now().to_rfc3339();

        sqlx::query!(
            r#"UPDATE invoices SET amount_paid = ?, payment_method = ?, updated_at = ? WHERE id = ?"#,
            new_paid, method, now, id
        )
        .execute(&mut *tx)
        .await?;

        if applied > 0 {
            if let Some(customer_id) = &invoice.customer_id {
                sqlx::query!("UPDATE customers SET outstanding_balance = MAX(0, outstanding_balance - ?) WHERE id = ?", applied, customer_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        tx.commit().await?;

        self.get(id).await?.ok_or(AppError::NotFound("Invoice not found after update".into()))
    }

    pub async fn get_next_invoice_no(&self) -> Result<String, AppError> {
        let mut tx = self.pool.begin().await?;
        let no = get_next_invoice_no(&mut tx).await?;
        tx.commit().await?;
        Ok(no)
    }

    pub async fn count(&self) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM invoices")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }
}

/// Hard stop for the carry-chain walk. A legitimate chain is one invoice long
/// per new sale; anything near this depth is either corruption or a loop, and
/// breaking out beats spinning forever.
const MAX_CHAIN_WALK: usize = 100;

/// Push a reduction in one invoice's due amount forward through its carry chain.
///
/// When a return shrinks invoice A's total, every invoice that absorbed A's
/// balance must shrink by the same amount — otherwise the successor still
/// bills a `previous_balance` that no longer exists, and the customer pays
/// for goods they sent back.
///
/// Walks `carried_to_invoice_id` from `start_invoice_id`, re-deriving each
/// successor's `previous_balance` / `total` / `amount_paid`. Stops on a
/// missing successor (retention may have removed the head of a chain) and
/// clamps at 0 wherever a hop would go negative.
pub(crate) async fn propagate_carry_forward(
    conn: &mut sqlx::SqliteConnection,
    start_invoice_id: &str,
    due_reduction: i64,
) -> Result<(), AppError> {
    let head: Option<Option<String>> =
        sqlx::query_scalar("SELECT carried_to_invoice_id FROM invoices WHERE id = ?")
            .bind(start_invoice_id)
            .fetch_optional(&mut *conn)
            .await?;

    let mut cursor = head.flatten();
    let mut reduction = due_reduction.max(0);
    let mut hops = 0usize;

    while let Some(id) = cursor {
        if reduction == 0 {
            break;
        }
        if hops >= MAX_CHAIN_WALK {
            log::warn!(
                "Carry chain exceeded {} hops from invoice {}; stopping propagation",
                MAX_CHAIN_WALK,
                start_invoice_id
            );
            break;
        }
        hops += 1;

        let row: Option<(i64, i64, i64, i64, i64, Option<String>)> = sqlx::query_as(
            r#"SELECT subtotal, discount, previous_balance, total, amount_paid, carried_to_invoice_id
               FROM invoices WHERE id = ?"#,
        )
        .bind(&id)
        .fetch_optional(&mut *conn)
        .await?;

        // Successor already gone (e.g. retention purged it): nothing downstream
        // references it, so the walk is finished rather than an error.
        let (subtotal, discount, previous_balance, total, amount_paid, next) = match row {
            Some(r) => r,
            None => break,
        };

        let items_part = subtotal.saturating_sub(discount);
        let new_previous_balance = previous_balance.saturating_sub(reduction);
        let new_total = items_part + new_previous_balance;
        let new_amount_paid = amount_paid.min(new_total);

        let old_due = (total - amount_paid).max(0);
        let new_due = (new_total - new_amount_paid).max(0);

        sqlx::query(
            r#"UPDATE invoices
               SET previous_balance = ?, total = ?, amount_paid = ?, updated_at = ?
               WHERE id = ?"#,
        )
        .bind(new_previous_balance)
        .bind(new_total)
        .bind(new_amount_paid)
        .bind(Utc::now().to_rfc3339())
        .bind(&id)
        .execute(&mut *conn)
        .await?;

        // Whatever this successor's due actually dropped by is what its own
        // successor absorbs next. Clamping (previous_balance hit 0, or the
        // invoice became fully paid) correctly attenuates the cascade.
        reduction = old_due - new_due;
        cursor = next;
    }

    Ok(())
}

async fn get_next_invoice_no(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<String, AppError> {
    let current_year = chrono::Utc::now().year() as i64;
    
    let row = sqlx::query!("SELECT year, sequence FROM invoice_counter WHERE id = 1")
        .fetch_optional(&mut **tx)
        .await?;
    
    let (year, sequence) = match row {
        Some(r) if r.year == current_year => (r.year, r.sequence + 1),
        Some(_) => (current_year, 1),
        None => (current_year, 1),
    };
    
    sqlx::query!("INSERT OR REPLACE INTO invoice_counter (id, year, sequence) VALUES (1, ?, ?)", year, sequence)
        .execute(&mut **tx)
        .await?;
    
    Ok(format!("INV-{}-{:04}", year, sequence))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[tokio::test]
    async fn create_carries_the_previous_balance_forward() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Kashif").await;

        let first = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;
        assert_eq!(first.previous_balance, 0);
        assert_eq!(first.total, 1000);
        assert_eq!(balance(&pool, &c.id).await, 1000);

        let second = credit_invoice(&pool, &c, vec![item("Mixer", 1, 500)]).await;
        assert_eq!(second.previous_balance, 1000);
        assert_eq!(second.total, 1500);
        assert_eq!(second.amount_paid, 0);
        assert_eq!(balance(&pool, &c.id).await, 1500);

        // The source is flagged so the UI can show "carried to INV-...".
        let source = get_invoice(&pool, &first.id).await;
        assert_eq!(source.carried_to_invoice_id, Some(second.id.clone()));
    }

    #[tokio::test]
    async fn create_marks_every_absorbed_source_not_just_the_newest() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Legacy").await;

        // Two open, uncarried invoices: the state a pre-fix database lands in.
        let old_a = raw_open_invoice(&pool, Some(&c.id), "Legacy", 700, 0, 0, None).await;
        let old_b = raw_open_invoice(&pool, Some(&c.id), "Legacy", 300, 0, 0, None).await;
        sqlx::query("UPDATE customers SET outstanding_balance = 1000 WHERE id = ?")
            .bind(&c.id)
            .execute(&pool)
            .await
            .unwrap();

        let next = credit_invoice(&pool, &c, vec![item("Tap", 1, 250)]).await;
        assert_eq!(next.previous_balance, 1000);
        assert_eq!(next.total, 1250);

        // Both sources must be marked: leaving either open strands a debt
        // that has already been folded into `next`.
        assert_eq!(
            get_invoice(&pool, &old_a).await.carried_to_invoice_id,
            Some(next.id.clone())
        );
        assert_eq!(
            get_invoice(&pool, &old_b).await.carried_to_invoice_id,
            Some(next.id.clone())
        );
        assert_eq!(balance(&pool, &c.id).await, 1250);
    }

    #[tokio::test]
    async fn create_for_walkin_carries_nothing() {
        let pool = test_pool().await;
        let before = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invoices")
            .fetch_one(&pool)
            .await
            .unwrap();

        let inv = InvoiceRepository::new(pool.clone())
            .create(CreateInvoiceInput {
                customer_id: None,
                customer_name: "Walk-in".into(),
                subtotal: 400,
                discount: 0,
                total: 0,
                amount_paid: None,
                payment_method: "credit".into(),
                notes: None,
                delivery_date: None,
                items: vec![item("Seal", 4, 100)],
            })
            .await
            .unwrap();

        assert_eq!(inv.previous_balance, 0);
        assert_eq!(inv.total, 400);
        assert_eq!(inv.carried_to_invoice_id, None);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invoices")
                .fetch_one(&pool)
                .await
                .unwrap(),
            before + 1
        );
    }

    #[tokio::test]
    async fn create_with_cash_pays_the_carried_total_in_full() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Cash").await;
        credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;

        let paid = InvoiceRepository::new(pool.clone())
            .create(CreateInvoiceInput {
                customer_id: Some(c.id.clone()),
                customer_name: c.name.clone(),
                subtotal: 500,
                discount: 0,
                total: 0,
                amount_paid: None,
                payment_method: "cash".into(),
                notes: None,
                delivery_date: None,
                items: vec![item("Valve", 1, 500)],
            })
            .await
            .unwrap();

        assert_eq!(paid.previous_balance, 1000);
        assert_eq!(paid.total, 1500);
        assert_eq!(paid.amount_paid, 1500);
        assert_eq!(balance(&pool, &c.id).await, 0);
    }

    #[tokio::test]
    async fn mark_paid_on_a_carried_invoice_is_rejected() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Chaser").await;
        let first = credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;
        credit_invoice(&pool, &c, vec![item("Mixer", 1, 500)]).await;

        let repo = InvoiceRepository::new(pool.clone());
        match repo.mark_paid(&first.id, 500, "cash").await {
            Err(AppError::Validation(msg)) => {
                assert!(msg.contains("carried"), "unexpected message: {}", msg)
            }
            Err(other) => panic!("expected Validation error, got {:?}", other),
            Ok(_) => panic!("expected Validation error, got Ok"),
        }
    }

    #[tokio::test]
    async fn balance_matches_the_leaf_due_after_a_chain_is_built() {
        let pool = test_pool().await;
        let c = make_customer(&pool, "Chain").await;

        credit_invoice(&pool, &c, vec![item("Tile", 10, 100)]).await;
        let second = credit_invoice(&pool, &c, vec![item("Mixer", 1, 500)]).await;
        let third = credit_invoice(&pool, &c, vec![item("Tap", 2, 250)]).await;

        let leaf = get_invoice(&pool, &third.id).await;
        // 1000 + (1000 + 500) + (1500 + 500): the leaf carries everything.
        assert_eq!(leaf.total - leaf.amount_paid, 2000);
        assert_eq!(balance(&pool, &c.id).await, 2000);
        assert_eq!(
            get_invoice(&pool, &second.id).await.carried_to_invoice_id,
            Some(third.id.clone())
        );
        assert_eq!(balance(&pool, &c.id).await, expected_leaf_balance(&pool, &c.id).await);
    }
}