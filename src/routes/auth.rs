use axum::{extract::State, routing::{get, post}, Json, Router};
use serde_json::{json, Value};

use crate::auth::jwt::create_token;
use crate::auth::middleware::AuthUser;
use crate::auth::password::verify_password;
use crate::error::ApiError;
use crate::models::user::{LoginRequest, LoginResponse, UserPayload};
use crate::state::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Brute-force protection: max failed login attempts per username per window.
const LOGIN_MAX_ATTEMPTS: usize = 5;
const LOGIN_WINDOW_SECS: u64 = 60;

/// In-memory sliding-window rate limiter keyed by username.
/// Returns Err(TooManyRequests) once the limit is exceeded.
fn check_rate_limit(
    map: &Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    username: &str,
) -> Result<(), ApiError> {
    let mut attempts = map.lock().unwrap();
    let now = Instant::now();
    let window = Duration::from_secs(LOGIN_WINDOW_SECS);
    let history = attempts.entry(username.to_string()).or_default();
    history.retain(|t| now.duration_since(*t) < window);
    if history.len() >= LOGIN_MAX_ATTEMPTS {
        return Err(ApiError::TooManyRequests(
            "Too many login attempts, try again later".to_string(),
        ));
    }
    history.push(now);
    Ok(())
}

fn reset_rate_limit(map: &Arc<Mutex<HashMap<String, Vec<Instant>>>>, username: &str) {
    if let Ok(mut attempts) = map.lock() {
        attempts.remove(username);
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        // The RustDesk client POSTs to /api/currentUser; GET kept for API consumers.
        .route("/api/currentUser", get(current_user).post(current_user))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    check_rate_limit(&state.login_attempts, &req.username)?;

    let user = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT * FROM users WHERE username = ? AND status = 1",
    )
    .bind(&req.username)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::Unauthorized("Invalid username or password".to_string()))?;

    if !verify_password(&req.password, &user.password_hash) {
        return Err(ApiError::Unauthorized("Invalid username or password".to_string()));
    }

    // Successful login — clear the attempt history for this user.
    reset_rate_limit(&state.login_attempts, &req.username);

    let token = create_token(
        &user.username,
        user.id,
        user.is_admin,
        &state.config.jwt_secret,
        state.config.token_expiry_hours,
    )?;

    // Log the login in audit
    let ip = req.id.clone();
    sqlx::query("INSERT INTO audit_log (user_id, action, rustdesk_id, ip) VALUES (?, 'login', ?, ?)")
        .bind(user.id)
        .bind(&req.id)
        .bind(&ip)
        .execute(&state.db)
        .await
        .ok();

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "access_token".to_string(),
        user: UserPayload {
            name: if user.name.is_empty() {
                user.username.clone()
            } else {
                user.name
            },
            email: user.email,
            is_admin: user.is_admin,
            note: String::new(),
        },
    }))
}

async fn logout(AuthUser(_claims): AuthUser) -> Json<Value> {
    Json(json!({}))
}

async fn current_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    let user = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT * FROM users WHERE id = ?",
    )
    .bind(claims.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(json!({
        "verifier": "",
        "name": if user.name.is_empty() { user.username } else { user.name },
        "email": user.email,
        "is_admin": user.is_admin,
        "note": ""
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_blocks_after_max_attempts() {
        let map: Arc<Mutex<HashMap<String, Vec<Instant>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        for _ in 0..LOGIN_MAX_ATTEMPTS {
            assert!(check_rate_limit(&map, "admin").is_ok());
        }
        // Attempt past the limit within the window must be rejected.
        assert!(matches!(
            check_rate_limit(&map, "admin"),
            Err(ApiError::TooManyRequests(_))
        ));
        // A different user is not affected.
        assert!(check_rate_limit(&map, "other").is_ok());
    }

    #[test]
    fn rate_limit_resets_after_successful_login() {
        let map: Arc<Mutex<HashMap<String, Vec<Instant>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        for _ in 0..LOGIN_MAX_ATTEMPTS {
            let _ = check_rate_limit(&map, "admin");
        }
        assert!(check_rate_limit(&map, "admin").is_err());
        reset_rate_limit(&map, "admin");
        assert!(check_rate_limit(&map, "admin").is_ok());
    }
}
