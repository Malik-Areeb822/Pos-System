// src-tauri/src/database/connection.rs
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Connection, SqlitePool};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

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

/// Get the database path in %PROGRAMDATA%\MoonPipe\
pub fn get_db_path(_app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let program_data = std::env::var("PROGRAMDATA")
        .map_err(|_| "PROGRAMDATA environment variable not found")?;
    let db_dir = PathBuf::from(program_data).join("MoonPipe");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&db_dir)
        .map_err(|e| format!("Failed to create database directory: {}", e))?;

    Ok(db_dir.join("moonpipe.db"))
}

/// Open a connection pool on the given database file and bring it up to date
/// (migrations + seed). Used at startup and after a backup restore.
pub async fn open_pool(db_path: &Path) -> Result<DbPool, String> {
    let db_path_str = db_path.to_string_lossy().replace('\\', "/");
    let db_url = format!("sqlite:{}?mode=rwc", db_path_str);

    // Per-connection pragmas. `journal_mode` is deliberately absent: it is a
    // persistent property of the database file, and the switch into WAL takes
    // an exclusive lock that `busy_timeout` cannot wait on — so it is issued
    // exactly once below, on a lone connection, before the pool exists.
    let opts: SqliteConnectOptions = db_url
        .parse::<SqliteConnectOptions>()
        .map_err(|e| format!("Invalid database URL: {}", e))?
        .busy_timeout(Duration::from_millis(5000))
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Normal);

    // One-time WAL switch. Deliberately non-fatal: WAL is hardening, not a
    // correctness requirement, and refusing to boot would be worse than
    // falling back to the rollback journal.
    {
        let mut solo = SqliteConnection::connect_with(&opts)
            .await
            .map_err(|e| format!("Failed to connect to database: {}", e))?;
        match sqlx::query_scalar::<_, String>("PRAGMA journal_mode=WAL")
            .fetch_one(&mut solo)
            .await
        {
            Ok(mode) if mode.eq_ignore_ascii_case("wal") => log::info!("SQLite journal mode: WAL"),
            Ok(mode) => log::warn!("SQLite journal mode is '{}' (expected wal)", mode),
            Err(e) => log::warn!("Could not enable WAL journal mode: {}", e),
        }
        let _ = solo.close().await;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_db() -> PathBuf {
        std::env::temp_dir().join(format!("moonpipe-wal-{}.sqlite", uuid::Uuid::new_v4()))
    }

    async fn pragma_text(pool: &DbPool, sql: &str) -> String {
        sqlx::query_scalar::<_, String>(sql)
            .fetch_one(pool)
            .await
            .unwrap_or_else(|e| panic!("{} failed: {}", sql, e))
    }

    async fn pragma_int(pool: &DbPool, sql: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(sql)
            .fetch_one(pool)
            .await
            .unwrap_or_else(|e| panic!("{} failed: {}", sql, e))
    }

    #[tokio::test]
    async fn open_pool_enables_wal_and_per_connection_pragmas() {
        let db = scratch_db();
        let pool = open_pool(&db).await.expect("open scratch pool");

        let journal = pragma_text(&pool, "PRAGMA journal_mode").await;
        assert_eq!(
            journal.to_ascii_lowercase(),
            "wal",
            "journal_mode must be WAL"
        );

        assert_eq!(
            pragma_int(&pool, "PRAGMA foreign_keys").await,
            1,
            "foreign_keys must be ON"
        );

        // 0=OFF, 1=NORMAL, 2=FULL, 3=EXTRA
        assert_eq!(
            pragma_int(&pool, "PRAGMA synchronous").await,
            1,
            "synchronous must be NORMAL in WAL mode"
        );

        assert_eq!(
            pragma_int(&pool, "PRAGMA busy_timeout").await,
            5000,
            "busy_timeout must be 5000ms"
        );

        pool.close().await;
        let _ = std::fs::remove_file(&db);
    }

    #[tokio::test]
    async fn journal_mode_survives_close_and_reopen() {
        let db = scratch_db();

        let first = open_pool(&db).await.expect("first open");
        let before = pragma_text(&first, "PRAGMA journal_mode").await;
        assert_eq!(before.to_ascii_lowercase(), "wal");
        first.close().await;

        // WAL is stored in the database file header, so a fresh pool must see
        // it again without re-running the one-time switch.
        let second = open_pool(&db).await.expect("reopen");
        let after = pragma_text(&second, "PRAGMA journal_mode").await;
        assert_eq!(after.to_ascii_lowercase(), "wal");
        second.close().await;

        let _ = std::fs::remove_file(&db);
    }
}
