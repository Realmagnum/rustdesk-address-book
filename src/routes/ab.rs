use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::ApiError;
use crate::models::address_book::*;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ab", get(get_ab_legacy).post(update_ab_legacy))
        .route("/api/ab/get", post(get_ab_get))
        .route("/api/ab/personal", get(get_personal).post(get_personal))
        .route("/api/ab/shared/profiles", get(get_shared_profiles).post(get_shared_profiles))
        .route("/api/ab/settings", get(get_ab_settings).post(get_ab_settings))
        // Shared books management (owner / full-control only)
        .route("/api/ab/create", post(create_ab))
        .route("/api/ab/shares", get(list_shares))
        .route("/api/ab/share", post(upsert_share).delete(remove_share))
        .route("/api/ab/admin/profiles", get(get_admin_profiles))
}

/// Ensure the user has a personal address book, creating one if needed.
/// Returns the personal AB's guid.
async fn ensure_personal_ab(
    db: &sqlx::SqlitePool,
    user_id: i64,
    username: &str,
) -> Result<String, ApiError> {
    let existing = sqlx::query_scalar::<_, String>(
        "SELECT guid FROM address_books WHERE owner_id = ? AND is_personal = TRUE",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    if let Some(guid) = existing {
        return Ok(guid);
    }

    let guid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES (?, 'Personal', ?, TRUE)")
        .bind(&guid)
        .bind(user_id)
        .execute(db)
        .await?;

    tracing::info!("Created personal address book for user {}", username);
    Ok(guid)
}

/// GET /api/ab/personal — returns the user's personal address book profile.
async fn get_personal(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<AbPersonalResponse>, ApiError> {
    let guid = ensure_personal_ab(&state.db, claims.user_id, &claims.sub).await?;

    Ok(Json(AbPersonalResponse {
        data: AbProfile {
            guid,
            name: "Personal".to_string(),
            owner: claims.sub,
            rule: 3, // full control
            note: String::new(),
        },
    }))
}

/// GET /api/ab — legacy endpoint, returns the entire address book as a JSON string.
async fn get_ab_legacy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<LegacyAbResponse>, ApiError> {
    let ab_data = build_legacy_ab(&state, claims.user_id, &claims.sub).await?;
    let data_str = serde_json::to_string(&ab_data)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(LegacyAbResponse { data: data_str }))
}

/// Response for POST /api/ab/get — the endpoint the current RustDesk client
/// actually calls when opening the address book tab.
#[derive(Serialize)]
struct AbGetResponse {
    updated_at: i64,
    data: String,
}

/// POST /api/ab/get — client-compatible address book fetch.
/// The official RustDesk client (ab.tis getAb) POSTs to /api/ab/get and
/// expects { updated_at, data: "<stringified {tags,peers,tag_colors}>" }.
async fn get_ab_get(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<AbGetResponse>, ApiError> {
    let ab_data = build_legacy_ab(&state, claims.user_id, &claims.sub).await?;
    let data_str = serde_json::to_string(&ab_data)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(AbGetResponse {
        updated_at: chrono::Utc::now().timestamp(),
        data: data_str,
    }))
}

/// Shared logic: build the legacy address book payload for a user.
async fn build_legacy_ab(
    state: &AppState,
    user_id: i64,
    username: &str,
) -> Result<LegacyAbData, ApiError> {
    let guid = ensure_personal_ab(&state.db, user_id, username).await?;

    // Fetch all peers
    let peers = sqlx::query_as::<_, crate::models::peer::Peer>(
        "SELECT * FROM peers WHERE ab_guid = ? ORDER BY rustdesk_id",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;

    // Fetch all tags
    let tags = sqlx::query_as::<_, crate::models::tag::Tag>(
        "SELECT * FROM tags WHERE ab_guid = ? ORDER BY name",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;

    // Build tag names list
    let tag_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();

    // Build tag_colors map as a JSON string
    let mut tag_colors_map = serde_json::Map::new();
    for tag in &tags {
        tag_colors_map.insert(tag.name.clone(), json!(tag.color));
    }
    let tag_colors_str = serde_json::to_string(&tag_colors_map).unwrap_or_default();

    // Build legacy peers with their tags
    let mut legacy_peers = Vec::new();
    for peer in &peers {
        let peer_tags: Vec<String> = sqlx::query_scalar(
            "SELECT t.name FROM tags t JOIN peer_tags pt ON t.id = pt.tag_id WHERE pt.peer_id = ?",
        )
        .bind(peer.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        legacy_peers.push(LegacyPeer {
            id: peer.rustdesk_id.clone(),
            hash: peer.hash.clone(),
            username: peer.username.clone(),
            hostname: peer.hostname.clone(),
            platform: peer.platform.clone(),
            alias: peer.alias.clone(),
            tags: peer_tags,
        });
    }

    Ok(LegacyAbData {
        tags: tag_names,
        peers: legacy_peers,
        tag_colors: tag_colors_str,
    })
}

/// POST /api/ab — legacy endpoint, replaces the entire address book from a JSON string.
async fn update_ab_legacy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<LegacyAbUpdateRequest>,
) -> Result<Json<Value>, ApiError> {
    let guid = ensure_personal_ab(&state.db, claims.user_id, &claims.sub).await?;

    let ab_data: LegacyAbData = serde_json::from_str(&req.data)
        .map_err(|e| ApiError::BadRequest(format!("Invalid address book data: {}", e)))?;

    // Parse tag_colors
    let tag_colors: std::collections::HashMap<String, i64> =
        serde_json::from_str(&ab_data.tag_colors).unwrap_or_default();

    // Delete existing data and replace atomically
    // Delete peer_tags first (foreign key), then peers, then tags
    sqlx::query("DELETE FROM peer_tags WHERE peer_id IN (SELECT id FROM peers WHERE ab_guid = ?)")
        .bind(&guid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM peers WHERE ab_guid = ?")
        .bind(&guid)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM tags WHERE ab_guid = ?")
        .bind(&guid)
        .execute(&state.db)
        .await?;

    // Insert tags
    for tag_name in &ab_data.tags {
        let color = tag_colors.get(tag_name).copied().unwrap_or(4278190080);
        sqlx::query("INSERT INTO tags (ab_guid, name, color) VALUES (?, ?, ?)")
            .bind(&guid)
            .bind(tag_name)
            .bind(color)
            .execute(&state.db)
            .await?;
    }

    // Insert peers and their tag associations
    for peer in &ab_data.peers {
        let result = sqlx::query(
            "INSERT INTO peers (ab_guid, rustdesk_id, hash, username, hostname, platform, alias) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&guid)
        .bind(&peer.id)
        .bind(&peer.hash)
        .bind(&peer.username)
        .bind(&peer.hostname)
        .bind(&peer.platform)
        .bind(&peer.alias)
        .execute(&state.db)
        .await?;

        let peer_id = result.last_insert_rowid();

        for tag_name in &peer.tags {
            let tag_id: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM tags WHERE ab_guid = ? AND name = ?",
            )
            .bind(&guid)
            .bind(tag_name)
            .fetch_optional(&state.db)
            .await?;

            if let Some(tag_id) = tag_id {
                sqlx::query("INSERT OR IGNORE INTO peer_tags (peer_id, tag_id) VALUES (?, ?)")
                    .bind(peer_id)
                    .bind(tag_id)
                    .execute(&state.db)
                    .await?;
            }
        }
    }

    Ok(Json(json!({})))
}

/// GET /api/ab/shared/profiles — returns shared address books the user has access to.
async fn get_shared_profiles(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<AbSharedProfilesResponse>, ApiError> {
    // Find address books shared with this user directly or via groups
    let shared: Vec<(String, String, i64, i32)> = sqlx::query_as(
        "SELECT ab.guid, ab.name, ab.owner_id, s.rule
         FROM address_books ab
         JOIN ab_shares s ON ab.guid = s.ab_guid
         WHERE (s.user_id = ? OR s.group_id IN (SELECT group_id FROM user_groups WHERE user_id = ?))
         AND ab.is_personal = FALSE",
    )
    .bind(claims.user_id)
    .bind(claims.user_id)
    .fetch_all(&state.db)
    .await?;

    let mut profiles = Vec::new();
    for (guid, name, owner_id, rule) in shared {
        let owner_name: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?")
            .bind(owner_id)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or_default();

        profiles.push(AbProfile {
            guid,
            name,
            owner: owner_name,
            rule,
            note: String::new(),
        });
    }

    let total = profiles.len() as i64;
    Ok(Json(AbSharedProfilesResponse {
        data: profiles,
        total,
    }))
}

/// GET /api/ab/settings — address book configuration.
async fn get_ab_settings(AuthUser(_claims): AuthUser) -> Json<Value> {
    Json(json!({
        "max_peer_one_ab": 0
    }))
}

// ================= Access control =================

/// Effective access rule for a user on an address book.
/// Returns `(guid, rule)` where rule: 1=read-only, 2=read-write, 3=full (admin).
/// Owner always gets rule 3; otherwise the maximum rule from direct or group shares.
pub async fn resolve_ab_access(
    db: &sqlx::SqlitePool,
    user_id: i64,
    ab_guid: &str,
) -> Result<(String, i32), ApiError> {
    if ab_guid.is_empty() {
        // Personal book — full control for its owner.
        let guid: Option<String> = sqlx::query_scalar(
            "SELECT guid FROM address_books WHERE owner_id = ? AND is_personal = TRUE",
        )
        .bind(user_id)
        .fetch_optional(db)
        .await?;
        let guid = guid
            .ok_or_else(|| ApiError::NotFound("No personal address book found".to_string()))?;
        return Ok((guid, 3));
    }

    let row: Option<(String, i32)> = sqlx::query_as(
        "SELECT ab.guid,
                CASE WHEN ab.owner_id = ? THEN 3
                     ELSE COALESCE((SELECT MAX(s.rule) FROM ab_shares s
                                    WHERE s.ab_guid = ab.guid
                                      AND (s.user_id = ? OR s.group_id IN (SELECT group_id FROM user_groups WHERE user_id = ?))), 0)
                END AS rule
         FROM address_books ab
         WHERE ab.guid = ?",
    )
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(ab_guid)
    .fetch_optional(db)
    .await?;

    let (guid, rule) = row
        .ok_or_else(|| ApiError::NotFound("Address book not found".to_string()))?;
    if rule <= 0 {
        return Err(ApiError::Forbidden("Access denied to this address book".to_string()));
    }
    Ok((guid, rule))
}

/// Read access (rule >= 1). Returns the resolved guid.
pub async fn resolve_ab_guid(
    db: &sqlx::SqlitePool,
    user_id: i64,
    ab_guid: &str,
) -> Result<String, ApiError> {
    Ok(resolve_ab_access(db, user_id, ab_guid).await?.0)
}

/// Write access (rule >= 2). Returns the resolved guid.
pub async fn resolve_ab_write(
    db: &sqlx::SqlitePool,
    user_id: i64,
    ab_guid: &str,
) -> Result<String, ApiError> {
    let (guid, rule) = resolve_ab_access(db, user_id, ab_guid).await?;
    if rule < 2 {
        return Err(ApiError::Forbidden("Address book is read-only".to_string()));
    }
    Ok(guid)
}

/// Full access (rule >= 3, owner or admin share). Returns the resolved guid.
pub async fn resolve_ab_admin(
    db: &sqlx::SqlitePool,
    user_id: i64,
    ab_guid: &str,
) -> Result<String, ApiError> {
    let (guid, rule) = resolve_ab_access(db, user_id, ab_guid).await?;
    if rule < 3 {
        return Err(ApiError::Forbidden("Administrator access required".to_string()));
    }
    Ok(guid)
}

// ================= Shared book management =================

#[derive(Debug, Deserialize)]
struct CreateAbRequest {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SharesQuery {
    #[serde(default)]
    ab: String,
}

#[derive(Debug, Deserialize)]
struct ShareRequest {
    ab_guid: String,
    #[serde(default)]
    user_id: Option<i64>,
    #[serde(default)]
    group_id: Option<i64>,
    #[serde(default)]
    rule: i32,
}

/// POST /api/ab/create — create a shared address book (the caller becomes owner).
async fn create_ab(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateAbRequest>,
) -> Result<Json<Value>, ApiError> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Name is required".to_string()));
    }
    let guid = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES (?, ?, ?, FALSE)",
    )
    .bind(&guid)
    .bind(name)
    .bind(claims.user_id)
    .execute(&state.db)
    .await?;
    Ok(Json(json!({ "guid": guid, "name": name })))
}

/// GET /api/ab/shares?ab=<guid> — list shares of a book (admin only).
async fn list_shares(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(query): Query<SharesQuery>,
) -> Result<Json<Value>, ApiError> {
    let guid = resolve_ab_admin(&state.db, claims.user_id, &query.ab).await?;
    let rows: Vec<(String, Option<String>, String, i32)> = sqlx::query_as(
        "SELECT s.ab_guid,
                COALESCE(u.username, g.name, '') AS target,
                CASE WHEN s.user_id IS NOT NULL THEN 'user' ELSE 'group' END AS kind,
                s.rule
         FROM ab_shares s
         LEFT JOIN users u ON s.user_id = u.id
         LEFT JOIN groups g ON s.group_id = g.id
         WHERE s.ab_guid = ?",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;

    let shares: Vec<Value> = rows
        .into_iter()
        .map(|(ab, target, kind, rule)| {
            json!({ "ab_guid": ab, "target": target, "kind": kind, "rule": rule })
        })
        .collect();
    Ok(Json(json!({ "data": shares })))
}

/// POST /api/ab/share — create or update a share (admin only).
async fn upsert_share(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ShareRequest>,
) -> Result<Json<Value>, ApiError> {
    let guid = resolve_ab_admin(&state.db, claims.user_id, &req.ab_guid).await?;
    if req.user_id.is_none() && req.group_id.is_none() {
        return Err(ApiError::BadRequest("user_id or group_id is required".to_string()));
    }
    if !(1..=3).contains(&req.rule) {
        return Err(ApiError::BadRequest("rule must be 1 (read), 2 (read-write) or 3 (admin)".to_string()));
    }
    let user_id = req.user_id;
    let group_id = req.group_id;
    // Upsert: remove existing share for the same target, then insert.
    sqlx::query(
        "DELETE FROM ab_shares WHERE ab_guid = ? AND user_id IS ? AND group_id IS ?",
    )
    .bind(&guid)
    .bind(user_id)
    .bind(group_id)
    .execute(&state.db)
    .await?;
    sqlx::query("INSERT INTO ab_shares (ab_guid, user_id, group_id, rule) VALUES (?, ?, ?, ?)")
        .bind(&guid)
        .bind(user_id)
        .bind(group_id)
        .bind(req.rule)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({})))
}

/// DELETE /api/ab/share — remove a share (admin only).
async fn remove_share(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ShareRequest>,
) -> Result<Json<Value>, ApiError> {
    let guid = resolve_ab_admin(&state.db, claims.user_id, &req.ab_guid).await?;
    let user_id = req.user_id;
    let group_id = req.group_id;
    if user_id.is_none() && group_id.is_none() {
        return Err(ApiError::BadRequest("user_id or group_id is required".to_string()));
    }
    sqlx::query("DELETE FROM ab_shares WHERE ab_guid = ? AND user_id IS ? AND group_id IS ?")
        .bind(&guid)
        .bind(user_id)
        .bind(group_id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({})))
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

    async fn seed_user(pool: &sqlx::SqlitePool, username: &str, is_admin: bool) -> i64 {
        let hash = crate::auth::password::hash_password("pw").unwrap();
        sqlx::query(
            "INSERT INTO users (username, password_hash, name, is_admin) VALUES (?, ?, ?, ?)",
        )
        .bind(username)
        .bind(&hash)
        .bind(username)
        .bind(is_admin)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn owner_gets_full_access() {
        let pool = test_db().await;
        let uid = seed_user(&pool, "alice", true).await;
        let guid = "book-1";
        sqlx::query(
            "INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES (?, 'Shared', ?, FALSE)",
        )
        .bind(guid)
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();
        let (resolved, rule) = resolve_ab_access(&pool, uid, guid).await.unwrap();
        assert_eq!(resolved, guid);
        assert_eq!(rule, 3);
    }

    #[tokio::test]
    async fn stranger_is_denied() {
        let pool = test_db().await;
        let owner = seed_user(&pool, "alice", false).await;
        let uid = seed_user(&pool, "bob", false).await;
        let guid = "book-2";
        sqlx::query(
            "INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES (?, 'Shared', ?, FALSE)",
        )
        .bind(guid)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            resolve_ab_access(&pool, uid, guid).await,
            Err(ApiError::Forbidden(_))
        ));
    }

    #[tokio::test]
    async fn share_grants_rule_and_blocks_admin() {
        let pool = test_db().await;
        let owner = seed_user(&pool, "alice", false).await;
        let uid = seed_user(&pool, "bob", false).await;
        let guid = "book-3";
        sqlx::query(
            "INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES (?, 'Shared', ?, FALSE)",
        )
        .bind(guid)
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ab_shares (ab_guid, user_id, rule) VALUES (?, ?, 2)")
            .bind(guid)
            .bind(uid)
            .execute(&pool)
            .await
            .unwrap();
        let (_, rule) = resolve_ab_access(&pool, uid, guid).await.unwrap();
        assert_eq!(rule, 2);
        assert!(resolve_ab_write(&pool, uid, guid).await.is_ok());
        assert!(matches!(
            resolve_ab_admin(&pool, uid, guid).await,
            Err(ApiError::Forbidden(_))
        ));
    }

    #[tokio::test]
    async fn empty_guid_resolves_personal_book() {
        let pool = test_db().await;
        let uid = seed_user(&pool, "alice", false).await;
        sqlx::query(
            "INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES ('p1', 'Personal', ?, TRUE)",
        )
        .bind(uid)
        .execute(&pool)
        .await
        .unwrap();
        let (guid, rule) = resolve_ab_access(&pool, uid, "").await.unwrap();
        assert_eq!(guid, "p1");
        assert_eq!(rule, 3);
    }
}

