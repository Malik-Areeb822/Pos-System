// src-tauri/src/services/backup.rs
use crate::DbPool;
use crate::error::AppError;
use std::path::PathBuf;
use tokio::fs;

pub async fn export_database(pool: &DbPool, output_path: &PathBuf) -> Result<(), AppError> {
    let config = crate::config::CONFIG.clone();
    fs::copy(&config.db_path, output_path).await
        .map_err(|e| AppError::Internal(format!("Backup failed: {}", e)))?;
    
    let backup_pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", output_path.display())).await?;
    sqlx::query("SELECT 1").execute(&backup_pool).await?;
    backup_pool.close().await;
    
    Ok(())
}

pub async fn import_database(pool: &DbPool, backup_path: &PathBuf) -> Result<(), AppError> {
    if !backup_path.exists() {
        return Err(AppError::NotFound("Backup file not found".into()));
    }
    
    let config = crate::config::CONFIG.clone();
    
    let backup_pool = sqlx::SqlitePool::connect(&format!("sqlite://{}", backup_path.display())).await?;
    sqlx::query("SELECT 1").execute(&backup_pool).await?;
    backup_pool.close().await;
    
    fs::copy(backup_path, &config.db_path).await
        .map_err(|e| AppError::Internal(format!("Restore failed: {}", e)))?;
    
    sqlx::migrate!("./src/database/migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Migration(e.to_string()))?;
    
    Ok(())
}

pub async fn list_backups() -> Result<Vec<BackupInfo>, AppError> {
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