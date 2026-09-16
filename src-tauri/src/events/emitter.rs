// src-tauri/src/events/emitter.rs
use tauri::{AppHandle, Emitter};

pub async fn emit_invoices_changed(app: &AppHandle) {
    let _ = app.emit("invoices:changed", ());
}

pub async fn emit_customers_changed(app: &AppHandle) {
    let _ = app.emit("customers:changed", ());
}

pub async fn emit_products_changed(app: &AppHandle) {
    let _ = app.emit("products:changed", ());
}

pub async fn emit_cashiers_changed(app: &AppHandle) {
    let _ = app.emit("cashiers:changed", ());
}

pub async fn emit_suppliers_changed(app: &AppHandle) {
    let _ = app.emit("suppliers:changed", ());
}

/// Emitted after a backup restore replaced the live database: every cached
/// query in the frontend is stale and must be refetched.
pub async fn emit_database_restored(app: &AppHandle) {
    let _ = app.emit("database:restored", ());
}