/// GET /api/ab/admin/profiles — address books the user owns or administers
/// (owner, or share with rule 3). Used by the web console's Shared Books view.
async fn get_admin_profiles(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows: Vec<(String, String, String, i32, bool)> = sqlx::query_as(
        "SELECT ab.guid, ab.name,
                COALESCE(u.username, '') AS owner,
                CASE WHEN ab.owner_id = ? THEN 3
                     ELSE COALESCE((SELECT MAX(s.rule) FROM ab_shares s
                                    WHERE s.ab_guid = ab.guid
                                      AND (s.user_id = ? OR s.group_id IN (SELECT group_id FROM user_groups WHERE user_id = ?))), 0)
                END AS rule,
                ab.is_personal
         FROM address_books ab
         LEFT JOIN users u ON ab.owner_id = u.id
         WHERE ab.owner_id = ?
            OR EXISTS (SELECT 1 FROM ab_shares s WHERE s.ab_guid = ab.guid
                       AND (s.user_id = ? OR s.group_id IN (SELECT group_id FROM user_groups WHERE user_id = ?))
                       AND s.rule >= 3)",
    )
    .bind(claims.user_id)
    .bind(claims.user_id)
    .bind(claims.user_id)
    .bind(claims.user_id)
    .bind(claims.user_id)
    .bind(claims.user_id)
    .fetch_all(&state.db)
    .await?;

    let books: Vec<Value> = rows
        .into_iter()
        .filter(|(_, _, _, rule, _)| *rule >= 1)
        .map(|(guid, name, owner, rule, is_personal)| {
            json!({ "guid": guid, "name": name, "owner": owner, "rule": rule, "is_personal": is_personal })
        })
        .collect();
    Ok(Json(json!({ "data": books })))
}
