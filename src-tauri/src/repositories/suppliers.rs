// src-tauri/src/repositories/suppliers.rs
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::{Utc, Datelike};
use crate::error::AppError;

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Supplier {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
    pub outstanding_balance: i64,
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

pub struct SupplierRepository {
    pool: SqlitePool,
}

impl SupplierRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Supplier>, AppError> {
        let suppliers = sqlx::query_as::<_, Supplier>(
            r#"SELECT id, name, phone, email, company, address, notes, outstanding_balance, created_at, updated_at FROM suppliers ORDER BY name"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(suppliers)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Supplier>, AppError> {
        let supplier = sqlx::query_as::<_, Supplier>(
            r#"SELECT id, name, phone, email, company, address, notes, outstanding_balance, created_at, updated_at FROM suppliers WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(supplier)
    }

    pub async fn create(&self, input: CreateSupplierInput) -> Result<Supplier, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"INSERT INTO suppliers (id, name, phone, email, company, address, notes, outstanding_balance, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, ?)"#,
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.phone)
        .bind(&input.email)
        .bind(&input.company)
        .bind(&input.address)
        .bind(&input.notes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get(&id).await?.ok_or_else(|| AppError::NotFound("Supplier not found after creation".into()))
    }

    pub async fn update(&self, id: &str, input: CreateSupplierInput) -> Result<Supplier, AppError> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"UPDATE suppliers SET name = ?, phone = ?, email = ?, company = ?, address = ?, notes = ?, updated_at = ? WHERE id = ?"#,
        )
        .bind(&input.name)
        .bind(&input.phone)
        .bind(&input.email)
        .bind(&input.company)
        .bind(&input.address)
        .bind(&input.notes)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.get(id).await?.ok_or_else(|| AppError::NotFound("Supplier not found after update".into()))
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let supplier = self.get(id).await?.ok_or_else(|| AppError::NotFound("Supplier not found".into()))?;
        if supplier.outstanding_balance != 0 {
            return Err(AppError::Validation(format!(
                "Cannot delete supplier with non-zero balance of PKR {}",
                supplier.outstanding_balance
            )));
        }

        sqlx::query("DELETE FROM suppliers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

pub struct SupplierPurchaseRepository {
    pool: SqlitePool,
}

impl SupplierPurchaseRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub const SEARCH_CAP: i64 = 200;

    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
        query: Option<&str>,
        supplier_id: Option<&str>,
    ) -> Result<Vec<SupplierPurchase>, AppError> {
        let purchases = if let Some(sid) = supplier_id.map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query_as::<_, SupplierPurchase>(
                r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, amount_paid, payment_method, notes, created_at, updated_at
                   FROM supplier_purchases
                   WHERE supplier_id = ?
                   ORDER BY created_at DESC LIMIT ?"#,
            )
            .bind(sid)
            .bind(Self::SEARCH_CAP)
            .fetch_all(&self.pool)
            .await?
        } else if let Some(q) = query.map(str::trim).filter(|q| !q.is_empty()) {
            sqlx::query_as::<_, SupplierPurchase>(
                r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, amount_paid, payment_method, notes, created_at, updated_at
                   FROM supplier_purchases
                   WHERE LOWER(purchase_no) LIKE '%' || LOWER(?) || '%'
                      OR LOWER(supplier_name) LIKE '%' || LOWER(?) || '%'
                   ORDER BY created_at DESC LIMIT ?"#,
            )
            .bind(q)
            .bind(q)
            .bind(Self::SEARCH_CAP)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, SupplierPurchase>(
                r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, amount_paid, payment_method, notes, created_at, updated_at
                   FROM supplier_purchases ORDER BY created_at DESC LIMIT ? OFFSET ?"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(purchases)
    }

    pub async fn get(&self, id: &str) -> Result<Option<SupplierPurchase>, AppError> {
        let purchase = sqlx::query_as::<_, SupplierPurchase>(
            r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, amount_paid, payment_method, notes, created_at, updated_at
               FROM supplier_purchases WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(purchase)
    }

    pub async fn get_with_items(&self, id: &str) -> Result<Option<(SupplierPurchase, Vec<SupplierPurchaseItem>)>, AppError> {
        let purchase = self.get(id).await?;
        if let Some(p) = purchase {
            let items = self.get_items(id).await?;
            Ok(Some((p, items)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_items(&self, purchase_id: &str) -> Result<Vec<SupplierPurchaseItem>, AppError> {
        let items = sqlx::query_as::<_, SupplierPurchaseItem>(
            r#"SELECT id, purchase_id, description, quantity, unit, unit_price, line_total, created_at
               FROM supplier_purchase_items WHERE purchase_id = ?"#,
        )
        .bind(purchase_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(items)
    }

    pub async fn create(&self, input: CreatePurchaseInput) -> Result<SupplierPurchase, AppError> {
        let mut tx = self.pool.begin().await?;

        let purchase_no = get_next_purchase_no(&mut tx).await?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let total = input.subtotal.saturating_sub(input.discount);
        let default_paid = if input.payment_method == "credit" { 0 } else { total };
        let amount_paid = input.amount_paid.unwrap_or(default_paid).clamp(0, total);
        let due = total - amount_paid;

        sqlx::query(
            r#"INSERT INTO supplier_purchases (id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, amount_paid, payment_method, notes, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(&purchase_no)
        .bind(&input.supplier_id)
        .bind(&input.supplier_name)
        .bind(input.subtotal)
        .bind(input.discount)
        .bind(total)
        .bind(amount_paid)
        .bind(&input.payment_method)
        .bind(&input.notes)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        for item in input.items {
            let item_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO supplier_purchase_items (id, purchase_id, description, quantity, unit, unit_price, line_total, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&item_id)
            .bind(&id)
            .bind(&item.description)
            .bind(item.quantity)
            .bind(&item.unit)
            .bind(item.unit_price)
            .bind(item.line_total)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        // Mirror any due amount onto the supplier's outstanding balance
        if due > 0 {
            sqlx::query("UPDATE suppliers SET outstanding_balance = outstanding_balance + ?, updated_at = ? WHERE id = ?")
                .bind(due)
                .bind(&now)
                .bind(&input.supplier_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        self.get(&id).await?.ok_or_else(|| AppError::NotFound("Purchase not found after creation".into()))
    }

    pub async fn mark_paid(&self, id: &str, amount: i64, method: &str) -> Result<SupplierPurchase, AppError> {
        let mut tx = self.pool.begin().await?;

        let purchase = self.get(id).await?.ok_or_else(|| AppError::NotFound("Purchase not found".into()))?;
        let old_paid = purchase.amount_paid;
        let due_before = (purchase.total - old_paid).max(0);

        if amount > due_before {
            return Err(AppError::Validation(format!(
                "Payment {} exceeds remaining balance {}", amount, due_before
            )));
        }

        let applied = amount.clamp(0, due_before);
        let new_paid = old_paid + applied;
        let now = Utc::now().to_rfc3339();

        sqlx::query("UPDATE supplier_purchases SET amount_paid = ?, payment_method = ?, updated_at = ? WHERE id = ?")
            .bind(new_paid)
            .bind(method)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;

        if applied > 0 {
            sqlx::query("UPDATE suppliers SET outstanding_balance = MAX(0, outstanding_balance - ?), updated_at = ? WHERE id = ?")
                .bind(applied)
                .bind(&now)
                .bind(&purchase.supplier_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        self.get(id).await?.ok_or_else(|| AppError::NotFound("Purchase not found after update".into()))
    }

    pub async fn get_next_purchase_no(&self) -> Result<String, AppError> {
        let mut tx = self.pool.begin().await?;
        let no = get_next_purchase_no(&mut tx).await?;
        tx.commit().await?;
        Ok(no)
    }
}

async fn get_next_purchase_no(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<String, AppError> {
    let current_year = chrono::Utc::now().year() as i64;

    let row: Option<(i64, i64)> = sqlx::query_as("SELECT year, sequence FROM purchase_counter WHERE id = 1")
        .fetch_optional(&mut **tx)
        .await?;

    let (year, sequence) = match row {
        Some((y, s)) if y == current_year => (y, s + 1),
        Some(_) => (current_year, 1),
        None => (current_year, 1),
    };

    sqlx::query("INSERT OR REPLACE INTO purchase_counter (id, year, sequence) VALUES (1, ?, ?)")
        .bind(year)
        .bind(sequence)
        .execute(&mut **tx)
        .await?;

    Ok(format!("PO-{}-{:04}", year, sequence))
}