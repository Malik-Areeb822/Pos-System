// src-tauri/src/commands/products.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::repositories::{ProductRepository, CreateProductInput, Product};
use crate::DbPool;
use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ListProductsInput {
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateProductInputCmd {
    pub name: String,
    pub sku: Option<String>,
    pub category: String,
    pub description: String,
    pub color: Option<String>,
    pub size: Option<String>,
    pub finish: Option<String>,
    pub unit: String,
    pub price: i64,
    pub pieces_per_carton: Option<i64>,
    pub stock_qty: i64,
    pub low_stock_threshold: i64,
    pub image_url: Option<String>,
    pub is_published: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateProductInput {
    pub id: String,
    pub name: String,
    pub sku: Option<String>,
    pub category: String,
    pub description: String,
    pub color: Option<String>,
    pub size: Option<String>,
    pub finish: Option<String>,
    pub unit: String,
    pub price: i64,
    pub pieces_per_carton: Option<i64>,
    pub stock_qty: i64,
    pub low_stock_threshold: i64,
    pub image_url: Option<String>,
    pub is_published: Option<bool>,
}

#[tauri::command]
pub async fn list_products(pool: State<'_, DbPool>, _input: ListProductsInput, _auth: Option<String>) -> Result<Vec<Product>, AppError> {
    let repo = ProductRepository::new(pool.inner().clone());
    repo.list().await
}

#[tauri::command]
pub async fn get_product(pool: State<'_, DbPool>, id: String, _auth: Option<String>) -> Result<Option<Product>, AppError> {
    let repo = ProductRepository::new(pool.inner().clone());
    repo.get(&id).await
}

#[tauri::command]
pub async fn create_product(app: AppHandle, pool: State<'_, DbPool>, input: CreateProductInputCmd, auth_header: Option<String>) -> Result<Product, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let repo = ProductRepository::new(pool.inner().clone());
    let create_input = crate::repositories::CreateProductInput {
        name: input.name,
        sku: input.sku,
        category: input.category,
        description: Some(input.description),
        color: input.color,
        size: input.size,
        finish: input.finish,
        unit: input.unit,
        price: input.price,
        pieces_per_carton: input.pieces_per_carton,
        stock_qty: input.stock_qty,
        low_stock_threshold: input.low_stock_threshold,
        image_url: input.image_url,
        is_published: input.is_published.unwrap_or(true) as i64,
    };
    let product = repo.create(create_input).await?;
    crate::events::emit_products_changed(&app).await;
    Ok(product)
}

#[tauri::command]
pub async fn update_product(app: AppHandle, pool: State<'_, DbPool>, input: UpdateProductInput, auth_header: Option<String>) -> Result<Product, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let repo = ProductRepository::new(pool.inner().clone());
    let create_input = crate::repositories::CreateProductInput {
        name: input.name,
        sku: input.sku,
        category: input.category,
        description: Some(input.description),
        color: input.color,
        size: input.size,
        finish: input.finish,
        unit: input.unit,
        price: input.price,
        pieces_per_carton: input.pieces_per_carton,
        stock_qty: input.stock_qty,
        low_stock_threshold: input.low_stock_threshold,
        image_url: input.image_url,
        is_published: input.is_published.unwrap_or(true) as i64,
    };
    let product = repo.update(&input.id, create_input).await?;
    crate::events::emit_products_changed(&app).await;
    Ok(product)
}

#[tauri::command]
pub async fn delete_product(app: AppHandle, pool: State<'_, DbPool>, id: String, auth_header: Option<String>) -> Result<(), AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let repo = ProductRepository::new(pool.inner().clone());
    repo.delete(&id).await?;
    crate::events::emit_products_changed(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn import_products(app: AppHandle, pool: State<'_, DbPool>, csv_content: String, auth_header: Option<String>) -> Result<crate::services::inventory::CsvImportResult, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let result = crate::services::inventory::import_products_from_csv(pool.inner(), &csv_content).await?;
    crate::events::emit_products_changed(&app).await;
    Ok(result)
}

#[tauri::command]
pub async fn export_products(pool: State<'_, DbPool>, _auth: Option<String>) -> Result<Vec<Product>, AppError> {
    let repo = ProductRepository::new(pool.inner().clone());
    repo.list().await
}