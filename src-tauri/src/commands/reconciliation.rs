// src-tauri/src/commands/reconciliation.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::error::AppError;

#[tauri::command]
pub async fn reconcile_balances(app: AppHandle, db: State<'_, crate::database::Db>, auth_header: Option<String>) -> Result<i64, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let n = crate::services::reconciliation::reconcile_balances(&pool).await?;
    crate::events::emit_customers_changed(&app).await;
    Ok(n)
}
