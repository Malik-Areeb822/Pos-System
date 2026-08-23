// src-tauri/src/auth/middleware.rs
use tauri::{State, AppHandle, Manager};
use crate::auth::jwt::{validate_token, Claims};
use crate::error::AppError;
use crate::repositories::UserRepository;
use crate::DbPool;

pub async fn get_current_user(app: &AppHandle, auth_header: Option<String>) -> Result<Claims, AppError> {
    let token = auth_header
        .and_then(|h| h.strip_prefix("Bearer ").map(|s| s.to_string()))
        .ok_or_else(|| AppError::Auth("Missing or invalid Authorization header".into()))?;

    let claims = validate_token(&token)?;
    Ok(claims)
}

pub async fn require_auth(app: &AppHandle, auth_header: Option<String>) -> Result<Claims, AppError> {
    get_current_user(app, auth_header).await
}

pub async fn require_admin(app: &AppHandle, auth_header: Option<String>) -> Result<Claims, AppError> {
    let claims = get_current_user(app, auth_header).await?;
    if !claims.has_role("admin") {
        return Err(AppError::Forbidden("Admin role required".into()));
    }
    Ok(claims)
}

pub async fn require_cashier_or_admin(app: &AppHandle, auth_header: Option<String>) -> Result<Claims, AppError> {
    let claims = get_current_user(app, auth_header).await?;
    if !claims.has_role("cashier") && !claims.has_role("admin") {
        return Err(AppError::Forbidden("Cashier or admin role required".into()));
    }
    Ok(claims)
}

pub async fn get_user_from_db(app: &AppHandle, user_id: &str) -> Result<crate::repositories::users::User, AppError> {
    let db = app.try_state::<crate::database::Db>()
        .ok_or_else(|| AppError::Internal("DB pool not available".into()))?;
    let repo = UserRepository::new(db.pool().await);
    repo.get_by_id(user_id).await?.ok_or(AppError::NotFound("User not found".into()))
}