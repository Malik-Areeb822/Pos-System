// src-tauri/src/repositories/customers.rs
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::Utc;
use crate::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub outstanding_balance: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub invoice_count: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateCustomerInput {
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

pub struct CustomerRepository {
    pool: SqlitePool,
}

impl CustomerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Customer>, AppError> {
        let customers = sqlx::query_as_unchecked!(
            Customer,
            r#"SELECT 
                c.id, c.name, c.phone, c.email, c.address, c.outstanding_balance, c.created_at, c.updated_at,
                COUNT(i.id) AS invoice_count
               FROM customers c
               LEFT JOIN invoices i ON i.customer_id = c.id
               GROUP BY c.id
               ORDER BY c.name"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(customers)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Customer>, AppError> {
        let customer = sqlx::query_as_unchecked!(
            Customer,
            r#"SELECT 
                c.id, c.name, c.phone, c.email, c.address, c.outstanding_balance, c.created_at, c.updated_at,
                COUNT(i.id) AS invoice_count
               FROM customers c
               LEFT JOIN invoices i ON i.customer_id = c.id
               WHERE c.id = ?
               GROUP BY c.id"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(customer)
    }

    pub async fn find_duplicate(
        &self,
        exclude_id: Option<&str>,
        name: &str,
        phone: Option<&str>,
    ) -> Result<Option<Customer>, AppError> {
        let norm_name = name.trim().to_lowercase();
        let norm_phone = phone.unwrap_or("").trim().to_string();

        let customer = if let Some(id) = exclude_id {
            sqlx::query_as_unchecked!(
                Customer,
                r#"SELECT id, name, phone, email, address, outstanding_balance, created_at, updated_at,
                          0 AS invoice_count
                   FROM customers
                   WHERE id != ?
                     AND lower(trim(name)) = ?
                     AND trim(COALESCE(phone, '')) = ?
                   LIMIT 1"#,
                id,
                norm_name,
                norm_phone
            )
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_as_unchecked!(
                Customer,
                r#"SELECT id, name, phone, email, address, outstanding_balance, created_at, updated_at,
                          0 AS invoice_count
                   FROM customers
                   WHERE lower(trim(name)) = ?
                     AND trim(COALESCE(phone, '')) = ?
                   LIMIT 1"#,
                norm_name,
                norm_phone
            )
            .fetch_optional(&self.pool)
            .await?
        };

        Ok(customer)
    }

    pub async fn create(&self, input: CreateCustomerInput) -> Result<Customer, AppError> {
        if self.find_duplicate(None, &input.name, input.phone.as_deref()).await?.is_some() {
            return Err(AppError::Conflict(
                "A customer with this name and phone already exists. Open the existing customer instead.".into(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query!(
            r#"INSERT INTO customers (id, name, phone, email, address, outstanding_balance, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, 0, ?, ?)"#,
            id, input.name, input.phone, input.email, input.address, now, now
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {},
            Err(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
                return Err(AppError::Conflict(
                    "A customer with this name and phone already exists. Open the existing customer instead.".into(),
                ));
            }
            Err(e) => return Err(e.into()),
        }

        self.get(&id).await?.ok_or(AppError::NotFound("Customer not found after creation".into()))
    }

    pub async fn update(&self, id: &str, input: CreateCustomerInput) -> Result<Customer, AppError> {
        if self.find_duplicate(Some(id), &input.name, input.phone.as_deref()).await?.is_some() {
            return Err(AppError::Conflict(
                "A customer with this name and phone already exists. Open the existing customer instead.".into(),
            ));
        }

        let now = Utc::now().to_rfc3339();
        
        let result = sqlx::query!(
            r#"UPDATE customers SET name = ?, phone = ?, email = ?, address = ?, updated_at = ? WHERE id = ?"#,
            input.name, input.phone, input.email, input.address, now, id
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {},
            Err(sqlx::Error::Database(ref db_err)) if db_err.is_unique_violation() => {
                return Err(AppError::Conflict(
                    "A customer with this name and phone already exists. Open the existing customer instead.".into(),
                ));
            }
            Err(e) => return Err(e.into()),
        }

        self.get(id).await?.ok_or(AppError::NotFound("Customer not found after update".into()))
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM customers WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[tokio::test]
    async fn distinct_customers_can_be_created() {
        let pool = test_pool().await;
        let repo = CustomerRepository::new(pool);

        let c1 = repo.create(CreateCustomerInput {
            name: "Bilal Khan".into(),
            phone: Some("03001234567".into()),
            email: None,
            address: None,
        }).await.expect("create c1");
        assert_eq!(c1.name, "Bilal Khan");

        // Same phone, different name -> allowed
        let c2 = repo.create(CreateCustomerInput {
            name: "Tariq Mahmood".into(),
            phone: Some("03001234567".into()),
            email: None,
            address: None,
        }).await.expect("create c2 with same phone");
        assert_eq!(c2.name, "Tariq Mahmood");

        // Same name, different phone -> allowed
        let c3 = repo.create(CreateCustomerInput {
            name: "Bilal Khan".into(),
            phone: Some("03219876543".into()),
            email: None,
            address: None,
        }).await.expect("create c3 with different phone");
        assert_eq!(c3.name, "Bilal Khan");

        // Null phone, different names -> allowed
        let c4 = repo.create(CreateCustomerInput {
            name: "Customer Without Phone 1".into(),
            phone: None,
            email: None,
            address: None,
        }).await.expect("create c4 without phone");
        assert_eq!(c4.name, "Customer Without Phone 1");

        let c5 = repo.create(CreateCustomerInput {
            name: "Customer Without Phone 2".into(),
            phone: None,
            email: None,
            address: None,
        }).await.expect("create c5 without phone");
        assert_eq!(c5.name, "Customer Without Phone 2");
    }

    #[tokio::test]
    async fn duplicate_customer_creation_returns_conflict() {
        let pool = test_pool().await;
        let repo = CustomerRepository::new(pool);

        repo.create(CreateCustomerInput {
            name: "Hamza Tariq".into(),
            phone: Some("03335555555".into()),
            email: None,
            address: None,
        }).await.expect("create initial");

        // Exact duplicate
        let err1 = repo.create(CreateCustomerInput {
            name: "Hamza Tariq".into(),
            phone: Some("03335555555".into()),
            email: None,
            address: None,
        }).await.unwrap_err();
        match err1 {
            AppError::Conflict(msg) => assert!(msg.contains("already exists")),
            other => panic!("expected AppError::Conflict, got {:?}", other),
        }

        // Case-insensitive and trimmed duplicate
        let err2 = repo.create(CreateCustomerInput {
            name: "  hamza TARIQ  ".into(),
            phone: Some(" 03335555555  ".into()),
            email: None,
            address: None,
        }).await.unwrap_err();
        match err2 {
            AppError::Conflict(msg) => assert!(msg.contains("already exists")),
            other => panic!("expected AppError::Conflict, got {:?}", other),
        }

        // Duplicate with no phone (None vs whitespace)
        repo.create(CreateCustomerInput {
            name: "No Phone Customer".into(),
            phone: None,
            email: None,
            address: None,
        }).await.expect("create no phone");

        let err3 = repo.create(CreateCustomerInput {
            name: "  no phone customer  ".into(),
            phone: Some("   ".into()),
            email: None,
            address: None,
        }).await.unwrap_err();
        match err3 {
            AppError::Conflict(msg) => assert!(msg.contains("already exists")),
            other => panic!("expected AppError::Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn update_to_duplicate_returns_conflict() {
        let pool = test_pool().await;
        let repo = CustomerRepository::new(pool);

        let c1 = repo.create(CreateCustomerInput {
            name: "Customer One".into(),
            phone: Some("03001111111".into()),
            email: None,
            address: None,
        }).await.expect("create c1");

        let c2 = repo.create(CreateCustomerInput {
            name: "Customer Two".into(),
            phone: Some("03002222222".into()),
            email: None,
            address: None,
        }).await.expect("create c2");

        // Self-update without changes or updating other fields -> should succeed
        let updated = repo.update(&c1.id, CreateCustomerInput {
            name: "Customer One".into(),
            phone: Some("03001111111".into()),
            email: Some("c1@example.com".into()),
            address: Some("Lahore".into()),
        }).await.expect("self update should succeed");
        assert_eq!(updated.email.as_deref(), Some("c1@example.com"));

        // Updating c2 to match c1 -> should return Conflict
        let err = repo.update(&c2.id, CreateCustomerInput {
            name: "  customer ONE ".into(),
            phone: Some(" 03001111111 ".into()),
            email: None,
            address: None,
        }).await.unwrap_err();
        match err {
            AppError::Conflict(msg) => assert!(msg.contains("already exists")),
            other => panic!("expected AppError::Conflict, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn migration_018_merges_duplicates_and_preserves_balances_and_invoices() {
        let pool = test_pool().await;

        // Temporarily drop unique index so we can inject legacy duplicates to test the migration merge SQL
        sqlx::query("DROP INDEX IF EXISTS ux_customers_name_phone")
            .execute(&pool)
            .await
            .expect("drop index for test setup");

        // Insert duplicate customer rows directly
        let id1 = "cust-keeper";
        let id2 = "cust-dupe1";
        let id3 = "cust-dupe2";

        sqlx::query(
            "INSERT INTO customers (id, name, phone, outstanding_balance, created_at, updated_at)
             VALUES (?, 'Muhammad Usman', '03007654321', 500, '2026-01-01T10:00:00Z', '2026-01-01T10:00:00Z')"
        ).bind(id1).execute(&pool).await.expect("insert keeper");

        sqlx::query(
            "INSERT INTO customers (id, name, phone, outstanding_balance, created_at, updated_at)
             VALUES (?, '  muhammad usman  ', ' 03007654321 ', 300, '2026-01-02T10:00:00Z', '2026-01-02T10:00:00Z')"
        ).bind(id2).execute(&pool).await.expect("insert dupe1");

        sqlx::query(
            "INSERT INTO customers (id, name, phone, outstanding_balance, created_at, updated_at)
             VALUES (?, 'MUHAMMAD USMAN', '03007654321', 200, '2026-01-03T10:00:00Z', '2026-01-03T10:00:00Z')"
        ).bind(id3).execute(&pool).await.expect("insert dupe2");

        // Insert invoices attached to each duplicate
        sqlx::query(
            "INSERT INTO invoices (id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, created_at, updated_at)
             VALUES ('inv-1', 'INV-TEST-001', ?, 'Muhammad Usman', 500, 0, 500, 0, '2026-01-01T10:00:00Z', '2026-01-01T10:00:00Z')"
        ).bind(id1).execute(&pool).await.expect("insert inv1");

        sqlx::query(
            "INSERT INTO invoices (id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, created_at, updated_at)
             VALUES ('inv-2', 'INV-TEST-002', ?, 'muhammad usman', 300, 0, 300, 0, '2026-01-02T10:00:00Z', '2026-01-02T10:00:00Z')"
        ).bind(id2).execute(&pool).await.expect("insert inv2");

        sqlx::query(
            "INSERT INTO invoices (id, invoice_no, customer_id, customer_name, subtotal, discount, total, amount_paid, created_at, updated_at)
             VALUES ('inv-3', 'INV-TEST-003', ?, 'MUHAMMAD USMAN', 200, 0, 200, 0, '2026-01-03T10:00:00Z', '2026-01-03T10:00:00Z')"
        ).bind(id3).execute(&pool).await.expect("insert inv3");

        // Execute the migration 018 script directly inside a transaction
        let migration_sql = include_str!("../database/migrations/018_customer_unique.sql");
        let mut tx = pool.begin().await.expect("begin migration tx");
        for statement in migration_sql.split(';') {
            let stmt = statement.trim();
            if !stmt.is_empty() {
                sqlx::query(stmt).execute(&mut *tx).await.expect("execute migration statement");
            }
        }
        tx.commit().await.expect("commit migration tx");

        // Verify:
        // 1. Only 1 customer row remains for Muhammad Usman
        let remaining_customers: Vec<(String, i64)> = sqlx::query_as(
            "SELECT id, outstanding_balance FROM customers WHERE lower(trim(name)) = 'muhammad usman'"
        ).fetch_all(&pool).await.expect("query remaining");

        assert_eq!(remaining_customers.len(), 1, "only keeper customer should remain");
        assert_eq!(remaining_customers[0].0, id1, "keeper id should match earliest created_at");
        assert_eq!(remaining_customers[0].1, 1000, "balance must be sum of all merged rows (500 + 300 + 200)");

        // 2. All 3 invoices are repointed to the keeper
        let inv1_cust: String = sqlx::query_scalar("SELECT customer_id FROM invoices WHERE id = 'inv-1'")
            .fetch_one(&pool).await.expect("inv1");
        let inv2_cust: String = sqlx::query_scalar("SELECT customer_id FROM invoices WHERE id = 'inv-2'")
            .fetch_one(&pool).await.expect("inv2");
        let inv3_cust: String = sqlx::query_scalar("SELECT customer_id FROM invoices WHERE id = 'inv-3'")
            .fetch_one(&pool).await.expect("inv3");

        assert_eq!(inv1_cust, id1);
        assert_eq!(inv2_cust, id1);
        assert_eq!(inv3_cust, id1);

        // 3. Unique index is in place and blocks new duplicates at SQL level
        let dup_insert = sqlx::query(
            "INSERT INTO customers (id, name, phone, outstanding_balance, created_at, updated_at)
             VALUES ('cust-new-dup', 'muhammad usman', '03007654321', 0, '2026-01-04T10:00:00Z', '2026-01-04T10:00:00Z')"
        ).execute(&pool).await;

        assert!(dup_insert.is_err(), "unique index must reject duplicate insert");
    }
}