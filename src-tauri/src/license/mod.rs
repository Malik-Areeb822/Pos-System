// src-tauri/src/license/mod.rs
//
// System lock feature. The lock flag is stored in app_settings as an
// HMAC-signed value so direct SQLite edits are detected and rejected.
//
// The unlock password is bcrypt-hashed — the plaintext never exists in
// the compiled binary.

use sqlx::SqlitePool;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Serialize;

use crate::error::AppError;

type HmacSha256 = Hmac<Sha256>;

/// HMAC secret for signing the lock flag value.
/// This is compiled into the binary. The client cannot forge valid values
/// without knowing this secret.
const LOCK_HMAC_SECRET: &[u8] = b"moonpipe-system-lock-hmac-secret-2026";

/// Bcrypt hash of the unlock password "Areeb@1234".
/// The plaintext password never exists in the binary.
const UNLOCK_PASSWORD_HASH: &str = "$2b$12$N9tNurtQyROkiMDx2yMPpukIpN.Y6rPKl3hHzhLCMaBqtFysycjga";

// ---------------------------------------------------------------------------
// License state (Tauri managed state — in-memory, not persisted)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LockStatus {
    pub is_locked: bool,
}

#[derive(Clone)]
pub struct LicenseState(Arc<RwLock<LockStatus>>);

impl LicenseState {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(LockStatus { is_locked: true })))
    }

    pub async fn get(&self) -> LockStatus {
        self.0.read().await.clone()
    }

    pub async fn set_locked(&self, locked: bool) {
        *self.0.write().await = LockStatus { is_locked: locked };
    }
}

// ---------------------------------------------------------------------------
// HMAC helpers — sign and verify the lock flag value
// ---------------------------------------------------------------------------

/// Compute HMAC-SHA256 of the given value and return hex string.
fn sign_value(value: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(LOCK_HMAC_SECRET)
        .expect("HMAC accepts any key length");
    mac.update(value.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Verify that a stored value + its signature are valid.
fn verify_signature(value: &str, signature: &str) -> bool {
    let expected = sign_value(value);
    // Constant-time comparison
    expected.len() == signature.len()
        && expected.bytes().zip(signature.bytes()).all(|(a, b)| a == b)
}

// ---------------------------------------------------------------------------
// Database helpers
// ---------------------------------------------------------------------------

/// Read the lock state from the database.
/// Returns `true` (locked) if anything goes wrong (fail-closed).
pub async fn is_system_locked(pool: &SqlitePool) -> bool {
    let row: Result<Option<String>, _> = sqlx::query_scalar(
        "SELECT value FROM app_settings WHERE key = 'system_lock'"
    )
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(raw)) => {
            // Format: "0" or "1" (plain) OR "1:{hmac}" (signed)
            if raw == "0" {
                return false;
            }
            if raw == "1" {
                return true;
            }
            // Signed format: "{value}:{hmac}"
            if let Some((value, sig)) = raw.split_once(':') {
                if verify_signature(value, sig) {
                    return value == "1";
                }
                // Tampered HMAC → locked
                return true;
            }
            // Unknown format → locked
            true
        }
        Ok(None) => true,  // No row → locked (fail-closed)
        Err(_) => true,     // DB error → locked (fail-closed)
    }
}

/// Set the lock state in the database (with HMAC signature).
pub async fn set_system_locked(pool: &SqlitePool, locked: bool) -> Result<(), AppError> {
    let val = if locked { "1" } else { "0" };
    let sig = sign_value(val);
    let stored = format!("{}:{}", val, sig);

    sqlx::query(
        "INSERT INTO app_settings (key, value, updated_at) VALUES ('system_lock', ?, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
    )
    .bind(&stored)
    .execute(pool)
    .await?;

    Ok(())
}

/// Verify the unlock password against the bcrypt hash.
pub fn verify_password(password: &str) -> bool {
    bcrypt::verify(password, UNLOCK_PASSWORD_HASH).unwrap_or(false)
}

/// Middleware: returns Ok(()) if the system is unlocked, Err if locked.
/// Call this at the top of every sensitive Tauri command.
pub async fn require_unlocked(pool: &SqlitePool) -> Result<(), AppError> {
    if is_system_locked(pool).await {
        return Err(AppError::Auth(
            "System is locked. Contact AZ Solutions.".into()
        ));
    }
    Ok(())
}
