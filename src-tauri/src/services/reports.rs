// src-tauri/src/services/reports.rs
use crate::DbPool;
use crate::error::AppError;

#[derive(serde::Serialize)]
pub struct DashboardStats {
    pub total_sales_today: i64,
    pub total_invoices_today: i64,
    pub low_stock_count: i64,
    pub outstanding_balance: i64,
}

pub async fn get_dashboard(pool: &DbPool) -> Result<DashboardStats, AppError> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    
    let total_sales_today: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(total), 0) FROM invoices WHERE date(created_at) = ?",
        today
    )
    .fetch_one(pool)
    .await?;
    
    let total_invoices_today: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM invoices WHERE date(created_at) = ?",
        today
    )
    .fetch_one(pool)
    .await?;
    
    let low_stock_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM products WHERE stock_qty <= low_stock_threshold AND is_published = 1"
    )
    .fetch_one(pool)
    .await?;
    
    let outstanding_balance: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(outstanding_balance), 0) FROM customers WHERE outstanding_balance > 0"
    )
    .fetch_one(pool)
    .await?;

    Ok(DashboardStats {
        total_sales_today,
        total_invoices_today,
        low_stock_count,
        outstanding_balance,
    })
}