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
        // shrink subtotal/total by returned value, clamp paid, move due-delta onto customer
        let invoice_id = input.invoice_id.clone();
        let invoice_query = sqlx::query!("SELECT customer_id, subtotal, total, amount_paid FROM invoices WHERE id = ?", invoice_id);
        let invoice = invoice_query.fetch_optional(&mut *tx).await?;

        if let Some(inv) = invoice {
            let new_subtotal = (inv.subtotal - input.line_total).max(0);
            let new_total = (inv.total - input.line_total).max(0);
            let new_paid = inv.amount_paid.min(new_total);
            let old_due = (inv.total - inv.amount_paid).max(0);
            let new_due = (new_total - new_paid).max(0);
            let due_delta = new_due - old_due;

            sqlx::query!(
                r#"UPDATE invoices SET subtotal = ?, total = ?, amount_paid = ?, updated_at = ? WHERE id = ?"#,
                new_subtotal, new_total, new_paid, now, invoice_id
            )
            .execute(&mut *tx)
            .await?;

            if due_delta != 0 {
                if let Some(customer_id) = inv.customer_id {
                    sqlx::query!("UPDATE customers SET outstanding_balance = MAX(0, outstanding_balance + ?) WHERE id = ?", due_delta, customer_id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }

        tx.commit().await?;

        self.get(&id).await?.ok_or(AppError::NotFound("Return not found after creation".into()))
    }
}