// src-tauri/src/repositories/customers.rs
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::error::AppError;

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub outstanding_balance: i64,
    pub created_at: String,
    pub updated_at: String,
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
            r#"SELECT id, name, phone, email, address, outstanding_balance, created_at, updated_at FROM customers ORDER BY name"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(customers)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Customer>, AppError> {
        let customer = sqlx::query_as_unchecked!(
            Customer,
            r#"SELECT id, name, phone, email, address, outstanding_balance, created_at, updated_at FROM customers WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(customer)
    }

    pub async fn create(&self, input: CreateCustomerInput) -> Result<Customer, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query!(
            r#"INSERT INTO customers (id, name, phone, email, address, outstanding_balance, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, 0, ?, ?)"#,
            id, input.name, input.phone, input.email, input.address, now, now
        )
        .execute(&self.pool)
        .await?;

        self.get(&id).await?.ok_or(AppError::NotFound("Customer not found after creation".into()))
    }

    pub async fn update(&self, id: &str, input: CreateCustomerInput) -> Result<Customer, AppError> {
        let now = Utc::now().to_rfc3339();
        
        sqlx::query!(
            r#"UPDATE customers SET name = ?, phone = ?, email = ?, address = ?, updated_at = ? WHERE id = ?"#,
            input.name, input.phone, input.email, input.address, now, id
        )
        .execute(&self.pool)
        .await?;

        self.get(id).await?.ok_or(AppError::NotFound("Customer not found after update".into()))
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM customers WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_balance(&self, id: &str, delta: i64) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE customers SET outstanding_balance = outstanding_balance + ?, updated_at = ? WHERE id = ?", delta, now, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}