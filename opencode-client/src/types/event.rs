use serde::Deserialize;

use super::message::{Message, Part};
use super::permission::Permission;
use super::session::{Session, SessionStatus};
use super::tool::ToolState;

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalEvent {
    pub payload: Event,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "server.connected")]
    ServerConnected {},

    #[serde(rename = "server.heartbeat")]
    ServerHeartbeat {},

    #[serde(rename = "session.created")]
    SessionCreated { #[serde(rename = "properties")] properties: SessionProperties },

    #[serde(rename = "session.updated")]
    SessionUpdated { #[serde(rename = "properties")] properties: SessionProperties },

    #[serde(rename = "session.deleted")]
    SessionDeleted { #[serde(rename = "properties")] properties: SessionProperties },

    #[serde(rename = "session.status")]
    SessionStatus { #[serde(rename = "properties")] properties: SessionStatusProperties },

    #[serde(rename = "session.idle")]
    SessionIdle { #[serde(rename = "properties")] properties: SessionIdProperties },

    #[serde(rename = "session.compacted")]
    SessionCompacted { #[serde(rename = "properties")] properties: SessionIdProperties },

    #[serde(rename = "session.diff")]
    SessionDiff { #[serde(rename = "properties")] properties: SessionDiffProperties },

    #[serde(rename = "session.error")]
    SessionError { #[serde(rename = "properties")] properties: SessionErrorProperties },

    #[serde(rename = "message.updated")]
    MessageUpdated { #[serde(rename = "properties")] properties: MessageUpdatedProperties },

    #[serde(rename = "message.removed")]
    MessageRemoved { #[serde(rename = "properties")] properties: MessageRemovedProperties },

    #[serde(rename = "message.part.updated")]
    MessagePartUpdated { #[serde(rename = "properties")] properties: MessagePartUpdatedProperties },

    #[serde(rename = "message.part.removed")]
    MessagePartRemoved { #[serde(rename = "properties")] properties: MessagePartRemovedProperties },

    #[serde(rename = "permission.updated")]
    PermissionUpdated { #[serde(rename = "properties")] properties: Permission },

    #[serde(rename = "permission.replied")]
    PermissionReplied { #[serde(rename = "properties")] properties: PermissionRepliedProperties },

    #[serde(rename = "file.edited")]
    FileEdited { #[serde(rename = "properties")] properties: FileEditedProperties },

    #[serde(rename = "file.watcher.updated")]
    FileWatcherUpdated { #[serde(rename = "properties")] properties: FileWatcherProperties },

    #[serde(rename = "todo.updated")]
    TodoUpdated { #[serde(rename = "properties")] properties: TodoUpdatedProperties },

    #[serde(rename = "command.executed")]
    CommandExecuted { #[serde(rename = "properties")] properties: CommandExecutedProperties },

    #[serde(rename = "vcs.branch.updated")]
    VcsBranchUpdated { #[serde(rename = "properties")] properties: VcsBranchProperties },

    #[serde(rename = "pty.created")]
    PtyCreated { #[serde(rename = "properties")] properties: PtyProperties },

    #[serde(rename = "pty.updated")]
    PtyUpdated { #[serde(rename = "properties")] properties: PtyProperties },

    #[serde(rename = "pty.exited")]
    PtyExited { #[serde(rename = "properties")] properties: PtyExitedProperties },

    #[serde(rename = "pty.deleted")]
    PtyDeleted { #[serde(rename = "properties")] properties: PtyIdProperties },
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionProperties {
    pub info: Session,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionStatusProperties {
    pub sessionID: String,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionIdProperties {
    pub sessionID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionDiffProperties {
    pub sessionID: String,
    // diff: Vec<FileDiff> — добавить при необходимости
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionErrorProperties {
    pub sessionID: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageUpdatedProperties {
    pub info: Message,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageRemovedProperties {
    pub sessionID: String,
    pub messageID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePartUpdatedProperties {
    pub part: Part,
    pub delta: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePartRemovedProperties {
    pub sessionID: String,
    pub messageID: String,
    pub partID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionRepliedProperties {
    pub sessionID: String,
    pub permissionID: String,
    pub response: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEditedProperties {
    pub file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileWatcherProperties {
    pub file: String,
    #[serde(rename = "event")]
    pub event_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TodoUpdatedProperties {
    pub sessionID: String,
    // todos: Vec<Todo> — добавить
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandExecutedProperties {
    pub name: String,
    pub sessionID: String,
    pub arguments: String,
    pub messageID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VcsBranchProperties {
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PtyProperties {
    pub info: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PtyExitedProperties {
    pub id: String,
    pub exitCode: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PtyIdProperties {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolStateProperties {
    pub tool: String,
    pub callID: String,
    pub state: ToolState,
}