// src-tauri/src/config.rs
use once_cell::sync::Lazy;
use std::path::PathBuf;

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
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "citytiles-pos-secret-change-in-production".to_string()),
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