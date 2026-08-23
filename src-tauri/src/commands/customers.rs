// src-tauri/src/commands/customers.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::repositories::{CustomerRepository, CreateCustomerInput, Customer};
use crate::DbPool;
use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateCustomerInputCmd {
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCustomerInput {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

#[tauri::command]
pub async fn list_customers(db: State<'_, crate::database::Db>, _auth: Option<String>) -> Result<Vec<Customer>, AppError> {
    let pool = db.pool().await;
    let repo = CustomerRepository::new(pool);
    repo.list().await
}

#[tauri::command]
pub async fn get_customer(db: State<'_, crate::database::Db>, id: String, _auth: Option<String>) -> Result<Option<Customer>, AppError> {
    let pool = db.pool().await;
    let repo = CustomerRepository::new(pool);
    repo.get(&id).await
}

#[tauri::command]
pub async fn create_customer(app: AppHandle, db: State<'_, crate::database::Db>, input: CreateCustomerInputCmd, auth_header: Option<String>) -> Result<Customer, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CustomerRepository::new(pool);
    let create_input = crate::repositories::CreateCustomerInput {
        name: input.name,
        phone: input.phone,
        email: input.email,
        address: input.address,
    };
    let customer = repo.create(create_input).await?;
    crate::events::emit_customers_changed(&app).await;
    Ok(customer)
}

#[tauri::command]
pub async fn update_customer(app: AppHandle, db: State<'_, crate::database::Db>, input: UpdateCustomerInput, auth_header: Option<String>) -> Result<Customer, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CustomerRepository::new(pool);
    let create_input = crate::repositories::CreateCustomerInput {
        name: input.name,
        phone: input.phone,
        email: input.email,
        address: input.address,
    };
    let customer = repo.update(&input.id, create_input).await?;
    crate::events::emit_customers_changed(&app).await;
    Ok(customer)
}

#[tauri::command]
pub async fn delete_customer(app: AppHandle, db: State<'_, crate::database::Db>, id: String, auth_header: Option<String>) -> Result<(), AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = CustomerRepository::new(pool);
    repo.delete(&id).await?;
    crate::events::emit_customers_changed(&app).await;
    Ok(())
}