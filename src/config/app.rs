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

impl Config {
    pub fn from_args<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut config = Self::default();
        let mut args = args.into_iter();

        let _ = args.next();

        while let Some(arg) = args.next() {
            let (key, value) = if let Some((key, value)) = arg.split_once('=') {
                (key.to_string(), value.to_string())
            } else {
                let value = args
                    .next()
                    .ok_or_else(|| format!("missing value for {}", arg))?;
                (arg, value)
            };

            match key.as_str() {
                "--host" => config.host = value,
                "--port" => {
                    config.port = value
                        .parse()
                        .map_err(|_| format!("invalid port: {}", value))?;
                }
                "--database-url" => config.database_url = value,
                "--api-key" => config.api_key = value,
                "--jwt-secret" => config.jwt_secret = value,
                "--jwt-expiry-hours" => {
                    config.jwt_expiry_hours = value
                        .parse()
                        .map_err(|_| format!("invalid jwt expiry hours: {}", value))?;
                }
                _ => {}
            }
        }

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(3010),
            api_key: std::env::var("API_KEY")
                .unwrap_or_else(|_| "vibe-mcp-secret-key".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "vibe-jwt-secret-key-change-in-production".to_string()),
            jwt_expiry_hours: std::env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            database_url: std::env::var("DATABASE_URL").unwrap_or_default(),
        }
    }
}
