use axum::{extract::State, routing::{get, post}, Json, Router};
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::error::ApiError;
use crate::models::device::*;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/heartbeat", post(heartbeat))
        .route("/api/system/heartbeat", post(heartbeat))
        .route("/api/system/sysinfo", post(sysinfo))
        .route("/api/audit", post(audit))
        // Official client endpoints (hbbs_http/sync.rs) + device inventory
        .route("/api/sysinfo", post(sysinfo_upload))
        .route("/api/sysinfo_ver", post(sysinfo_ver))
        .route("/api/devices", get(list_devices))
}

async fn heartbeat(
    State(state): State<AppState>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<Value>, ApiError> {
    if !req.id.is_empty() {
        sqlx::query(
            "INSERT INTO devices (rustdesk_id, last_online)
             VALUES (?, CURRENT_TIMESTAMP)
             ON CONFLICT(rustdesk_id) DO UPDATE SET last_online = CURRENT_TIMESTAMP",
        )
        .bind(&req.id)
        .execute(&state.db)
        .await?;
    }

    // The client expects a modified_at field back
    Ok(Json(json!({ "modified_at": "" })))
}

async fn sysinfo(
    State(state): State<AppState>,
    Json(req): Json<SysinfoRequest>,
) -> Result<Json<Value>, ApiError> {
    if !req.id.is_empty() {
        sqlx::query(
            "INSERT INTO devices (rustdesk_id, hostname, platform, os, cpu, memory, version, last_online)
             VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(rustdesk_id) DO UPDATE SET
                 hostname = excluded.hostname,
                 platform = excluded.platform,
                 os = excluded.os,
                 cpu = excluded.cpu,
                 memory = excluded.memory,
                 version = excluded.version,
                 last_online = CURRENT_TIMESTAMP",
        )
        .bind(&req.id)
        .bind(&req.hostname)
        .bind(&req.platform)
        .bind(&req.os)
        .bind(&req.cpu)
        .bind(&req.memory)
        .bind(&req.version)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(json!({})))
}

async fn audit(
    State(state): State<AppState>,
    Json(req): Json<AuditRequest>,
) -> Result<Json<Value>, ApiError> {
    let rustdesk_id = if req.rustdesk_id.is_empty() {
        &req.id
    } else {
        &req.rustdesk_id
    };

    sqlx::query(
        "INSERT INTO audit_log (action, rustdesk_id, peer_id, ip, note) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&req.action)
    .bind(rustdesk_id)
    .bind(&req.peer_id)
    .bind(&req.ip)
    .bind(&req.note)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({})))
}

// ================= Auto-registration (official client) =================
//
// The official RustDesk client (hbbs_http/sync.rs) posts its full sysinfo to
// `/api/sysinfo` (heartbeat_url with "heartbeat" replaced by "sysinfo") and
// polls `/api/sysinfo_ver` to skip redundant uploads. Neither endpoint existed,
// so devices never registered — the same API-drift bug class as the address
// book endpoints.

/// POST /api/sysinfo — full sysinfo upload from the client.
/// The client sends a JSON object with system info plus optional presets
/// (ab_alias/ab_tag/ab_note/device_group_name); unknown fields are ignored.
async fn sysinfo_upload(
    State(state): State<AppState>,
    Json(req): Json<SysinfoRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = req.id.clone();
    if id.is_empty() {
        return Ok(Json(json!({ "modified_at": "" })));
    }

    sqlx::query(
        "INSERT INTO devices (rustdesk_id, hostname, platform, os, cpu, memory, version, username, last_online)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(rustdesk_id) DO UPDATE SET
             hostname = excluded.hostname,
             platform = excluded.platform,
             os = excluded.os,
             cpu = excluded.cpu,
             memory = excluded.memory,
             version = excluded.version,
             username = excluded.username,
             last_online = CURRENT_TIMESTAMP",
    )
    .bind(&id)
    .bind(&req.hostname)
    .bind(&req.platform)
    .bind(&req.os)
    .bind(&req.cpu)
    .bind(&req.memory)
    .bind(&req.version)
    .bind(&req.username)
    .execute(&state.db)
    .await?;

    if state.config.auto_add_devices {
        auto_add_device(
            &state.db,
            &id,
            &req.hostname,
            &req.username,
            &req.platform,
            &req.ab_alias,
            &req.ab_tag,
            &req.ab_note,
        )
        .await?;
    }

    // The client expects a modified_at field back.
    Ok(Json(json!({ "modified_at": "" })))
}

/// POST /api/sysinfo_ver — return a version token. The client skips re-uploading
/// unchanged sysinfo after the first successful upload (hash-based check), so
/// returning "" is correct: devices re-register only when their info changes.
async fn sysinfo_ver() -> Json<Value> {
    Json(json!(""))
}

