use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Permission {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub pattern: Option<serde_json::Value>,
    pub sessionID: String,
    pub messageID: String,
    pub callID: Option<String>,
    pub title: String,
    pub metadata: serde_json::Value,
    pub time: PermissionTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionTime {
    pub created: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionResponse {
    pub response: String,
    pub remember: Option<bool>,
}