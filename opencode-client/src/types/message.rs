use serde::Deserialize;

use super::tool::ToolState;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User {
        id: String,
        sessionID: String,
        time: MessageTime,
        agent: String,
        model: MessageModel,
        system: Option<String>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        id: String,
        sessionID: String,
        time: MessageTime,
        error: Option<MessageError>,
        parentID: String,
        modelID: String,
        providerID: String,
        mode: String,
        path: AssistantPath,
        cost: f64,
        tokens: MessageTokens,
        finish: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageTime {
    pub created: u64,
    pub completed: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageModel {
    pub providerID: String,
    pub modelID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantPath {
    pub cwd: String,
    pub root: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageTokens {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    #[serde(rename = "cache")]
    pub cache: CacheUsage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheUsage {
    pub read: u64,
    pub write: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "name")]
pub enum MessageError {
    #[serde(rename = "ProviderAuthError")]
    ProviderAuth { data: serde_json::Value },
    #[serde(rename = "UnknownError")]
    Unknown { data: serde_json::Value },
    #[serde(rename = "MessageOutputLengthError")]
    OutputLength { data: serde_json::Value },
    #[serde(rename = "MessageAbortedError")]
    Aborted { data: serde_json::Value },
    #[serde(rename = "APIError")]
    Api { data: serde_json::Value },
}

/// Часть сообщения (часть потока).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename = "text")]
    Text {
        id: String,
        messageID: String,
        text: String,
        synthetic: Option<bool>,
        ignored: Option<bool>,
    },
    #[serde(rename = "reasoning")]
    Reasoning { id: String, text: String },
    #[serde(rename = "file")]
    File {
        id: String,
        mime: String,
        url: String,
    },
    #[serde(rename = "tool")]
    Tool {
        id: String,
        callID: String,
        tool: String,
        state: ToolState,
    },
    #[serde(rename = "step-start")]
    StepStart { id: String },
    #[serde(rename = "step-finish")]
    StepFinish { id: String, reason: String, cost: f64 },
    #[serde(rename = "snapshot")]
    Snapshot { id: String },
    #[serde(rename = "patch")]
    Patch { id: String, hash: String },
    #[serde(rename = "agent")]
    Agent { id: String, name: String },
    #[serde(rename = "retry")]
    Retry { id: String, attempt: u64 },
    #[serde(rename = "compaction")]
    Compaction { id: String, auto: bool },
    #[serde(rename = "subtask")]
    Subtask {
        id: String,
        prompt: String,
        description: String,
        agent: String,
    },
}