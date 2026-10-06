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
    pub previous_balance: i64,
    pub amount_paid: i64,
    pub payment_method: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub carried_to_purchase_id: Option<String>,
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
                r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, created_at, updated_at, carried_to_purchase_id
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
                r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, created_at, updated_at, carried_to_purchase_id
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
                r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, created_at, updated_at, carried_to_purchase_id
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
            r#"SELECT id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, created_at, updated_at, carried_to_purchase_id
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

        // Read the supplier's current outstanding balance and carry it forward.
        let supplier_id = input.supplier_id.trim().to_string();
        let previous_balance: i64 = if supplier_id.is_empty() {
            0
        } else {
            sqlx::query_scalar::<_, i64>("SELECT outstanding_balance FROM suppliers WHERE id = ?")
                .bind(&supplier_id)
                .fetch_optional(&mut *tx)
                .await?
                .unwrap_or(0)
        };

        // Total = new items + carried balance. The frontend may also send
        // `input.total`, but we ignore it — the backend is source of truth.
        let total = input.subtotal.saturating_sub(input.discount) + previous_balance;
        let default_paid = if input.payment_method == "credit" { 0 } else { total };
        let amount_paid = input.amount_paid.unwrap_or(default_paid).clamp(0, total);
        let due = total - amount_paid;

        sqlx::query(
            r#"INSERT INTO supplier_purchases (id, purchase_no, supplier_id, supplier_name, subtotal, discount, total, previous_balance, amount_paid, payment_method, notes, created_at, updated_at, carried_to_purchase_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)"#,
        )
        .bind(&id)
        .bind(&purchase_no)
        .bind(&supplier_id)
        .bind(&input.supplier_name)
        .bind(input.subtotal)
        .bind(input.discount)
        .bind(total)
        .bind(previous_balance)
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

        // Set the supplier's outstanding balance to the new due amount
        // (not += — each purchase carries the full chain, so the latest
        // purchase's due IS the supplier's total outstanding). Unconditional,
        // because a fully-paid purchase must also zero the balance.
        if !supplier_id.is_empty() {
            sqlx::query(
                "UPDATE suppliers SET outstanding_balance = MAX(0, ?), updated_at = ? WHERE id = ?",
            )
            .bind(due)
            .bind(&now)
            .bind(&supplier_id)
            .execute(&mut *tx)
            .await?;
        }

        // When this purchase absorbs a previous balance, mark EVERY source
        // purchase it absorbed so the UI can show it as carried on each one.
        // A supplier may hold more than one open purchase when this feature
        // first runs against pre-existing data — marking only the newest would
        // strand the rest as open-but-invisible debt (they would be excluded
        // from reconciliation while their money is already inside this
        // purchase's total).
        if previous_balance > 0 && !supplier_id.is_empty() {
            sqlx::query(
                r#"UPDATE supplier_purchases SET carried_to_purchase_id = ?
                   WHERE supplier_id = ? AND id != ?
                     AND (total - amount_paid) > 0
                     AND carried_to_purchase_id IS NULL"#,
            )
            .bind(&id)
            .bind(&supplier_id)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.get(&id).await?.ok_or_else(|| AppError::NotFound("Purchase not found after creation".into()))
    }

    pub async fn mark_paid(&self, id: &str, amount: i64, method: &str) -> Result<SupplierPurchase, AppError> {
        let mut tx = self.pool.begin().await?;

        let purchase = self.get(id).await?.ok_or_else(|| AppError::NotFound("Purchase not found".into()))?;

        // Block payment on a purchase whose balance was carried to a newer one.
        if let Some(carried_to) = &purchase.carried_to_purchase_id {
            let linked = self.get(carried_to).await?;
            let linked_no = linked.map(|p| p.purchase_no).unwrap_or_default();
            return Err(AppError::Validation(format!(
                "This purchase's balance was carried to {}. Pay that purchase instead.",
                linked_no
            )));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::test_support::*;

    #[tokio::test]
    async fn create_carries_previous_supplier_balance_forward() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Al-Hamd").await;

        credit_purchase(&pool, &s, vec![purchase_item("PVC Pipe", 1, 1000)]).await;
        let second =
            credit_purchase(&pool, &s, vec![purchase_item("Fitting", 1, 500)]).await;

        assert_eq!(second.previous_balance, 1000);
        assert_eq!(second.total, 1500);
        assert_eq!(supplier_balance(&pool, &s.id).await, 1500);
    }

    #[tokio::test]
    async fn create_for_new_supplier_carries_nothing() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Fresh Vendor").await;

        let first =
            credit_purchase(&pool, &s, vec![purchase_item("Mixer tap", 2, 700)]).await;

        assert_eq!(first.previous_balance, 0);
        assert_eq!(first.total, 1400);
        assert_eq!(first.carried_to_purchase_id, None);
        assert_eq!(supplier_balance(&pool, &s.id).await, 1400);
    }

    #[tokio::test]
    async fn create_with_cash_pays_the_carried_total_in_full() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Cash Supplier").await;
        let repo = SupplierPurchaseRepository::new(pool.clone());

        credit_purchase(&pool, &s, vec![purchase_item("Tiles", 1, 1000)]).await;

        // Cash with no explicit amount_paid defaults to the grand total,
        // so the carried balance is settled in the same transaction.
        let cash = repo
            .create(CreatePurchaseInput {
                supplier_id: s.id.clone(),
                supplier_name: s.name.clone(),
                subtotal: 500,
                discount: 0,
                total: 0, // backend is source of truth
                amount_paid: None,
                payment_method: "cash".to_string(),
                notes: None,
                items: vec![],
            })
            .await
            .expect("create cash purchase");

        assert_eq!(cash.previous_balance, 1000);
        assert_eq!(cash.total, 1500);
        assert_eq!(cash.amount_paid, 1500);
        assert_eq!(supplier_balance(&pool, &s.id).await, 0);
    }

    #[tokio::test]
    async fn create_assigns_balance_instead_of_accumulating() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Accumulator").await;

        credit_purchase(&pool, &s, vec![purchase_item("Cable", 1, 1000)]).await;
        credit_purchase(&pool, &s, vec![purchase_item("Clamp", 1, 500)]).await;

        // The second purchase's total (1500) already contains the first
        // purchase's due. Accumulating would report 1000 + 1500 = 2500.
        assert_eq!(supplier_balance(&pool, &s.id).await, 1500);
    }

    #[tokio::test]
    async fn create_marks_every_absorbed_purchase_not_just_the_newest() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Legacy Book").await;

        // Pre-carry database shape: two open purchases neither carried.
        let older = raw_open_purchase(&pool, &s.id, &s.name, 1000, 0, 0, None).await;
        let newer = raw_open_purchase(&pool, &s.id, &s.name, 500, 0, 0, None).await;
        sqlx::query("UPDATE suppliers SET outstanding_balance = 1500 WHERE id = ?")
            .bind(&s.id)
            .execute(&pool)
            .await
            .unwrap();

        let leaf = credit_purchase(&pool, &s, vec![purchase_item("Grout", 1, 300)]).await;

        assert_eq!(leaf.previous_balance, 1500);
        assert_eq!(get_purchase(&pool, &older).await.carried_to_purchase_id, Some(leaf.id.clone()));
        assert_eq!(get_purchase(&pool, &newer).await.carried_to_purchase_id, Some(leaf.id.clone()));
        assert_eq!(supplier_balance(&pool, &s.id).await, 1800);
    }

    #[tokio::test]
    async fn mark_paid_on_a_carried_purchase_is_rejected() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Blocked Payee").await;
        let repo = SupplierPurchaseRepository::new(pool.clone());

        let absorbed = raw_open_purchase(&pool, &s.id, &s.name, 1000, 0, 0, None).await;
        sqlx::query("UPDATE suppliers SET outstanding_balance = 1000 WHERE id = ?")
            .bind(&s.id)
            .execute(&pool)
            .await
            .unwrap();
        let leaf = credit_purchase(&pool, &s, vec![purchase_item("Valve", 1, 200)]).await;

        // The absorbed purchase's money now lives in the leaf.
        let err = repo.mark_paid(&absorbed, 100, "cash").await.unwrap_err();
        match err {
            AppError::Validation(msg) => {
                assert!(msg.contains(&leaf.purchase_no), "message should name the successor: {}", msg);
            }
            other => panic!("expected Validation, got {:?}", other),
        }
        assert_eq!(supplier_balance(&pool, &s.id).await, 1200);

        // Paying the leaf works and decrements the balance.
        repo.mark_paid(&leaf.id, 1200, "cash").await.expect("pay leaf");
        assert_eq!(supplier_balance(&pool, &s.id).await, 0);
    }

    #[tokio::test]
    async fn balance_matches_leaf_due_after_a_chain_is_built() {
        let pool = test_pool().await;
        let s = make_supplier(&pool, "Chain Co").await;

        credit_purchase(&pool, &s, vec![purchase_item("A", 1, 1000)]).await;
        credit_purchase(&pool, &s, vec![purchase_item("B", 1, 500)]).await;
        credit_purchase(&pool, &s, vec![purchase_item("C", 1, 250)]).await;

        assert_eq!(supplier_balance(&pool, &s.id).await, 1750);
        assert_eq!(
            supplier_balance(&pool, &s.id).await,
            expected_leaf_supplier_balance(&pool, &s.id).await
        );
    }
}