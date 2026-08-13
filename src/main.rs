#![allow(dead_code)]

mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod state;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::auth::password::hash_password;
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    let config = Config::load();
    tracing::info!("Starting RustDesk Address Book Server on port {}", config.port);

    // Initialize database
    let pool = db::init_pool(&config.db_path).await;
    db::run_migrations(&pool).await;

    // First-run: create admin user if no users exist
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if user_count == 0 {
        let password_hash = hash_password(&config.admin_password)
            .expect("Failed to hash admin password");

        sqlx::query(
            "INSERT INTO users (username, password_hash, name, is_admin) VALUES (?, ?, 'Administrator', TRUE)",
        )
        .bind(&config.admin_username)
        .bind(&password_hash)
        .execute(&pool)
        .await
        .expect("Failed to create admin user");

        tracing::info!(
            "Created default admin user: {} (change password immediately!)",
            config.admin_username
        );
    }

    let state = AppState {
        db: pool,
        config: config.clone(),
        login_attempts: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
    };

    // CORS — same-origin only (web console is served from this app).
    // Native RustDesk clients are not subject to CORS, so this does not
    // affect them; it stops random websites from calling the API.
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(routes::api_router())
        .fallback(routes::frontend::serve_frontend)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    tracing::info!("Listening on http://{}", addr);
    axum::serve(listener, app).await.expect("Server error");
}
