// src-tauri/src/commands/cashiers.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::{require_admin, get_current_user};
use crate::repositories::{CashierRepository, Cashier};
use crate::auth::password::hash_password;
use crate::DbPool;
use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ApproveCashierInput {
    pub id: String,
}

#[derive(Deserialize)]
pub struct RejectCashierInput {
    pub id: String,
}

#[derive(Deserialize)]
pub struct SuspendCashierInput {
    pub id: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordInput {
    pub id: String,
    pub new_password: String,
}

#[tauri::command]
pub async fn list_cashiers(app: AppHandle, db: State<'_, crate::database::Db>, auth_header: Option<String>) -> Result<Vec<Cashier>, AppError> {
    require_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CashierRepository::new(pool);
    repo.list().await
}

#[tauri::command]
pub async fn approve_cashier(app: AppHandle, db: State<'_, crate::database::Db>, input: ApproveCashierInput, auth_header: Option<String>) -> Result<Cashier, AppError> {
    require_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CashierRepository::new(pool);
    let cashier = repo.update_status(&input.id, "approved").await?;
    crate::events::emit_cashiers_changed(&app).await;
    Ok(cashier)
}

#[tauri::command]
pub async fn reject_cashier(app: AppHandle, db: State<'_, crate::database::Db>, input: RejectCashierInput, auth_header: Option<String>) -> Result<Cashier, AppError> {
    require_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CashierRepository::new(pool);
    let cashier = repo.update_status(&input.id, "rejected").await?;
    crate::events::emit_cashiers_changed(&app).await;
    Ok(cashier)
}

#[tauri::command]
pub async fn suspend_cashier(app: AppHandle, db: State<'_, crate::database::Db>, input: SuspendCashierInput, auth_header: Option<String>) -> Result<Cashier, AppError> {
    require_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CashierRepository::new(pool);
    let cashier = repo.update_status(&input.id, "suspended").await?;
    crate::events::emit_cashiers_changed(&app).await;
    Ok(cashier)
}

#[tauri::command]
pub async fn reset_password(app: AppHandle, db: State<'_, crate::database::Db>, input: ResetPasswordInput, auth_header: Option<String>) -> Result<(), AppError> {
    require_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CashierRepository::new(pool);
    let password_hash = hash_password(&input.new_password)?;
    repo.reset_password(&input.id, &password_hash).await?;
    crate::events::emit_cashiers_changed(&app).await;
    Ok(())
}