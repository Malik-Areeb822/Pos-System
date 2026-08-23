// src-tauri/src/events/mod.rs
pub mod emitter;

pub use emitter::{emit_invoices_changed, emit_customers_changed, emit_products_changed, emit_cashiers_changed};