use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/peers", get(list_peers))
        .route("/api/device-group/accessible", get(list_accessible_groups))
}

fn require_admin(claims: &crate::auth::jwt::Claims) -> Result<(), ApiError> {
    if !claims.is_admin {
        return Err(ApiError::Forbidden(
            "Administrator access required".to_string(),
        ));
    }
    Ok(())
}

async fn list_peers(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&claims)?;

    let devices: Vec<(String, String, String, String, String)> =
        sqlx::query_as(
            "SELECT rustdesk_id, hostname, os, username, platform FROM devices ORDER BY rustdesk_id",
        )
        .fetch_all(&state.db)
        .await?;

    let items: Vec<Value> = devices
        .into_iter()
        .map(|(id, hostname, os, username, platform)| {
            json!({
                "id": id,
                "info": {
                    "device_name": hostname,
                    "username": username,
                    "os": os,
                    "platform": platform
                },
                "status": 1,
                "user": "",
                "user_name": "",
                "device_group_name": "",
                "note": ""
            })
        })
        .collect();

    let total = items.len();
    Ok(Json(json!({ "data": items, "total": total })))
}

async fn list_accessible_groups(
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&claims)?;
    Ok(Json(json!({ "data": [], "total": 0 })))
}
