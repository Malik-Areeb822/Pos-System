// src-tauri/src/repositories/cashiers.rs
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::error::AppError;

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Cashier {
    pub id: String,
    pub email: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub employee_id: Option<String>,
    pub status: String,
    pub roles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateCashierInput {
    pub email: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub employee_id: Option<String>,
    pub password: String,
}

pub struct CashierRepository {
    pool: SqlitePool,
}

impl CashierRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Cashier>, AppError> {
        let users = sqlx::query!(
            r#"SELECT u.id, u.email, u.full_name, u.phone, u.employee_id, u.status, u.created_at, u.updated_at
               FROM users u
               JOIN user_roles ur ON u.id = ur.user_id
               WHERE ur.role = 'cashier'
               ORDER BY u.full_name"#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut cashiers = Vec::new();
        for user in users {
            let user_id = user.id.clone();
            let roles_query = sqlx::query_scalar!("SELECT role FROM user_roles WHERE user_id = ?", user_id);
            let roles = roles_query.fetch_all(&self.pool).await?;
            cashiers.push(Cashier {
                id: user.id.expect("user id should not be null"),
                email: user.email,
                full_name: user.full_name,
                phone: user.phone,
                employee_id: user.employee_id,
                status: user.status,
                roles,
                created_at: user.created_at,
                updated_at: user.updated_at,
            });
        }
        Ok(cashiers)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Cashier>, AppError> {
        let user = sqlx::query!(
            r#"SELECT u.id, u.email, u.full_name, u.phone, u.employee_id, u.status, u.created_at, u.updated_at
               FROM users u
               JOIN user_roles ur ON u.id = ur.user_id
               WHERE u.id = ? AND ur.role = 'cashier'"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(user) = user {
            let user_id = user.id.clone();
            let roles_query = sqlx::query_scalar!("SELECT role FROM user_roles WHERE user_id = ?", user_id);
            let roles = roles_query.fetch_all(&self.pool).await?;
            Ok(Some(Cashier {
                id: user.id.expect("user id should not be null"),
                email: user.email,
                full_name: user.full_name,
                phone: user.phone,
                employee_id: user.employee_id,
                status: user.status,
                roles,
                created_at: user.created_at,
                updated_at: user.updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_status(&self, id: &str, status: &str) -> Result<Cashier, AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE users SET status = ?, updated_at = ? WHERE id = ?", status, now, id)
            .execute(&self.pool)
            .await?;
        
        self.get(id).await?.ok_or(AppError::NotFound("Cashier not found".into()))
    }

    pub async fn reset_password(&self, id: &str, new_password_hash: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?", new_password_hash, now, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}