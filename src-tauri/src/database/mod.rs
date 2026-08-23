// src-tauri/src/database/mod.rs
pub mod connection;
pub mod seed;

use sqlx::SqlitePool;

pub use connection::{init_db, open_pool, Db, DbPool};
pub use seed::run_seed;