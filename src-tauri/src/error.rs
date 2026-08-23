// src-tauri/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Authorization failed: {0}")]
    Forbidden(String),

    #[error("User already exists: {0}")]
    Conflict(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired or invalid")]
    TokenInvalid,

    #[error("Password hashing failed: {0}")]
    PasswordHash(#[from] bcrypt::BcryptError),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("USB error: {0}")]
    Usb(#[from] rusb::Error),

    #[error("PDF generation failed: {0}")]
    Pdf(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;