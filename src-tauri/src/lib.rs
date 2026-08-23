// src-tauri/src/lib.rs
pub mod auth;
pub mod commands;
pub mod config;
pub mod database;
pub mod error;
pub mod events;
pub mod repositories;
pub mod services;
pub mod autostart;

pub use error::{AppError, Result};
pub use database::DbPool;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::default(), None))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.webview_windows().get("main").map(|w| w.set_focus());
        }))
        .plugin(tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build())
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(async {
                crate::database::init_db(app.handle()).await
            })?;
            app.manage(crate::database::Db::new(pool));
            
            // Setup autostart
            crate::autostart::setup_autostart(app.handle())?;
            crate::autostart::setup_single_instance(app.handle())?;
            
            // Install panic hook
            std::panic::set_hook(Box::new(|panic_info| {
                tracing::error!("Panic: {:?}", panic_info);
            }));
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            commands::auth::login,
            commands::auth::register,
            commands::auth::me,
            commands::auth::logout,
            commands::auth::check_admin_exists,
            commands::auth::get_roles,
            
            // Product commands
            commands::products::list_products,
            commands::products::get_product,
            commands::products::create_product,
            commands::products::update_product,
            commands::products::delete_product,
            commands::products::import_products,
            commands::products::export_products,
            
            // Customer commands
            commands::customers::list_customers,
            commands::customers::get_customer,
            commands::customers::create_customer,
            commands::customers::update_customer,
            commands::customers::delete_customer,
            
            // Invoice commands
            commands::invoices::list_invoices,
            commands::invoices::get_invoice,
            commands::invoices::get_invoice_with_items,
            commands::invoices::create_invoice,
            commands::invoices::mark_invoice_paid,
            commands::invoices::get_next_invoice_no,
            
            // Return commands
            commands::returns::list_returns,
            commands::returns::create_return,
            
            // Report commands
            commands::reports::get_dashboard,
            commands::reports::get_sales_report,
            commands::reports::get_inventory_report,
            
            // Cashier commands
            commands::cashiers::list_cashiers,
            commands::cashiers::approve_cashier,
            commands::cashiers::reject_cashier,
            commands::cashiers::suspend_cashier,
            commands::cashiers::reset_password,
            
            // Print commands
            commands::print::print_receipt,
            commands::print::print_invoice_pdf,
            commands::print::list_usb_printers,
            
            // Backup commands
            commands::backup::export_database,
            commands::backup::import_database,
            commands::backup::list_backups,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}