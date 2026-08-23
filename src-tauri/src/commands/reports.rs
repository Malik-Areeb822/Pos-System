// src-tauri/src/commands/reports.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_cashier_or_admin;
use crate::DbPool;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Serialize)]
pub struct DashboardStats {
    pub total_sales_today: i64,
    pub total_invoices_today: i64,
    pub low_stock_count: i64,
    pub outstanding_balance: i64,
}

#[derive(Serialize)]
pub struct SalesReportItem {
    pub date: String,
    pub total_sales: i64,
    pub invoice_count: i64,
    pub cash_sales: i64,
    pub credit_sales: i64,
    pub bank_sales: i64,
}

#[derive(Serialize)]
pub struct InventoryReportItem {
    pub id: String,
    pub name: String,
    pub sku: Option<String>,
    pub category: String,
    pub stock_qty: i64,
    pub low_stock_threshold: i64,
    pub unit: String,
    pub price: i64,
    pub value: i64,
}

#[derive(Deserialize)]
pub struct SalesReportInput {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[tauri::command]
pub async fn get_dashboard(db: State<'_, crate::database::Db>, _auth: Option<String>) -> Result<DashboardStats, AppError> {
    let pool = db.pool().await;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    
    let total_sales_today: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(total), 0) FROM invoices WHERE date(created_at) = ?",
        today
    )
    .fetch_one(&pool)
    .await?;
    
    let total_invoices_today: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM invoices WHERE date(created_at) = ?",
        today
    )
    .fetch_one(&pool)
    .await?;
    
    let low_stock_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM products WHERE stock_qty <= low_stock_threshold AND is_published = 1"
    )
    .fetch_one(&pool)
    .await?;
    
    let outstanding_balance: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(outstanding_balance), 0) FROM customers WHERE outstanding_balance > 0"
    )
    .fetch_one(&pool)
    .await?;

    Ok(DashboardStats {
        total_sales_today,
        total_invoices_today,
        low_stock_count,
        outstanding_balance,
    })
}

#[tauri::command]
pub async fn get_sales_report(db: State<'_, crate::database::Db>, input: SalesReportInput, _auth: Option<String>) -> Result<Vec<SalesReportItem>, AppError> {
    let pool = db.pool().await;
    let from = input.from_date.unwrap_or_else(|| {
        chrono::Utc::now().format("%Y-%m-01").to_string()
    });
    let to = input.to_date.unwrap_or_else(|| {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    });

    let rows = sqlx::query!(
        r#"
        SELECT 
            date(created_at) as date,
            SUM(total) as total_sales,
            COUNT(*) as invoice_count,
            SUM(CASE WHEN payment_method = 'cash' THEN total ELSE 0 END) as cash_sales,
            SUM(CASE WHEN payment_method = 'credit' THEN total ELSE 0 END) as credit_sales,
            SUM(CASE WHEN payment_method = 'bank' THEN total ELSE 0 END) as bank_sales
        FROM invoices
        WHERE date(created_at) BETWEEN ? AND ?
        GROUP BY date(created_at)
        ORDER BY date(created_at)
        "#,
        from, to
    )
    .fetch_all(&pool)
    .await?;

    let report = rows.into_iter().map(|r| SalesReportItem {
        date: r.date.unwrap_or_default(),
        total_sales: r.total_sales,
        invoice_count: r.invoice_count,
        cash_sales: r.cash_sales,
        credit_sales: r.credit_sales,
        bank_sales: r.bank_sales,
    }).collect();

    Ok(report)
}

#[tauri::command]
pub async fn get_inventory_report(db: State<'_, crate::database::Db>, _auth: Option<String>) -> Result<Vec<InventoryReportItem>, AppError> {
    let pool = db.pool().await;
    let rows = sqlx::query!(
        r#"
        SELECT id, name, sku, category, stock_qty, low_stock_threshold, unit, price,
               (stock_qty * price) as value
        FROM products
        WHERE is_published = 1
        ORDER BY category, name
        "#
    )
    .fetch_all(&pool)
    .await?;

    let report = rows.into_iter().map(|r| InventoryReportItem {
        id: r.id.unwrap_or_default(),
        name: r.name,
        sku: r.sku,
        category: r.category,
        stock_qty: r.stock_qty,
        low_stock_threshold: r.low_stock_threshold,
        unit: r.unit,
        price: r.price,
        value: r.value,
    }).collect();

    Ok(report)
}