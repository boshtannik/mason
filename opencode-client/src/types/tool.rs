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

impl ToolState {
    /// Status for the chat feed: running | completed | error.
    pub fn ui_status(&self) -> &'static str {
        match self {
            ToolState::Pending { .. } | ToolState::Running { .. } => "running",
            ToolState::Completed { .. } => "completed",
            ToolState::Error { .. } => "error",
        }
    }

    /// Short description for the tool card in the chat.
    pub fn description(&self) -> String {
        match self {
            ToolState::Pending { input } | ToolState::Running { input } => input
                .get("command")
                .and_then(|v| v.as_str())
                .or_else(|| input.get("prompt").and_then(|v| v.as_str()))
                .or_else(|| input.get("filePath").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string(),
            ToolState::Completed { title, .. } => title.clone(),
            ToolState::Error { error, .. } => error.clone(),
        }
    }
}

/// Turn part of a message: a tool call with its state.
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