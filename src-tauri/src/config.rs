// src-tauri/src/config.rs
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};

pub static CONFIG: Lazy<Config> = Lazy::new(Config::default);

#[derive(Debug, Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub bcrypt_cost: u32,
    pub db_path: PathBuf,
    pub app_data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let program_data = std::env::var("PROGRAMDATA")
            .unwrap_or_else(|_| "C:\\ProgramData".to_string());
        let app_data_dir = PathBuf::from(program_data).join("CityTiles");

        Self {
            jwt_secret: resolve_jwt_secret(&app_data_dir),
            jwt_expiry_hours: 24,
            bcrypt_cost: 12,
            db_path: app_data_dir.join("citytiles.db"),
            app_data_dir,
        }
    }
}

impl Config {
    pub fn jwt_secret_bytes(&self) -> &[u8] {
        self.jwt_secret.as_bytes()
    }
}

/// Secret resolution order:
/// 1. `JWT_SECRET` environment variable (wins over everything)
/// 2. Persisted key file `%PROGRAMDATA%\CityTiles\jwt.key` — a 64-char hex
///    string (32 random bytes) generated once on first boot and reused on
///    every later boot so login sessions survive restarts.
/// 3. Ephemeral random secret if the key file cannot be written — the app
///    never bricks, but users are logged out on each restart instead of
///    shipping a known constant.
fn resolve_jwt_secret(app_data_dir: &Path) -> String {
    if let Ok(secret) = std::env::var("JWT_SECRET") {
        if !secret.trim().is_empty() {
            return secret;
        }
    }

    let key_path = app_data_dir.join("jwt.key");
    if let Ok(existing) = std::fs::read_to_string(&key_path) {
        let trimmed = existing.trim().to_string();
        if trimmed.len() >= 32 {
            return trimmed;
        }
    }

    let secret = random_hex_secret();
    // Defensive about directory existence regardless of init order.
    match std::fs::create_dir_all(app_data_dir)
        .and_then(|_| std::fs::write(&key_path, &secret))
    {
        Ok(()) => secret,
        Err(e) => {
            log::warn!(
                "Could not persist jwt.key ({}); using an ephemeral secret for this session",
                e
            );
            random_hex_secret()
        }
    }
}

/// 64 hex characters (256 bits of entropy) sourced from the OS RNG via uuid v4.
fn random_hex_secret() -> String {
    let a = uuid::Uuid::new_v4();
    let b = uuid::Uuid::new_v4();
    format!("{}{}", a.simple(), b.simple())
}
