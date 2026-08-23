// src-tauri/src/auth/password.rs
use bcrypt::{hash, verify, DEFAULT_COST};
use crate::config::CONFIG;
use crate::error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let hash = hash(password, CONFIG.bcrypt_cost)?;
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let valid = verify(password, hash)?;
    Ok(valid)
}