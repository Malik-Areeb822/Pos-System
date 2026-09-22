// src-tauri/src/commands/suppliers.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::repositories::suppliers::{
    SupplierRepository, Supplier, CreateSupplierInput,
    SupplierPurchaseRepository, SupplierPurchase, SupplierPurchaseItem,
    CreatePurchaseInput, CreatePurchaseItemInput,
};
use crate::error::AppError;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateSupplierInputCmd {
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSupplierInputCmd {
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub company: Option<String>,
    pub address: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct ListSupplierPurchasesInput {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub query: Option<String>,
    pub supplier_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CreatePurchaseItemInputCmd {
    pub description: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
}

#[derive(Deserialize)]
pub struct CreatePurchaseInputCmd {
    pub supplier_id: String,
    pub supplier_name: String,
    pub subtotal: i64,
    pub discount: i64,
    pub total: i64,
    pub amount_paid: Option<i64>,
    pub payment_method: String,
    pub notes: Option<String>,
    pub items: Vec<CreatePurchaseItemInputCmd>,
}

#[derive(Deserialize)]
pub struct MarkPurchasePaidInput {
    pub id: String,
    pub amount: i64,
    pub payment_method: String,
}

#[tauri::command]
pub async fn list_suppliers(db: State<'_, crate::database::Db>, _auth: Option<String>) -> Result<Vec<Supplier>, AppError> {
    let pool = db.pool().await;
    let repo = SupplierRepository::new(pool);
    repo.list().await
}

#[tauri::command]
pub async fn get_supplier(db: State<'_, crate::database::Db>, id: String, _auth: Option<String>) -> Result<Option<Supplier>, AppError> {
    let pool = db.pool().await;
    let repo = SupplierRepository::new(pool);
    repo.get(&id).await
}

#[tauri::command]
pub async fn create_supplier(
    app: AppHandle,
    db: State<'_, crate::database::Db>,
    input: CreateSupplierInputCmd,
    auth_header: Option<String>,
) -> Result<Supplier, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Supplier name is required".into()));
    }
    let pool = db.pool().await;
    let repo = SupplierRepository::new(pool);
    let create_input = CreateSupplierInput {
        name: input.name.trim().to_string(),
        phone: input.phone,
        email: input.email,
        company: input.company,
        address: input.address,
        notes: input.notes,
    };
    let supplier = repo.create(create_input).await?;
    crate::events::emit_suppliers_changed(&app).await;
    Ok(supplier)
}

#[tauri::command]
pub async fn update_supplier(
    app: AppHandle,
    db: State<'_, crate::database::Db>,
    input: UpdateSupplierInputCmd,
    auth_header: Option<String>,
) -> Result<Supplier, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Supplier name is required".into()));
    }
    let pool = db.pool().await;
    let repo = SupplierRepository::new(pool);
    let update_input = CreateSupplierInput {
        name: input.name.trim().to_string(),
        phone: input.phone,
        email: input.email,
        company: input.company,
        address: input.address,
        notes: input.notes,
    };
    let supplier = repo.update(&input.id, update_input).await?;
    crate::events::emit_suppliers_changed(&app).await;
    Ok(supplier)
}

#[tauri::command]
pub async fn delete_supplier(
    app: AppHandle,
    db: State<'_, crate::database::Db>,
    id: String,
    auth_header: Option<String>,
) -> Result<(), AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    let pool = db.pool().await;
    let repo = SupplierRepository::new(pool);
    repo.delete(&id).await?;
    crate::events::emit_suppliers_changed(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn list_supplier_purchases(
    db: State<'_, crate::database::Db>,
    input: ListSupplierPurchasesInput,
    _auth: Option<String>,
) -> Result<Vec<SupplierPurchase>, AppError> {
    let pool = db.pool().await;
    let repo = SupplierPurchaseRepository::new(pool);
    repo.list(
        input.limit.unwrap_or(50),
        input.offset.unwrap_or(0),
        input.query.as_deref(),
        input.supplier_id.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn get_supplier_purchase(
    db: State<'_, crate::database::Db>,
    id: String,
    _auth: Option<String>,
) -> Result<Option<SupplierPurchase>, AppError> {
    let pool = db.pool().await;
    let repo = SupplierPurchaseRepository::new(pool);
    repo.get(&id).await
}

#[tauri::command]
pub async fn get_supplier_purchase_with_items(
    db: State<'_, crate::database::Db>,
    id: String,
    _auth: Option<String>,
) -> Result<Option<(SupplierPurchase, Vec<SupplierPurchaseItem>)>, AppError> {
    let pool = db.pool().await;
    let repo = SupplierPurchaseRepository::new(pool);
    repo.get_with_items(&id).await
}

#[tauri::command]
pub async fn create_supplier_purchase(
    app: AppHandle,
    db: State<'_, crate::database::Db>,
    input: CreatePurchaseInputCmd,
    auth_header: Option<String>,
) -> Result<SupplierPurchase, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;

    if input.discount < 0 {
        return Err(AppError::Validation("Discount cannot be negative".into()));
    }
    if input.discount > input.subtotal {
        return Err(AppError::Validation(
            "Discount cannot exceed subtotal".into(),
        ));
    }
    if input.subtotal < 0 || input.total < 0 {
        return Err(AppError::Validation("Purchase amounts cannot be negative".into()));
    }
    if let Some(paid) = input.amount_paid {
        if paid < 0 {
            return Err(AppError::Validation("Amount paid cannot be negative".into()));
        }
    }
    if input.items.is_empty() {
        return Err(AppError::Validation("Purchase must have at least one line item".into()));
    }
    for item in &input.items {
        if item.quantity <= 0 {
            return Err(AppError::Validation(format!("{}: quantity must be at least 1", item.description)));
        }
        if item.unit_price < 0 || item.line_total < 0 {
            return Err(AppError::Validation(format!("{}: amounts cannot be negative", item.description)));
        }
    }

    let pool = db.pool().await;
    let repo = SupplierPurchaseRepository::new(pool);
    let purchase_input = CreatePurchaseInput {
        supplier_id: input.supplier_id,
        supplier_name: input.supplier_name,
        subtotal: input.subtotal,
        discount: input.discount,
        total: input.total,
        amount_paid: input.amount_paid,
        payment_method: input.payment_method,
        notes: input.notes,
        items: input.items.into_iter().map(|item| CreatePurchaseItemInput {
            description: item.description,
            quantity: item.quantity,
            unit: item.unit,
            unit_price: item.unit_price,
            line_total: item.line_total,
        }).collect(),
    };

    let purchase = repo.create(purchase_input).await?;
    crate::events::emit_suppliers_changed(&app).await;
    Ok(purchase)
}

#[tauri::command]
pub async fn mark_supplier_purchase_paid(
    app: AppHandle,
    db: State<'_, crate::database::Db>,
    input: MarkPurchasePaidInput,
    auth_header: Option<String>,
) -> Result<SupplierPurchase, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    if input.amount <= 0 {
        return Err(AppError::Validation("Payment amount must be greater than zero".into()));
    }

    let pool = db.pool().await;
    let repo = SupplierPurchaseRepository::new(pool);
    let purchase = repo.mark_paid(&input.id, input.amount, &input.payment_method).await?;
    crate::events::emit_suppliers_changed(&app).await;
    Ok(purchase)
}