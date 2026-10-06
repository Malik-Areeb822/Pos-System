// src-tauri/src/commands/reports.rs
use tauri::State;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use chrono::TimeZone;

#[derive(Serialize)]
pub struct DashboardStats {
    pub total_sales_today: i64,
    pub total_invoices_today: i64,
    pub sales_7d: i64,
    pub invoices_7d: i64,
    pub low_stock_count: i64,
    pub outstanding_balance: i64,
    pub profit_today: i64,
    pub profit_7d: i64,
    pub profit_30d: i64,
}

#[derive(Serialize)]
pub struct SalesReportItem {
    pub date: String,
    pub total_sales: i64,
    pub invoice_count: i64,
    pub cash_sales: i64,
    pub credit_sales: i64,
    pub bank_sales: i64,
    pub profit: i64,
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
    pub purchase_price: i64,
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

    let now_local = chrono::Local::now();
    let today_start_naive = now_local.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_start_dt = now_local.timezone()
        .from_local_datetime(&today_start_naive)
        .single()
        .unwrap_or(now_local);
    let today_start_utc = today_start_dt.with_timezone(&chrono::Utc);
    let today_end_utc = today_start_utc + chrono::Duration::days(1);
    let start_7d_utc = today_start_utc - chrono::Duration::days(6);
    let start_30d_utc = today_start_utc - chrono::Duration::days(29);

    let today_start = today_start_utc.to_rfc3339();
    let today_end = today_end_utc.to_rfc3339();
    let start_7d = start_7d_utc.to_rfc3339();
    let start_30d = start_30d_utc.to_rfc3339();

