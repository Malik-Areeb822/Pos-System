// src-tauri/src/commands/license.rs
//
// System lock commands. Three commands:
//   check_system_lock  — read lock state (no auth needed)
//   set_system_lock    — set lock state (admin only)
//   unlock_with_password — verify password and unlock

use tauri::State;
use crate::database::Db;
use crate::error::AppError;
use crate::license;

#[derive(serde::Deserialize)]
pub struct SetLockInput {
    pub locked: bool,
}

#[derive(serde::Deserialize)]
pub struct UnlockInput {
    pub password: String,
}

/// Check whether the system is currently locked.
/// No authentication required — the frontend calls this on mount.
#[tauri::command]
pub async fn check_system_lock(
    db: State<'_, Db>,
) -> Result<license::LockStatus, AppError> {
    let pool = db.pool().await;
    let is_locked = license::is_system_locked(&pool).await;
    Ok(license::LockStatus { is_locked })
}

/// Set the system lock state. No JWT auth required —
/// the secret key combo IS the authentication.
#[tauri::command]
pub async fn set_system_lock(
    db: State<'_, Db>,
    input: SetLockInput,
) -> Result<license::LockStatus, AppError> {
    let pool = db.pool().await;
    license::set_system_locked(&pool, input.locked).await?;

    Ok(license::LockStatus { is_locked: input.locked })
}

/// Unlock the system with a password.
/// No authentication required — the password IS the authentication.
#[tauri::command]
pub async fn unlock_with_password(
    db: State<'_, Db>,
    input: UnlockInput,
) -> Result<license::LockStatus, AppError> {
    if !license::verify_password(&input.password) {
        return Err(AppError::Auth("Invalid unlock password".into()));
    }

    let pool = db.pool().await;
    license::set_system_locked(&pool, false).await?;

    Ok(license::LockStatus { is_locked: false })
}
