// src-tauri/src/commands/print.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::error::AppError;

#[tauri::command]
pub async fn print_receipt(
    app: AppHandle,
    db: State<'_, crate::database::Db>,
    invoice_id: String,
    auth_header: Option<String>,
    business: Option<crate::services::print::ReceiptBusiness>,
) -> Result<(), AppError> {
    let pool = db.pool().await;
    require_cashier_or_admin(&app, auth_header).await?;
    crate::services::print::print_receipt(&pool, &invoice_id, business).await
}

#[tauri::command]
pub async fn print_invoice_pdf(app: AppHandle, db: State<'_, crate::database::Db>, invoice_id: String, auth_header: Option<String>) -> Result<String, AppError> {
    let pool = db.pool().await;
    require_cashier_or_admin(&app, auth_header).await?;
    crate::services::invoice_pdf::generate_invoice_pdf(&app, &pool, &invoice_id).await
}

#[tauri::command]
pub async fn list_usb_printers(_app: AppHandle, _auth: Option<String>) -> Result<Vec<UsbPrinterInfo>, AppError> {
    crate::services::print::list_usb_printers().await
}

#[derive(serde::Serialize)]
pub struct UsbPrinterInfo {
    pub vid: u16,
    pub pid: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}