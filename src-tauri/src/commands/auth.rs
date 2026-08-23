// src-tauri/src/commands/auth.rs
use tauri::{State, AppHandle};
use crate::auth::{create_token, hash_password, verify_password, require_admin, get_user_from_db};
use crate::repositories::{UserRepository, CreateUserInput};
use crate::DbPool;
use crate::error::AppError;
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginOutput {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub full_name: String,
    pub roles: Vec<String>,
    pub status: String,
}

#[derive(Deserialize)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub employee_id: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterOutput {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct AdminExistsOutput {
    pub exists: bool,
}

#[tauri::command]
pub async fn login(pool: State<'_, DbPool>, input: LoginInput) -> Result<LoginOutput, AppError> {
    let repo = UserRepository::new(pool.inner().clone());
    let user = repo.get_by_email(&input.email).await?
        .ok_or(AppError::InvalidCredentials)?;

    if user.status != "approved" {
        return Err(AppError::Auth("Account not approved".into()));
    }

    let valid = verify_password(&input.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    let token = create_token(user.id.clone(), user.email.clone(), user.roles.clone())?;
    
    Ok(LoginOutput {
        token,
        user: UserInfo {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            roles: user.roles,
            status: user.status,
        },
    })
}

#[tauri::command]
pub async fn register(app: AppHandle, pool: State<'_, DbPool>, input: RegisterInput) -> Result<RegisterOutput, AppError> {
    let repo = UserRepository::new(pool.inner().clone());
    
    // Check if email exists
    if repo.get_by_email(&input.email).await?.is_some() {
        return Err(AppError::Conflict("Email already registered".into()));
    }

    // Check if any user exists (first user = admin)
    let user_count = repo.count().await?;
    let (status, roles) = if user_count == 0 {
        ("approved".to_string(), vec!["admin".to_string()])
    } else {
        ("pending".to_string(), vec!["cashier".to_string()])
    };

    let password_hash = hash_password(&input.password)?;
    
    let new_user = CreateUserInput {
        email: input.email,
        password_hash,
        full_name: input.full_name,
        phone: input.phone,
        employee_id: input.employee_id,
        status,
        roles: roles.clone(),
    };

    let user = repo.create(new_user).await?;
    
    // Emit event if admin approved
    if user.status == "approved" {
        crate::events::emit_cashiers_changed(&app).await;
    }

    let token = create_token(user.id.clone(), user.email.clone(), user.roles.clone())?;
    
    Ok(RegisterOutput {
        token,
        user: UserInfo {
            id: user.id,
            email: user.email,
            full_name: user.full_name,
            roles: user.roles,
            status: user.status,
        },
    })
}

#[tauri::command]
pub async fn me(app: AppHandle, pool: State<'_, DbPool>, auth_header: Option<String>) -> Result<UserInfo, AppError> {
    let claims = crate::auth::middleware::get_current_user(&app, auth_header).await?;
    let repo = UserRepository::new(pool.inner().clone());
    let user = repo.get_by_id(&claims.sub).await?.ok_or(AppError::NotFound("User not found".into()))?;
    
    Ok(UserInfo {
        id: user.id,
        email: user.email,
        full_name: user.full_name,
        roles: user.roles,
        status: user.status,
    })
}

#[tauri::command]
pub async fn logout() -> Result<(), AppError> {
    // Client-side token removal
    Ok(())
}

#[tauri::command]
pub async fn check_admin_exists(pool: State<'_, DbPool>) -> Result<AdminExistsOutput, AppError> {
    let repo = UserRepository::new(pool.inner().clone());
    let count = repo.count().await?;
    Ok(AdminExistsOutput { exists: count > 0 })
}

#[tauri::command]
pub async fn get_roles(app: AppHandle, pool: State<'_, DbPool>, auth_header: Option<String>) -> Result<Vec<String>, AppError> {
    let claims = crate::auth::middleware::require_admin(&app, auth_header).await?;
    Ok(claims.roles)
}