    // Net revenue: subtract anything this invoice carried over from a previous
    // one — that money was already counted when it first changed hands.
    let total_sales_today: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(total - previous_balance), 0) FROM invoices WHERE created_at >= ? AND created_at < ?",
        today_start,
        today_end
    )
    .fetch_one(&pool)
    .await?;

    let total_invoices_today: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM invoices WHERE created_at >= ? AND created_at < ?",
        today_start,
        today_end
    )
    .fetch_one(&pool)
    .await?;

    let sales_7d: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(total - previous_balance), 0) FROM invoices WHERE created_at >= ? AND created_at < ?",
        start_7d,
        today_end
    )
    .fetch_one(&pool)
    .await?;

    let invoices_7d: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM invoices WHERE created_at >= ? AND created_at < ?",
        start_7d,
        today_end
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

    let profit_sql = "SELECT COALESCE(SUM(
           CASE WHEN i.subtotal = 0 THEN ii.line_total - ii.purchase_price * ii.quantity
           ELSE ii.line_total * (i.subtotal - i.discount) / i.subtotal - ii.purchase_price * ii.quantity END
         ), 0)
         - COALESCE((
           SELECT SUM(
             CASE WHEN i2.subtotal = 0 THEN r.line_total - COALESCE(ii2.purchase_price, 0) * r.quantity
             ELSE r.line_total * (i2.subtotal - i2.discount) / i2.subtotal - COALESCE(ii2.purchase_price, 0) * r.quantity END
           )
           FROM returns r
           LEFT JOIN invoice_items ii2 ON ii2.invoice_id = r.invoice_id
             AND (ii2.product_id = r.product_id
                  OR (ii2.product_id IS NULL AND ii2.product_name = r.product_name))
           LEFT JOIN invoices i2 ON i2.id = r.invoice_id
           WHERE r.created_at >= ? AND r.created_at < ?
         ), 0)
         FROM invoice_items ii JOIN invoices i ON i.id = ii.invoice_id
         WHERE i.created_at >= ? AND i.created_at < ?";

    let profit_today: i64 = sqlx::query_scalar(profit_sql)
        .bind(&today_start)
        .bind(&today_end)
        .bind(&today_start)
        .bind(&today_end)
        .fetch_one(&pool)
        .await?;

    let profit_7d: i64 = sqlx::query_scalar(profit_sql)
        .bind(&start_7d)
        .bind(&today_end)
        .bind(&start_7d)
        .bind(&today_end)
        .fetch_one(&pool)
        .await?;

    let profit_30d: i64 = sqlx::query_scalar(profit_sql)
        .bind(&start_30d)
        .bind(&today_end)
        .bind(&start_30d)
        .bind(&today_end)
        .fetch_one(&pool)
        .await?;

    Ok(DashboardStats {
        total_sales_today,
        total_invoices_today,
        sales_7d,
        invoices_7d,
        low_stock_count,
        outstanding_balance,
        profit_today,
        profit_7d,
        profit_30d,
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

    let rows = sqlx::query(
        r#"
        SELECT
            date(i.created_at) as date,
            SUM(i.total - i.previous_balance) as total_sales,
            COUNT(DISTINCT i.id) as invoice_count,
            SUM(CASE WHEN i.payment_method = 'cash' THEN i.total - i.previous_balance ELSE 0 END) as cash_sales,
            SUM(CASE WHEN i.payment_method = 'credit' THEN i.total - i.previous_balance ELSE 0 END) as credit_sales,
            SUM(CASE WHEN i.payment_method = 'bank' THEN i.total - i.previous_balance ELSE 0 END) as bank_sales,
            COALESCE(SUM(
              CASE WHEN i.subtotal = 0 THEN ii.line_total - ii.purchase_price * ii.quantity
              ELSE ii.line_total * (i.subtotal - i.discount) / i.subtotal - ii.purchase_price * ii.quantity END
            ), 0)
              - COALESCE((
                SELECT SUM(
                  CASE WHEN i2.subtotal = 0 THEN r.line_total - COALESCE(ii2.purchase_price, 0) * r.quantity
                  ELSE r.line_total * (i2.subtotal - i2.discount) / i2.subtotal - COALESCE(ii2.purchase_price, 0) * r.quantity END
                )
                FROM returns r
                LEFT JOIN invoice_items ii2 ON ii2.invoice_id = r.invoice_id
                  AND (ii2.product_id = r.product_id
                       OR (ii2.product_id IS NULL AND ii2.product_name = r.product_name))
                LEFT JOIN invoices i2 ON i2.id = r.invoice_id
                WHERE date(r.created_at) = date(i.created_at)
              ), 0) as profit
        FROM invoices i
        LEFT JOIN invoice_items ii ON ii.invoice_id = i.id
        WHERE date(i.created_at) BETWEEN ? AND ?
        GROUP BY date(i.created_at)
        ORDER BY date(i.created_at)
        "#
    )
    .bind(&from)
    .bind(&to)
    .fetch_all(&pool)
    .await?;

    let report = rows.into_iter().map(|r| SalesReportItem {
        date: r.get::<Option<String>, _>("date").unwrap_or_default(),
        total_sales: r.get::<i64, _>("total_sales"),
        invoice_count: r.get::<i64, _>("invoice_count"),
        cash_sales: r.get::<i64, _>("cash_sales"),
        credit_sales: r.get::<i64, _>("credit_sales"),
        bank_sales: r.get::<i64, _>("bank_sales"),
        profit: r.get::<i64, _>("profit"),
    }).collect();

    Ok(report)
}

#[tauri::command]
pub async fn get_inventory_report(db: State<'_, crate::database::Db>, _auth: Option<String>) -> Result<Vec<InventoryReportItem>, AppError> {
    let pool = db.pool().await;
    let rows = sqlx::query!(
        r#"
        SELECT id, name, sku, category, stock_qty, low_stock_threshold, unit, price, purchase_price,
               (MAX(0, stock_qty) * price) as value
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
        purchase_price: r.purchase_price.unwrap_or(0),
        value: r.value.unwrap_or(0),
    }).collect();

    Ok(report)
}