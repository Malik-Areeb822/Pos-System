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
    pub amount_paid: i64,
    pub payment_method: String,
    pub notes: Option<String>,
    pub delivery_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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

    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
        query: Option<&str>,
    ) -> Result<Vec<Invoice>, AppError> {
        let invoices = if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
            // Full-history search across invoice number and customer name
            // (case-insensitive), newest first, capped.
            sqlx::query_as_unchecked!(
                Invoice,
                r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, payment_method, notes, delivery_date, created_at, updated_at FROM invoices
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
                r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, payment_method, notes, delivery_date, created_at, updated_at FROM invoices ORDER BY created_at DESC LIMIT ? OFFSET ?"#,
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
            r#"SELECT id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, payment_method, notes, delivery_date, created_at, updated_at FROM invoices WHERE id = ?"#,
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
            r#"SELECT id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, created_at FROM invoice_items WHERE invoice_id = ?"#,
            invoice_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(items)
    }

    pub async fn create(&self, input: CreateInvoiceInput) -> Result<Invoice, AppError> {
        let mut tx = self.pool.begin().await?;

        // Stock availability guard (hard block): aggregate wanted quantities
        // per product across all cart lines — the same product may appear on
        // several lines — then compare once against live stock inside this
        // transaction. Overselling can never drive stock negative.
        {
            let mut wanted: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
            for item in &input.items {
                if let Some(pid) = &item.product_id {
                    *wanted.entry(pid.clone()).or_insert(0) += item.quantity;
                }
            }
            let mut shortfalls: Vec<String> = Vec::new();
            for (pid, qty) in &wanted {
                if *qty <= 0 {
                    continue;
                }
                if let Some(p) = sqlx::query!(
                    "SELECT name, stock_qty FROM products WHERE id = ?",
                    pid
                )
                .fetch_optional(&mut *tx)
                .await?
                {
                    if p.stock_qty < *qty {
                        shortfalls.push(format!(
                            "\"{}\" — in stock: {}, tried: {}",
                            p.name, p.stock_qty, qty
                        ));
                    }
                }
            }
            if !shortfalls.is_empty() {
                return Err(AppError::Validation(format!(
                    "Not enough stock: {}",
                    shortfalls.join("; ")
                )));
            }
        }

        // Get next invoice number
        let invoice_no = get_next_invoice_no(&mut tx).await?;
        
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let default_paid = if input.payment_method == "credit" { 0 } else { input.total };
        let amount_paid = input.amount_paid.unwrap_or(default_paid).clamp(0, input.total);
        let due = input.total - amount_paid;

        sqlx::query!(
            r#"INSERT INTO invoices (id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, payment_method, notes, delivery_date, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            id, invoice_no, input.customer_id, input.customer_name, input.subtotal, input.discount, input.total, amount_paid, input.payment_method, input.notes, input.delivery_date, now, now
        )
        .execute(&mut *tx)
        .await?;

        for item in input.items {
            let item_id = Uuid::new_v4().to_string();
            sqlx::query!(
                r#"INSERT INTO invoice_items (id, invoice_id, product_id, product_name, quantity, unit, unit_price, line_total, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                item_id, id, item.product_id, item.product_name, item.quantity, item.unit, item.unit_price, item.line_total, now
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

        // Mirror any due onto the customer's outstanding balance (any method)
        if due > 0 {
            if let Some(customer_id) = &input.customer_id {
                sqlx::query!("UPDATE customers SET outstanding_balance = outstanding_balance + ? WHERE id = ?", due, customer_id)
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