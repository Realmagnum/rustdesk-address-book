use crate::config::Config;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Config,
    /// Login brute-force protection: username -> timestamps of recent attempts.
    pub login_attempts: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}
