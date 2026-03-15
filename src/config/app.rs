use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub api_key: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub database_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3010,
            api_key: std::env::var("API_KEY").unwrap_or_else(|_| "vibe-mcp-secret-key".to_string()),
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "vibe-jwt-secret-key-change-in-production".to_string()),
            jwt_expiry_hours: std::env::var("JWT_EXPIRY_HOURS").unwrap_or_else(|_| "24".to_string()).parse().unwrap_or(24),
            database_url: std::env::var("DATABASE_URL").unwrap_or_default(),
        }
    }
}
