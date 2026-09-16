use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub modified_at: i64,
    #[serde(default)]
    pub ver: i64,
}

#[derive(Debug, Deserialize)]
pub struct SysinfoRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub cpu: String,
    #[serde(default)]
    pub memory: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub username: String,
    // Client presets (OPTION_PRESET_ADDRESS_BOOK_*), used by auto-registration.
    #[serde(default, rename = "ab_alias")]
    pub ab_alias: String,
    #[serde(default, rename = "ab_tag")]
    pub ab_tag: String,
    #[serde(default, rename = "ab_note")]
    pub ab_note: String,
    #[serde(default, rename = "device_group_name")]
    pub device_group_name: String,
}

#[derive(Debug, Deserialize)]
pub struct AuditRequest {
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub id: String,
    #[serde(default, rename = "Id")]
    pub rustdesk_id: String,
    #[serde(default)]
    pub peer_id: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub note: String,
}

/// Device row as returned by GET /api/devices.
#[derive(Debug, sqlx::FromRow)]
pub struct DeviceRow {
    pub id: i64,
    pub rustdesk_id: String,
    pub hostname: String,
    pub platform: String,
    pub os: String,
    pub cpu: String,
    pub memory: String,
    pub version: String,
    pub username: String,
    pub last_online: String,
}
