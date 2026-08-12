use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AddressBook {
    pub guid: String,
    pub name: String,
    pub owner_id: i64,
    pub is_personal: bool,
    pub created_at: String,
}

/// Response for GET /api/ab/personal — matches RustDesk client expectation.
#[derive(Debug, Serialize)]
pub struct AbPersonalResponse {
    pub data: AbProfile,
}

#[derive(Debug, Serialize)]
pub struct AbProfile {
    pub guid: String,
    pub name: String,
    pub owner: String,
    pub rule: i32,
    pub note: String,
}

/// Response for GET /api/ab/shared/profiles.
#[derive(Debug, Serialize)]
pub struct AbSharedProfilesResponse {
    pub data: Vec<AbProfile>,
    pub total: i64,
}

/// Legacy address book format (GET /api/ab).
/// The `data` field is a JSON *string* containing serialized peers/tags.
#[derive(Debug, Serialize)]
pub struct LegacyAbResponse {
    pub data: String,
}

/// The inner content of the legacy address book data string.
#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyAbData {
    pub tags: Vec<String>,
    pub peers: Vec<LegacyPeer>,
    /// The official client does NOT send tag_colors on update —
    /// treat it as optional to stay compatible with current clients.
    #[serde(default = "default_tag_colors")]
    pub tag_colors: String,
}

fn default_tag_colors() -> String {
    "{}".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LegacyPeer {
    pub id: String,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub alias: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Legacy POST /api/ab request body.
#[derive(Debug, Deserialize)]
pub struct LegacyAbUpdateRequest {
    pub data: String,
}