/// Auto-add (or update) a registered device in the first admin's personal book.
/// Applies client-side presets: ab_alias -> alias, ab_tag -> tag, ab_note -> note.
///
/// NOTE: no closure/trait-object parameters — `&dyn Fn` without `Send` held
/// across an await makes the handler future non-Send, which axum rejects
/// (Handler bound E0277).
async fn auto_add_device(
    db: &sqlx::SqlitePool,
    id: &str,
    hostname: &str,
    username: &str,
    platform: &str,
    ab_alias: &str,
    ab_tag: &str,
    ab_note: &str,
) -> Result<(), ApiError> {
    let owner: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM users WHERE is_admin = TRUE ORDER BY id LIMIT 1",
    )
    .fetch_optional(db)
    .await?;
    let Some(owner_id) = owner else { return Ok(()); };

    let guid: Option<String> = sqlx::query_scalar(
        "SELECT guid FROM address_books WHERE owner_id = ? AND is_personal = TRUE",
    )
    .bind(owner_id)
    .fetch_optional(db)
    .await?;
    let Some(guid) = guid else { return Ok(()); };

    let alias = if ab_alias.is_empty() { hostname.to_string() } else { ab_alias.to_string() };

    sqlx::query(
        "INSERT INTO peers (ab_guid, rustdesk_id, hash, username, hostname, platform, alias, note)
         VALUES (?, ?, '', ?, ?, ?, ?, ?)
         ON CONFLICT(ab_guid, rustdesk_id) DO UPDATE SET
             hostname = CASE WHEN excluded.hostname <> '' THEN excluded.hostname ELSE peers.hostname END,
             username = CASE WHEN excluded.username <> '' THEN excluded.username ELSE peers.username END,
             platform = CASE WHEN excluded.platform <> '' THEN excluded.platform ELSE peers.platform END,
             alias = CASE WHEN peers.alias = '' THEN excluded.alias ELSE peers.alias END,
             note = CASE WHEN excluded.note <> '' THEN excluded.note ELSE peers.note END",
    )
    .bind(&guid)
    .bind(id)
    .bind(username)
    .bind(hostname)
    .bind(platform)
    .bind(&alias)
    .bind(ab_note)
    .execute(db)
    .await?;

    let tag = ab_tag;
    if !tag.is_empty() {
        let existing: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM tags WHERE ab_guid = ? AND name = ?",
        )
        .bind(&guid)
        .bind(&tag)
        .fetch_optional(db)
        .await?;
        let tag_id = match existing {
            Some(t) => t,
            None => {
                sqlx::query("INSERT INTO tags (ab_guid, name, color) VALUES (?, ?, 4286141768)")
                    .bind(&guid)
                    .bind(&tag)
                    .execute(db)
                    .await?;
                sqlx::query_scalar("SELECT id FROM tags WHERE ab_guid = ? AND name = ?")
                    .bind(&guid)
                    .bind(&tag)
                    .fetch_one(db)
                    .await?
            }
        };
        let peer_id: i64 = sqlx::query_scalar(
            "SELECT id FROM peers WHERE ab_guid = ? AND rustdesk_id = ?",
        )
        .bind(&guid)
        .bind(id)
        .fetch_one(db)
        .await?;
        sqlx::query("INSERT OR IGNORE INTO peer_tags (peer_id, tag_id) VALUES (?, ?)")
            .bind(peer_id)
            .bind(tag_id)
            .execute(db)
            .await?;
    }
    Ok(())
}

/// GET /api/devices — list registered devices with online status (auth required).
async fn list_devices(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    let devices: Vec<crate::models::device::DeviceRow> = sqlx::query_as(
        "SELECT id, rustdesk_id, hostname, platform, os, cpu, memory, version, username, last_online
         FROM devices ORDER BY last_online DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let now = chrono::Utc::now().naive_utc();
    let items: Vec<Value> = devices
        .into_iter()
        .map(|d| {
            let online = d
                .last_online
                .parse::<chrono::NaiveDateTime>()
                .map(|t| now.signed_duration_since(t).num_seconds() < 90)
                .unwrap_or(false);
            json!({
                "id": d.rustdesk_id,
                "hostname": d.hostname,
                "platform": d.platform,
                "os": d.os,
                "version": d.version,
                "username": d.username,
                "cpu": d.cpu,
                "memory": d.memory,
                "last_online": d.last_online,
                "online": online,
            })
        })
        .collect();
    Ok(Json(json!({ "data": items, "total": items.len() })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn test_db() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        db::run_migrations(&pool).await;
        pool
    }

    #[tokio::test]
    async fn auto_add_creates_peer_alias_and_tag() {
        let pool = test_db().await;
        let hash = crate::auth::password::hash_password("pw").unwrap();
        sqlx::query(
            "INSERT INTO users (username, password_hash, name, is_admin) VALUES ('admin', ?, 'Administrator', TRUE)",
        )
        .bind(&hash)
        .execute(&pool)
        .await
        .unwrap();
        let uid: i64 =
            sqlx::query_scalar("SELECT id FROM users WHERE username='admin'")
                .fetch_one(&pool)
                .await
                .unwrap();
        sqlx::query(
            "INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES ('p1', 'Personal', ?, TRUE)",
        )
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();

        auto_add_device(&pool, "999888777", "macbook-pro", "magnum", "macos", "MyMac", "srv", "")
            .await
            .unwrap();
        let (id, alias): (String, String) =
            sqlx::query_as("SELECT rustdesk_id, alias FROM peers")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(id, "999888777");
        assert_eq!(alias, "MyMac");

        let tag: Option<String> =
            sqlx::query_scalar("SELECT name FROM tags").fetch_optional(&pool).await.unwrap();
        assert_eq!(tag.as_deref(), Some("srv"));

        let links: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM peer_tags").fetch_one(&pool).await.unwrap();
        assert_eq!(links, 1);
    }
}
