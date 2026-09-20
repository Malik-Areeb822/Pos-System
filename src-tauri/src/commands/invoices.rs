// src-tauri/src/commands/invoices.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::repositories::{InvoiceRepository, Invoice, InvoiceItem};
use crate::DbPool;
use crate::error::AppError;
use serde::{Deserialize};

#[derive(Deserialize)]
pub struct ListInvoicesInput {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Full-history search across invoice number and customer name.
    pub query: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateInvoiceInputCmd {
    pub customer_id: Option<String>,
    pub customer_name: String,
    pub subtotal: i64,
    pub discount: i64,
    pub total: i64,
    pub amount_paid: Option<i64>,
    pub payment_method: String,
    pub notes: Option<String>,
    pub delivery_date: Option<String>,
    pub items: Vec<CreateInvoiceItemInputCmd>,
}

#[derive(Deserialize)]
pub struct CreateInvoiceItemInputCmd {
    pub product_id: Option<String>,
    pub product_name: String,
    pub quantity: i64,
    pub unit: String,
    pub unit_price: i64,
    pub line_total: i64,
    pub purchase_price: i64,
    pub total_area: Option<f64>,
}

#[derive(Deserialize)]
pub struct MarkPaidInput {
    pub id: String,
    pub amount: i64,
    pub payment_method: String,
}

#[tauri::command]
pub async fn list_invoices(db: State<'_, crate::database::Db>, input: ListInvoicesInput, _auth: Option<String>) -> Result<Vec<Invoice>, AppError> {
    let pool = db.pool().await;
    let repo = InvoiceRepository::new(pool);
    repo.list(input.limit.unwrap_or(50), input.offset.unwrap_or(0), input.query.as_deref()).await
}

#[tauri::command]
pub async fn get_invoice(db: State<'_, crate::database::Db>, id: String, _auth: Option<String>) -> Result<Option<Invoice>, AppError> {
    let pool = db.pool().await;
    let repo = InvoiceRepository::new(pool);
    repo.get(&id).await
}

#[tauri::command]
pub async fn get_invoice_with_items(db: State<'_, crate::database::Db>, id: String, _auth: Option<String>) -> Result<Option<(Invoice, Vec<InvoiceItem>)>, AppError> {
    let pool = db.pool().await;
    let repo = InvoiceRepository::new(pool);
    repo.get_with_items(&id).await
}

#[tauri::command]
pub async fn create_invoice(app: AppHandle, db: State<'_, crate::database::Db>, input: CreateInvoiceInputCmd, auth_header: Option<String>) -> Result<Invoice, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    // Ledger hardening: a negative discount would INFLATE the bill, so it is
    // rejected server-side no matter which client calls this command.
    if input.discount < 0 {
        return Err(AppError::Validation("Discount cannot be negative".into()));
    }
    if input.subtotal < 0 {
        return Err(AppError::Validation(
            "Invoice amounts cannot be negative".into(),
        ));
    }
    if let Some(paid) = input.amount_paid {
        if paid < 0 {
            return Err(AppError::Validation(
                "Amount paid cannot be negative".into(),
            ));
        }
    }
    for item in &input.items {
        if item.quantity <= 0 {
            return Err(AppError::Validation(format!(
                "{}: quantity must be at least 1",
                item.product_name
            )));
        }
        if item.unit_price < 0 || item.line_total < 0 {
            return Err(AppError::Validation(format!(
                "{}: amounts cannot be negative",
                item.product_name
            )));
        }
    }
    let pool = db.pool().await;
    let repo = InvoiceRepository::new(pool);
    let create_input = crate::repositories::CreateInvoiceInput {
        customer_id: input.customer_id,
        customer_name: input.customer_name,
        subtotal: input.subtotal,
        discount: input.discount,
        total: input.total,
        amount_paid: input.amount_paid,
        payment_method: input.payment_method,
        notes: input.notes,
        delivery_date: input.delivery_date,
        items: input.items.into_iter().map(|item| crate::repositories::CreateInvoiceItemInput {
            product_id: item.product_id,
            product_name: item.product_name,
            quantity: item.quantity,
            unit: item.unit,
            unit_price: item.unit_price,
            line_total: item.line_total,
            purchase_price: item.purchase_price,
            total_area: item.total_area,
        }).collect(),
    };
    let invoice = repo.create(create_input).await?;
    crate::events::emit_invoices_changed(&app).await;
    if let Some(customer_id) = &invoice.customer_id {
        crate::events::emit_customers_changed(&app).await;
    }
    crate::events::emit_products_changed(&app).await;
    Ok(invoice)
}

#[tauri::command]
pub async fn mark_invoice_paid(app: AppHandle, db: State<'_, crate::database::Db>, input: MarkPaidInput, auth_header: Option<String>) -> Result<Invoice, AppError> {
    require_cashier_or_admin(&app, auth_header).await?;
    if input.amount <= 0 {
        return Err(AppError::Validation(
            "Payment amount must be greater than zero".into(),
        ));
    }
    let pool = db.pool().await;
    let repo = InvoiceRepository::new(pool);
    let invoice = repo.mark_paid(&input.id, input.amount, &input.payment_method).await?;
    crate::events::emit_invoices_changed(&app).await;
    if let Some(customer_id) = &invoice.customer_id {
        crate::events::emit_customers_changed(&app).await;
    }
    Ok(invoice)
}

#[tauri::command]
pub async fn get_next_invoice_no(db: State<'_, crate::database::Db>, _auth: Option<String>) -> Result<String, AppError> {
    let pool = db.pool().await;
    let repo = InvoiceRepository::new(pool);
    repo.get_next_invoice_no().await
}