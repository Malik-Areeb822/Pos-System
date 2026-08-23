// src-tauri/src/database/mod.rs
pub mod connection;
pub mod seed;

use sqlx::SqlitePool;

pub use connection::{get_pool, init_db, DbPool};
pub use seed::run_seed;