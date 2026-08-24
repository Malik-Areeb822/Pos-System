// src-tauri/src/commands/backup.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_admin;
use crate::database::Db;
use crate::error::AppError;
use std::path::{Path, PathBuf};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::fs;

/// Expected core tables a valid CityTiles database must contain.
const REQUIRED_TABLES: &[&str] = &[
    "users",
    "products",
    "customers",
    "invoices",
    "invoice_items",
    "returns",
    "_sqlx_migrations",
];

fn backups_dir(config: &crate::config::Config) -> PathBuf {
    config.app_data_dir.join("backups")
}

fn timestamp() -> String {
    chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()
}

/// Escape a path for use as an SQLite string literal (VACUUM INTO).
fn sql_literal(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "''"))
}

#[tauri::command]
pub async fn export_database(app: AppHandle, db: State<'_, Db>, auth_header: Option<String>) -> Result<String, AppError> {
    require_admin(&app, auth_header).await?;

    let config = crate::config::CONFIG.clone();
    let dir = backups_dir(&config);
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| AppError::Internal(format!("Backup failed: {}", e)))?;
    let backup_path = dir.join(format!("citytiles_backup_{}.sqlite", timestamp()));
    let _ = fs::remove_file(&backup_path).await; // VACUUM INTO requires the target not to exist

    // Consistent snapshot straight from the live pool: safe against in-flight
    // writes and verifies the database is readable as a side effect.
    let pool = db.pool().await;
    let vacuum = format!("VACUUM INTO {}", sql_literal(&backup_path));
    sqlx::query(&vacuum)
        .execute(&pool)
        .await
        .map_err(|e| AppError::Internal(format!("Backup failed: {}", e)))?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_database(app: AppHandle, db: State<'_, Db>, backup_path: String, auth_header: Option<String>) -> Result<(), AppError> {
    require_admin(&app, auth_header).await?;

    let config = crate::config::CONFIG.clone();
    let src = PathBuf::from(&backup_path);
    if !src.exists() {
        return Err(AppError::NotFound("Backup file not found".into()));
    }

    // 1. Stage a copy next to the live DB (same volume).
    let stage_path = config.app_data_dir.join("restore_staging.sqlite");
    let _ = fs::remove_file(&stage_path).await;
    if fs::copy(&src, &stage_path).await.is_err() {
        return Err(AppError::Internal("Could not read the selected backup file".into()));
    }

    // 2. Validate the staged copy before touching anything live:
    //    integrity_check + all expected tables present.
    let stage_url = format!("sqlite:{}?mode=ro", stage_path.to_string_lossy().replace('\\', "/"));
    let probe = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&stage_url)
        .await;
    let probe = match probe {
        Ok(p) => p,
        Err(_) => {
            let _ = fs::remove_file(&stage_path).await;
            return Err(AppError::Internal("Selected file is not a readable SQLite database".into()));
        }
    };
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&probe)
        .await
        .unwrap_or_else(|_| "probe failed".to_string());
    let found_tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (
            'users','products','customers','invoices','invoice_items','returns','_sqlx_migrations'
        )",
    )
    .fetch_one(&probe)
    .await
    .unwrap_or(0);
    probe.close().await;

    if integrity != "ok" {
        let _ = fs::remove_file(&stage_path).await;
        return Err(AppError::Internal(format!("Backup failed integrity check ({})", integrity)));
    }
    if (found_tables as usize) < REQUIRED_TABLES.len() {
        let _ = fs::remove_file(&stage_path).await;
        return Err(AppError::Internal("File does not look like a CityTiles database".into()));
    }

    // 3. Auto-snapshot the current DB first (safety copy), best effort.
    {
        let live = db.pool().await;
        let snapshot = backups_dir(&config).join(format!("pre_restore_{}.sqlite", timestamp()));
        let _ = fs::create_dir_all(backups_dir(&config)).await;
        let vacuum = format!("VACUUM INTO {}", sql_literal(&snapshot));
        if sqlx::query(&vacuum).execute(&live).await.is_err() {
            log::warn!("Pre-restore snapshot failed; continuing with restore");
        }
    }

    // 4. Swap: take exclusive access, close the old pool (releases file locks),
    //    copy the staged file over the live DB, open + migrate a new pool,
    //    install it. Commands cloning pools block until this completes.
    let holder = db.holder();
    let mut guard = holder.write().await;
    let old = guard.clone();
    old.close().await;

    if let Err(e) = fs::copy(&stage_path, &config.db_path).await {
        // Try to bring the old pool back so the app stays usable.
        if let Ok(revived) = crate::database::open_pool(&config.db_path).await {
            *guard = revived;
        }
        let _ = fs::remove_file(&stage_path).await;
        return Err(AppError::Internal(format!("Restore failed: {}", e)));
    }
    let _ = fs::remove_file(&stage_path).await;

    match crate::database::open_pool(&config.db_path).await {
        Ok(new_pool) => {
            // Bring a restored database up to retention policy too (older
            // backups may contain sales beyond the 12-month window).
            let purge_pool = new_pool.clone();
            *guard = new_pool;
            drop(guard);
            let _ = old; // already closed
            crate::events::emit_database_restored(&app).await;
            tauri::async_runtime::spawn(crate::services::retention::run_maintenance(
                app.clone(),
                purge_pool,
            ));
            Ok(())
        }
        Err(e) => {
            // The copied file passed validation but won't open — surface loudly.
            log::error!("Restored database failed to open: {}", e);
            Err(AppError::Internal(format!(
                "Restore completed but the database failed to reopen: {}. Restart the app.",
                e
            )))
        }
    }
}

#[tauri::command]
pub async fn list_backups(_app: AppHandle, _db: State<'_, Db>, _auth: Option<String>) -> Result<Vec<BackupInfo>, AppError> {
    let config = crate::config::CONFIG.clone();
    let mut backups = Vec::new();

    // Current exports live under %PROGRAMDATA%\CityTiles\backups; older ones
    // sit directly in %PROGRAMDATA%\CityTiles. Scan both, dedupe by path.
    let mut roots = vec![backups_dir(&config)];
    roots.push(config.app_data_dir.clone());

    for root in roots {
        let mut entries = match fs::read_dir(&root).await {
            Ok(e) => e,
            Err(_) => continue,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_backup =
                (name.starts_with("citytiles_backup_") || name.starts_with("pre_restore_"))
                    && name.ends_with(".sqlite");
            if !is_backup {
                continue;
            }
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    backups.push(BackupInfo {
                        path: entry.path().to_string_lossy().to_string(),
                        name,
                        size: metadata.len(),
                        created: metadata
                            .created()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    });
                }
            }
        }
    }

    backups.sort_by(|a, b| b.created.cmp(&a.created));
    backups.dedup_by(|a, b| a.path == b.path);
    Ok(backups)
}

#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub created: u64,
}
