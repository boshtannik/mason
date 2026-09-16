use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "type")]
pub enum SessionStatus {
    #[default]
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "retry")]
    Retry {
        attempt: u32,
        message: String,
        next: u64,
    },
    #[serde(rename = "busy")]
    Busy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    pub id: String,
    pub projectID: String,
    pub directory: String,
    pub parentID: Option<String>,
    pub summary: Option<SessionSummary>,
    pub share: Option<SessionShare>,
    pub title: String,
    pub version: String,
    pub time: SessionTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionSummary {
    pub additions: u64,
    pub deletions: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionShare {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionTime {
    pub created: u64,
    pub updated: u64,
    pub compacting: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Todo {
    pub content: String,
    pub status: String,
    pub priority: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileDiff {
    pub file: String,
    pub before: String,
    pub after: String,
    pub additions: u64,
    pub deletions: u64,
}