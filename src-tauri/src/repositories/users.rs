// src-tauri/src/repositories/users.rs
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::error::AppError;

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub employee_id: Option<String>,
    pub status: String,
    pub roles: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateUserInput {
    pub email: String,
    pub password_hash: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub employee_id: Option<String>,
    pub status: String,
    pub roles: Vec<String>,
}

pub struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query!(
            r#"SELECT id, email, password_hash, full_name, phone, employee_id, status, created_at, updated_at FROM users WHERE email = ?"#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(user) = user {
            let user_id = user.id.clone();
            let roles_query = sqlx::query_scalar!("SELECT role FROM user_roles WHERE user_id = ?", user_id);
            let roles = roles_query.fetch_all(&self.pool).await?;
            Ok(Some(User {
                id: user.id.expect("user id should not be null"),
                email: user.email,
                password_hash: user.password_hash,
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

    pub async fn get_by_id(&self, id: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query!(
            r#"SELECT id, email, password_hash, full_name, phone, employee_id, status, created_at, updated_at FROM users WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(user) = user {
            let user_id = user.id.clone();
            let roles_query = sqlx::query_scalar!("SELECT role FROM user_roles WHERE user_id = ?", user_id);
            let roles = roles_query.fetch_all(&self.pool).await?;
            Ok(Some(User {
                id: user.id.expect("user id should not be null"),
                email: user.email,
                password_hash: user.password_hash,
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

    pub async fn count(&self) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    pub async fn create(&self, input: CreateUserInput) -> Result<User, AppError> {
        let mut tx = self.pool.begin().await?;
        
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query!(
            r#"INSERT INTO users (id, email, password_hash, full_name, phone, employee_id, status, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            id, input.email, input.password_hash, input.full_name, input.phone, input.employee_id, input.status, now, now
        )
        .execute(&mut *tx)
        .await?;

        for role in &input.roles {
            sqlx::query!("INSERT INTO user_roles (user_id, role) VALUES (?, ?)", id, role)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        self.get_by_id(&id).await?.ok_or(AppError::NotFound("User not found after creation".into()))
    }

    pub async fn update_password(&self, id: &str, new_password_hash: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?", new_password_hash, now, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_status(&self, id: &str, status: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE users SET status = ?, updated_at = ? WHERE id = ?", status, now, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}