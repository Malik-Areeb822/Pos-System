// src-tauri/src/auth/jwt.rs
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};
use crate::config::CONFIG;
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub roles: Vec<String>,
    pub exp: i64,
    pub iat: i64,
}

impl Claims {
    pub fn new(user_id: String, email: String, roles: Vec<String>) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(CONFIG.jwt_expiry_hours);
        Self {
            sub: user_id,
            email,
            roles,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

pub fn create_token(user_id: String, email: String, roles: Vec<String>) -> Result<String, AppError> {
    let claims = Claims::new(user_id, email, roles);
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(CONFIG.jwt_secret_bytes()))?;
    Ok(token)
}

pub fn validate_token(token: &str) -> Result<Claims, AppError> {
    let validation = Validation::new(Algorithm::HS256);
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(CONFIG.jwt_secret_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}