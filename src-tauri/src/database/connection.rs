// src-tauri/src/database/connection.rs
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tokio::sync::RwLock;
use tauri::Manager;

pub type DbPool = SqlitePool;

/// Swappable pool holder. Commands clone a cheap handle of the current live
/// pool per call; the backup/restore flow can replace the underlying pool
/// (close old -> swap file -> install new) without restarting the app.
#[derive(Clone)]
pub struct Db(Arc<RwLock<DbPool>>);

impl Db {
    pub fn new(pool: DbPool) -> Self {
        Self(Arc::new(RwLock::new(pool)))
    }

    /// Cheap clone of the current live pool. Blocks while a restore swap is
    /// in progress, which serializes POS operations behind the swap.
    pub async fn pool(&self) -> DbPool {
        self.0.read().await.clone()
    }

    /// Exclusive access to the holder for restore flows.
    /// (Named `holder`, not `inner`: tauri::State has its own `inner()`.)
    pub fn holder(&self) -> Arc<RwLock<DbPool>> {
        self.0.clone()
    }
}

/// Get the database path in %PROGRAMDATA%\CityTiles\
pub fn get_db_path(_app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let program_data = std::env::var("PROGRAMDATA")
        .map_err(|_| "PROGRAMDATA environment variable not found")?;
    let db_dir = PathBuf::from(program_data).join("CityTiles");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&db_dir)
        .map_err(|e| format!("Failed to create database directory: {}", e))?;

    Ok(db_dir.join("citytiles.db"))
}

/// Open a connection pool on the given database file and bring it up to date
/// (migrations + seed). Used at startup and after a backup restore.
pub async fn open_pool(db_path: &Path) -> Result<DbPool, String> {
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

/// Initialize database connection pool and run migrations
pub async fn init_db(app: &tauri::AppHandle) -> Result<DbPool, String> {
    let db_path = get_db_path(app)?;
    open_pool(&db_path).await
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
