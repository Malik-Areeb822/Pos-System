// src-tauri/src/database/seed.rs
use sqlx::SqlitePool;

/// Run seed data - only inserts if tables are empty
pub async fn run_seed(pool: &SqlitePool) -> Result<(), String> {
    // Check if products table is empty
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Failed to check products count: {}", e))?;
    
    if count == 0 {
        tracing::info!("Products table empty, seed data will be applied via migrations");
    }
    
    // The actual seed data is in 004_seed_data.sql migration
    // This function can be extended for additional runtime seeding if needed
    
    Ok(())
}