use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status")]
pub enum ToolState {
    #[serde(rename = "pending")]
    Pending { input: serde_json::Value },
    #[serde(rename = "running")]
    Running { input: serde_json::Value },
    #[serde(rename = "completed")]
    Completed {
        input: serde_json::Value,
        output: String,
        title: String,
    },
    #[serde(rename = "error")]
    Error {
        input: serde_json::Value,
        error: String,
    },
}

/// Тур-часть сообщения: вызов инструмента с состоянием.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolPart {
    pub id: String,
    pub sessionID: String,
    pub messageID: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub callID: String,
    pub tool: String,
    pub state: ToolState,
}