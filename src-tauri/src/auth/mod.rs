// src-tauri/src/auth/mod.rs
pub mod jwt;
pub mod password;
pub mod middleware;

pub use jwt::{create_token, validate_token, Claims};
pub use password::{hash_password, verify_password};
pub use middleware::{require_auth, require_admin, require_cashier_or_admin, get_user_from_db};