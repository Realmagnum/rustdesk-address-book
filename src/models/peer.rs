use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Peer {
    pub id: i64,
    pub ab_guid: String,
    pub rustdesk_id: String,
    pub hash: String,
    pub username: String,
    pub hostname: String,
    pub platform: String,
    pub alias: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Peer as returned to the RustDesk client.
#[derive(Debug, Serialize)]
pub struct PeerPayload {
    pub id: String,
    pub hash: String,
    pub username: String,
    pub hostname: String,
    pub platform: String,
    pub alias: String,
    pub tags: Vec<String>,
    pub note: String,
}

/// Request to add a peer.
#[derive(Debug, Deserialize)]
pub struct AddPeerRequest {
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
    #[serde(default)]
    pub note: String,
}

/// Request to update a peer. RustDesk sends the peer ID plus only the fields
/// that changed, so every mutable field must remain optional.
#[derive(Debug, Deserialize)]
pub struct UpdatePeerRequest {
    pub id: String,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub note: Option<String>,
}

/// RustDesk 1.4.x sends the DELETE payload as a bare JSON array. Older callers
/// may send an object, so support both shapes at the API boundary.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum DeletePeersRequest {
    Ids(Vec<String>),
    Object {
        #[serde(default)]
        ids: Vec<String>,
        #[serde(default)]
        id: Option<String>,
    },
}

impl DeletePeersRequest {
    pub fn into_ids(self) -> Vec<String> {
        match self {
            Self::Ids(ids) => ids,
            Self::Object { mut ids, id } => {
                if let Some(id) = id {
                    ids.push(id);
                }
                ids
            }
        }
    }
}

/// Response for GET /api/ab/peers.
#[derive(Debug, Serialize)]
pub struct PeersResponse {
    pub data: Vec<PeerPayload>,
    pub total: i64,
}
