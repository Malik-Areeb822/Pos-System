// src-tauri/src/services/returns.rs
use crate::DbPool;
use crate::error::AppError;
use crate::repositories::{ReturnRepository, CreateReturnInput};

pub async fn process_return(
    pool: &DbPool,
    invoice_id: &str,
    product_id: Option<String>,
    product_name: String,
    quantity: i64,
    unit: String,
    unit_price: i64,
    line_total: i64,
    reason: String,
    processed_by: String,
) -> Result<crate::repositories::returns::Return, AppError> {
    let repo = ReturnRepository::new(pool.clone());
    repo.create(CreateReturnInput {
        invoice_id: invoice_id.to_string(),
        product_id,
        product_name,
        quantity,
        unit,
        unit_price,
        line_total,
        reason,
        processed_by,
    }).await
}