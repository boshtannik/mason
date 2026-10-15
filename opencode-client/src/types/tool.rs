use serde::Deserialize;

/// Tool-call status values: the `status` discriminator of [`ToolState`] and the
/// value carried in the `[[tool]]` feed line. QML mirrors these as constants.
pub const STATUS_PENDING: &str = "pending";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_ERROR: &str = "error";

/// Tool-input JSON keys probed, in order, for the short display label. Named so
/// the lookup order is a single source of truth rather than literals in a loop.
pub const LABEL_INPUT_KEYS: &[&str] = &[
    "command",
    "filePath",
    "path",
    "pattern",
    "query",
    "url",
    "prompt",
    "description",
];

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
            ToolState::Pending { .. } | ToolState::Running { .. } => STATUS_RUNNING,
            ToolState::Completed { .. } => STATUS_COMPLETED,
            ToolState::Error { .. } => STATUS_ERROR,
        }
    }

    /// The tool-call input (pending/running), or `Null` for terminal states.
    pub fn input(&self) -> &serde_json::Value {
        match self {
            ToolState::Pending { input }
            | ToolState::Running { input }
            | ToolState::Completed { input, .. }
            | ToolState::Error { input, .. } => input,
        }
    }

    /// Short description for the tool row in the chat (inline in the
    /// transcript). Pulls the human-relevant field out of the tool input
    /// (command / path / pattern / …) and returns it for EVERY state, so the
    /// label does not change as the call goes pending → running → completed.
    /// Falls back to the terminal title/error when the input has no such field.
    ///
    /// `input` may be a plain string while a `session.next.tool.input.delta`
    /// stream is still being accumulated (the partial tool-call JSON, before the
    /// parsed object arrives with `tool.called`) — show that raw fragment then.
    pub fn description(&self) -> String {
        let input = self.input();
        if let Some(s) = input.as_str() {
            return s.to_string();
        }
        for key in LABEL_INPUT_KEYS {
            if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
        match self {
            ToolState::Completed { title, .. } => title.clone(),
            ToolState::Error { error, .. } => error.clone(),
            ToolState::Pending { .. } | ToolState::Running { .. } => String::new(),
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