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