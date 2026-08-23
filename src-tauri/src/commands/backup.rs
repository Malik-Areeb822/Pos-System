// src-tauri/src/commands/backup.rs
use tauri::{State, AppHandle};
use crate::auth::middleware::require_admin;
use crate::DbPool;
use crate::error::AppError;
use std::path::PathBuf;
use tokio::fs;

#[tauri::command]
pub async fn export_database(app: AppHandle, pool: State<'_, DbPool>, auth_header: Option<String>) -> Result<String, AppError> {
    require_admin(&app, auth_header).await?;
    
    let config = crate::config::CONFIG.clone();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = config.app_data_dir.join(format!("citytiles_backup_{}.sqlite", timestamp));
    
    // Copy the database file
    fs::copy(&config.db_path, &backup_path).await
        .map_err(|e| AppError::Internal(format!("Backup failed: {}", e)))?;
    
    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_database(app: AppHandle, pool: State<'_, DbPool>, backup_path: String, auth_header: Option<String>) -> Result<(), AppError> {
    require_admin(&app, auth_header).await?;
    
    let config = crate::config::CONFIG.clone();
    let src = PathBuf::from(backup_path);
    
    if !src.exists() {
        return Err(AppError::NotFound("Backup file not found".into()));
    }
    
    // Close existing pool connections (not trivial with sqlx, but we can overwrite)
    fs::copy(&src, &config.db_path).await
        .map_err(|e| AppError::Internal(format!("Restore failed: {}", e)))?;
    
    // Re-run migrations to ensure schema is correct
    sqlx::migrate!("./src/database/migrations")
        .run(&*pool)
        .await
        .map_err(|e| AppError::Migration(e.to_string()))?;
    
    Ok(())
}

#[tauri::command]
pub async fn list_backups(_app: AppHandle, _auth: Option<String>) -> Result<Vec<BackupInfo>, AppError> {
    let config = crate::config::CONFIG.clone();
    let mut backups = Vec::new();
    
    if let Ok(mut entries) = fs::read_dir(&config.app_data_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("citytiles_backup_") && name.ends_with(".sqlite") {
                if let Ok(metadata) = entry.metadata().await {
                    backups.push(BackupInfo {
                        path: entry.path().to_string_lossy().to_string(),
                        name,
                        size: metadata.len(),
                        created: metadata.created().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0),
                    });
                }
            }
        }
    }
    
    backups.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(backups)
}

#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub created: u64,
}