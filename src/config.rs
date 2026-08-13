use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub port: u16,
    pub db_path: String,
    pub jwt_secret: String,
    pub admin_username: String,
    pub admin_password: String,
    pub token_expiry_hours: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 21114,
            db_path: "data/db.sqlite3".to_string(),
            jwt_secret: String::new(),
            admin_username: "admin".to_string(),
            admin_password: String::new(),
            token_expiry_hours: 168, // 7 days
        }
    }
}

fn random_jwt_secret() -> String {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

impl Config {
    pub fn load() -> Self {
        // Try loading from config.toml, fall back to defaults
        let config_path = PathBuf::from("config.toml");
        let mut config: Config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .expect("Failed to read config.toml");
            toml::from_str(&content).expect("Failed to parse config.toml")
        } else {
            Config::default()
        };

        // Environment variables override config file
        if let Ok(v) = std::env::var("RUSTDESK_AB_PORT") {
            config.port = v.parse().expect("Invalid RUSTDESK_AB_PORT");
        }
        if let Ok(v) = std::env::var("RUSTDESK_AB_DB_PATH") {
            config.db_path = v;
        }
        if let Ok(v) = std::env::var("RUSTDESK_AB_JWT_SECRET") {
            config.jwt_secret = v;
        }
        if let Ok(v) = std::env::var("RUSTDESK_AB_ADMIN_USERNAME") {
            config.admin_username = v;
        }
        if let Ok(v) = std::env::var("RUSTDESK_AB_ADMIN_PASSWORD") {
            config.admin_password = v;
        }

        // Warn only when the secret is genuinely missing after env+file resolution.
        if config.jwt_secret.is_empty() {
            tracing::warn!("No JWT secret configured — using random secret (sessions won't survive restarts)");
            config.jwt_secret = random_jwt_secret();
        }
        if config.admin_password.is_empty() {
            tracing::warn!(
                "No admin password configured — using default 'admin'. CHANGE IT IMMEDIATELY."
            );
            config.admin_password = "admin".to_string();
        }

        config
    }
}
