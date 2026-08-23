// src-tauri/src/commands/returns.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::repositories::{ReturnRepository, CreateReturnInput, Return};
use crate::DbPool;
use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ListReturnsInput {
    pub invoice_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateReturnInputCmd {
    pub invoice_id: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
    pub reason: String,
}

#[tauri::command]
pub async fn list_returns(db: State<'_, crate::database::Db>, input: ListReturnsInput, _auth: Option<String>) -> Result<Vec<Return>, AppError> {
    let pool = db.pool().await;
    let repo = ReturnRepository::new(pool);
    repo.list(input.invoice_id).await
}

#[tauri::command]
pub async fn create_return(app: AppHandle, db: State<'_, crate::database::Db>, input: CreateReturnInputCmd, auth_header: Option<String>) -> Result<Return, AppError> {
    let auth_header_clone = auth_header.clone();
    require_cashier_or_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = ReturnRepository::new(pool);
    
    // Get current user for processed_by
    let claims = crate::auth::middleware::get_current_user(&app, auth_header_clone).await?;
    
    let ret = repo.create(CreateReturnInput {
        invoice_id: input.invoice_id,
        product_id: input.product_id,
        product_name: input.product_name,
        quantity: input.quantity,
        unit: input.unit,
        unit_price: input.unit_price,
        line_total: input.line_total,
        reason: input.reason,
        processed_by: claims.sub,
    }).await?;
    
    crate::events::emit_invoices_changed(&app).await;
    crate::events::emit_products_changed(&app).await;
    Ok(ret)
}