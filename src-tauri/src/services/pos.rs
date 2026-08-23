// src-tauri/src/services/pos.rs
use crate::DbPool;
use crate::error::AppError;
use crate::repositories::{ProductRepository, CustomerRepository};

pub async fn get_pos_products(pool: &DbPool) -> Result<Vec<crate::repositories::products::Product>, AppError> {
    let repo = ProductRepository::new(pool.clone());
    repo.list().await
}

pub async fn get_pos_customers(pool: &DbPool) -> Result<Vec<crate::repositories::customers::Customer>, AppError> {
    let repo = CustomerRepository::new(pool.clone());
    repo.list().await
}