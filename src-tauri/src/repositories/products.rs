// src-tauri/src/repositories/products.rs
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::error::AppError;

#[derive(Debug, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub sku: Option<String>,
    pub category: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub size: Option<String>,
    pub finish: Option<String>,
    /// Supplier/company name; surfaced for sanitary-ware in the UI.
    pub company: Option<String>,
    pub unit: String,
    pub price: i64,
    pub purchase_price: i64,
    pub pieces_per_carton: Option<i64>,
    pub area_per_tile: Option<f64>,
    pub stock_qty: i64,
    pub low_stock_threshold: i64,
    pub image_url: Option<String>,
    pub is_published: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateProductInput {
    pub name: String,
    pub sku: Option<String>,
    pub category: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub size: Option<String>,
    pub finish: Option<String>,
    pub company: Option<String>,
    pub unit: String,
    pub price: i64,
    pub purchase_price: i64,
    pub pieces_per_carton: Option<i64>,
    pub area_per_tile: Option<f64>,
    pub stock_qty: i64,
    pub low_stock_threshold: i64,
    pub image_url: Option<String>,
    pub is_published: i64,
}

pub struct ProductRepository {
    pool: SqlitePool,
}

impl ProductRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Product>, AppError> {
        let products = sqlx::query_as_unchecked!(
            Product,
            r#"SELECT id, name, sku, category, description, color, size, finish, company, unit, price, purchase_price, pieces_per_carton, area_per_tile, stock_qty, low_stock_threshold, image_url, is_published, created_at, updated_at FROM products ORDER BY name"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(products)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Product>, AppError> {
        let product = sqlx::query_as_unchecked!(
            Product,
            r#"SELECT id, name, sku, category, description, color, size, finish, company, unit, price, purchase_price, pieces_per_carton, area_per_tile, stock_qty, low_stock_threshold, image_url, is_published, created_at, updated_at FROM products WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(product)
    }

    pub async fn create(&self, input: CreateProductInput) -> Result<Product, AppError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let description = input.description.unwrap_or_default();

        sqlx::query!(
            r#"INSERT INTO products (id, name, sku, category, description, color, size, finish, company, unit, price, purchase_price, pieces_per_carton, area_per_tile, stock_qty, low_stock_threshold, image_url, is_published, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            id, input.name, input.sku, input.category, description,
            input.color, input.size, input.finish, input.company, input.unit, input.price, input.purchase_price, input.pieces_per_carton, input.area_per_tile,
            input.stock_qty, input.low_stock_threshold, input.image_url, input.is_published, now, now
        )
        .execute(&self.pool)
        .await?;

        self.get(&id).await?.ok_or(AppError::NotFound("Product not found after creation".into()))
    }

    pub async fn update(&self, id: &str, input: CreateProductInput) -> Result<Product, AppError> {
        let now = Utc::now().to_rfc3339();
        let description = input.description.unwrap_or_default();
        
        sqlx::query!(
            r#"UPDATE products SET name = ?, sku = ?, category = ?, description = ?, color = ?, size = ?, finish = ?, company = ?, unit = ?, price = ?, purchase_price = ?, pieces_per_carton = ?, area_per_tile = ?, stock_qty = ?, low_stock_threshold = ?, image_url = ?, is_published = ?, updated_at = ? WHERE id = ?"#,
            input.name, input.sku, input.category, description,
            input.color, input.size, input.finish, input.company, input.unit, input.price, input.purchase_price, input.pieces_per_carton, input.area_per_tile,
            input.stock_qty, input.low_stock_threshold, input.image_url, input.is_published, now, id
        )
        .execute(&self.pool)
        .await?;

        self.get(id).await?.ok_or(AppError::NotFound("Product not found after update".into()))
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query!("DELETE FROM products WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_stock(&self, id: &str, delta: i64) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query!("UPDATE products SET stock_qty = stock_qty + ?, updated_at = ? WHERE id = ?", delta, now, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn low_stock_count(&self) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM products WHERE stock_qty <= low_stock_threshold AND is_published = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }
}