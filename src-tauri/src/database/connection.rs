// src-tauri/src/database/connection.rs
use std::path::PathBuf;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tauri::Manager;

pub type DbPool = SqlitePool;

/// Get the database path in %PROGRAMDATA%\CityTiles\
pub fn get_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let program_data = std::env::var("PROGRAMDATA")
        .map_err(|_| "PROGRAMDATA environment variable not found")?;
    let db_dir = PathBuf::from(program_data).join("CityTiles");
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&db_dir)
        .map_err(|e| format!("Failed to create database directory: {}", e))?;
    
    Ok(db_dir.join("citytiles.db"))
}

/// Initialize database connection pool and run migrations
pub async fn init_db(app: &tauri::AppHandle) -> Result<DbPool, String> {
    let db_path = get_db_path(app)?;
    let db_path_str = db_path.to_string_lossy().replace('\\', "/");
    let db_url = format!("sqlite:{}?mode=rwc", db_path_str);
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;
    
    // Run migrations
    run_migrations(&pool).await?;
    
    // Run seed data
    super::seed::run_seed(&pool).await?;
    
    Ok(pool)
}

/// Run all SQL migrations
async fn run_migrations(pool: &DbPool) -> Result<(), String> {
    // Use sqlx migrate! macro for embedded migrations
    sqlx::migrate!("./src/database/migrations")
        .run(pool)
        .await
        .map_err(|e| format!("Migration failed: {}", e))?;
    
    Ok(())
}

/// Get existing pool from app state
pub fn get_pool(app: &tauri::AppHandle) -> Result<DbPool, String> {
    app.try_state::<DbPool>()
        .ok_or_else(|| "Database pool not initialized".to_string())
        .map(|state| state.inner().clone())
}