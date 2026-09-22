// src-tauri/src/services/reconciliation.rs
use crate::database::DbPool;
use crate::error::AppError;

/// Recompute every customer's outstanding_balance from the actual invoice ledger.
///
/// Sets each customer's balance to the due of their latest non-carried unpaid
/// invoice (the leaf of the carry chain). Customers with no unpaid invoices get
/// a zero balance. Walk-in customers (customer_id = NULL) are untouched.
///
/// Returns the number of customer rows affected.
pub async fn reconcile_balances(pool: &DbPool) -> Result<i64, AppError> {
    let result = sqlx::query(
        r#"UPDATE customers SET outstanding_balance = (
            SELECT COALESCE(MAX(0, inv.total - inv.amount_paid), 0)
            FROM invoices inv
            WHERE inv.customer_id = customers.id
              AND inv.carried_to_invoice_id IS NULL
              AND inv.total > inv.amount_paid
            ORDER BY inv.created_at DESC LIMIT 1
        ) WHERE id IN (
            SELECT DISTINCT customer_id FROM invoices WHERE customer_id IS NOT NULL
        )"#
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() as i64)
}

/// Fire-and-forget startup wrapper: reconcile, then notify the UI only if rows
/// actually changed. Never panics the app.
pub async fn run_on_startup(app: tauri::AppHandle, pool: DbPool) {
    match reconcile_balances(&pool).await {
        Ok(n) if n > 0 => {
            log::info!("Reconciled {} customer balances on startup", n);
            crate::events::emit_customers_changed(&app).await;
        }
        Ok(_) => {}
        Err(e) => log::error!("Balance reconciliation failed: {}", e),
    }
}
