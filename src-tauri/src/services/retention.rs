// src-tauri/src/services/retention.rs
//! Sales-data retention: keeps the local SQLite store light over years by
//! purging fully-settled invoices older than `RETENTION_DAYS` on launch.
//!
//! Safety rules:
//! - Credit / partially-paid invoices are never deleted. Customer
//!   `outstanding_balance` is a denormalized figure that cannot be
//!   reconciled without its source invoice, so an unpaid sale stays until
//!   it is fully settled (it becomes purge-eligible automatically).
//! - A `VACUUM INTO` snapshot is written to the standard backups folder
//!   before any deletion; if the snapshot fails, the purge aborts.
//! - `invoice_items` and `returns` rows cascade away with their invoice,
//!   keeping referential integrity intact.

use crate::database::DbPool;
use crate::error::AppError;

/// Sales older than this are eligible for purge.
const RETENTION_DAYS: i64 = 365;

/// Delete settled invoices older than the retention window.
/// Returns the number of invoices removed (0 when nothing to do).
pub async fn purge_old_sales(pool: &DbPool) -> Result<u64, AppError> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(RETENTION_DAYS)).to_rfc3339();

    // Only fully-settled sales are removable — protects receivables.
    let eligible: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM invoices WHERE created_at < ? AND amount_paid >= total",
    )
    .bind(&cutoff)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Retention check failed: {}", e)))?;

    if eligible == 0 {
        return Ok(0);
    }

    // Pre-purge snapshot into the standard backups folder so purged history
    // remains recoverable from Settings -> Backups.
    let config = crate::config::CONFIG.clone();
    let dir = config.app_data_dir.join("backups");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| AppError::Internal(format!("Retention backup failed: {}", e)))?;
    let snapshot = dir.join(format!(
        "citytiles_backup_{}.sqlite",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    ));
    let _ = tokio::fs::remove_file(&snapshot).await; // VACUUM INTO requires the target not to exist

    // Consistent snapshot straight from the live pool; aborts the purge on failure.
    let vacuum = format!(
        "VACUUM INTO '{}'",
        snapshot.to_string_lossy().replace('\'', "''")
    );
    sqlx::query(&vacuum)
        .execute(pool)
        .await
        .map_err(|e| {
            AppError::Internal(format!(
                "Retention aborted: could not snapshot before deleting ({})",
                e
            ))
        })?;

    let result = sqlx::query("DELETE FROM invoices WHERE created_at < ? AND amount_paid >= total")
        .bind(&cutoff)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Retention purge failed: {}", e)))?;

    let removed = result.rows_affected();
    log::info!(
        "Retention: removed {} invoice(s) older than {} days (snapshot: {})",
        removed,
        RETENTION_DAYS,
        snapshot.to_string_lossy()
    );
    Ok(removed)
}

/// Fire-and-forget maintenance run used at startup and after a backup restore:
/// purge, then notify the UI only if data actually changed. Never panics the app.
pub async fn run_maintenance(app: tauri::AppHandle, pool: DbPool) {
    match purge_old_sales(&pool).await {
        Ok(n) if n > 0 => {
            crate::events::emit_invoices_changed(&app).await;
            crate::events::emit_customers_changed(&app).await;
        }
        Ok(_) => {}
        Err(e) => log::error!("Retention maintenance failed: {}", e),
    }
}
