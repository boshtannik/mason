use serde::Deserialize;

use super::message::{Message, Part};
use super::session::{Session, SessionStatus, Todo};

/// Событие с /global/event (SSE). Документация: @opencode-ai/sdk@1.18.31, GlobalEvent.
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalEvent {
    pub payload: Event,
}

/// Официальный union событий opencode 1.18.31 (89 прямых + sync-агрегаты).
/// Полный, без Unknown-фолбэка: match'и по нему обязаны быть исчерпывающими.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "models-dev.refreshed")]
    ModelsDevRefreshed { properties: serde_json::Value },

    #[serde(rename = "integration.updated")]
    IntegrationUpdated { properties: serde_json::Value },

    #[serde(rename = "integration.connection.updated")]
    IntegrationConnectionUpdated { properties: IntegrationConnectionUpdatedProperties },

    #[serde(rename = "catalog.updated")]
    CatalogUpdated { properties: serde_json::Value },

    #[serde(rename = "session.created")]
    SessionCreated { properties: SessionCreatedProperties },

    #[serde(rename = "session.updated")]
    SessionUpdated { properties: SessionCreatedProperties },

    #[serde(rename = "session.deleted")]
    SessionDeleted { properties: SessionCreatedProperties },

    #[serde(rename = "message.updated")]
    MessageUpdated { properties: MessageUpdatedProperties },

    #[serde(rename = "message.removed")]
    MessageRemoved { properties: MessageRemovedProperties },

    #[serde(rename = "message.part.updated")]
    MessagePartUpdated { properties: MessagePartUpdatedProperties },

    #[serde(rename = "message.part.removed")]
    MessagePartRemoved { properties: MessagePartRemovedProperties },

    #[serde(rename = "session.next.agent.switched")]
    SessionNextAgentSwitched { properties: SessionNextAgentSwitchedProperties },

    #[serde(rename = "session.next.model.switched")]
    SessionNextModelSwitched { properties: SessionNextModelSwitchedProperties },

    #[serde(rename = "session.next.moved")]
    SessionNextMoved { properties: SessionNextMovedProperties },

    #[serde(rename = "session.next.prompted")]
    SessionNextPrompted { properties: SessionNextPromptedProperties },

    #[serde(rename = "session.next.prompt.admitted")]
    SessionNextPromptAdmitted { properties: SessionNextPromptedProperties },

    #[serde(rename = "session.next.context.updated")]
    SessionNextContextUpdated { properties: SessionNextMessageTextProperties },

    #[serde(rename = "session.next.synthetic")]
    SessionNextSynthetic { properties: SessionNextMessageTextProperties },

    #[serde(rename = "session.next.shell.started")]
    SessionNextShellStarted { properties: SessionNextShellStartedProperties },

    #[serde(rename = "session.next.shell.ended")]
    SessionNextShellEnded { properties: SessionNextShellEndedProperties },

    #[serde(rename = "session.next.step.started")]
    SessionNextStepStarted { properties: SessionNextStepStartedProperties },

    #[serde(rename = "session.next.step.ended")]
    SessionNextStepEnded { properties: SessionNextStepEndedProperties },

    #[serde(rename = "session.next.step.failed")]
    SessionNextStepFailed { properties: SessionNextStepFailedProperties },

    #[serde(rename = "session.next.text.started")]
    SessionNextTextStarted { properties: SessionNextPartLifecycleProperties },

    #[serde(rename = "session.next.text.delta")]
    SessionNextTextDelta { properties: SessionNextTextDeltaProperties },

    #[serde(rename = "session.next.text.ended")]
    SessionNextTextEnded { properties: SessionNextTextEndedProperties },

    #[serde(rename = "session.next.reasoning.started")]
    SessionNextReasoningStarted { properties: SessionNextPartLifecycleProperties },

    #[serde(rename = "session.next.reasoning.delta")]
    SessionNextReasoningDelta { properties: SessionNextReasoningDeltaProperties },

    #[serde(rename = "session.next.reasoning.ended")]
    SessionNextReasoningEnded { properties: SessionNextReasoningEndedProperties },

    #[serde(rename = "session.next.tool.input.started")]
    SessionNextToolInputStarted { properties: SessionNextToolInputStartedProperties },

    #[serde(rename = "session.next.tool.input.delta")]
    SessionNextToolInputDelta { properties: SessionNextToolInputDeltaProperties },

    #[serde(rename = "session.next.tool.input.ended")]
    SessionNextToolInputEnded { properties: SessionNextToolInputEndedProperties },

    #[serde(rename = "session.next.tool.called")]
    SessionNextToolCalled { properties: SessionNextToolCalledProperties },

    #[serde(rename = "session.next.tool.progress")]
    SessionNextToolProgress { properties: SessionNextToolProgressProperties },

    #[serde(rename = "session.next.tool.success")]
    SessionNextToolSuccess { properties: SessionNextToolSuccessProperties },

    #[serde(rename = "session.next.tool.failed")]
    SessionNextToolFailed { properties: SessionNextToolFailedProperties },

    #[serde(rename = "session.next.retried")]
    SessionNextRetried { properties: SessionNextRetriedProperties },

    #[serde(rename = "session.next.compaction.started")]
    SessionNextCompactionStarted { properties: SessionNextCompactionStartedProperties },

    #[serde(rename = "session.next.compaction.delta")]
    SessionNextCompactionDelta { properties: SessionNextCompactionDeltaProperties },

    #[serde(rename = "session.next.compaction.ended")]
    SessionNextCompactionEnded { properties: SessionNextCompactionEndedProperties },

    #[serde(rename = "session.next.revert.staged")]
    SessionNextRevertStaged { properties: SessionNextRevertStagedProperties },

    #[serde(rename = "session.next.revert.cleared")]
    SessionNextRevertCleared { properties: SessionNextRevertClearedProperties },

    #[serde(rename = "session.next.revert.committed")]
    SessionNextRevertCommitted { properties: SessionNextRevertCommittedProperties },

    #[serde(rename = "message.part.delta")]
    MessagePartDelta { properties: MessagePartDeltaProperties },

    #[serde(rename = "session.diff")]
    SessionDiff { properties: SessionDiffProperties },

    #[serde(rename = "session.error")]
    SessionError { properties: SessionErrorProperties },

    #[serde(rename = "installation.updated")]
    InstallationUpdated { properties: InstallationUpdatedProperties },

    #[serde(rename = "installation.update-available")]
    InstallationUpdateAvailable { properties: InstallationUpdatedProperties },

    #[serde(rename = "file.edited")]
    FileEdited { properties: FileEditedProperties },

    #[serde(rename = "reference.updated")]
    ReferenceUpdated { properties: serde_json::Value },

    #[serde(rename = "permission.v2.asked")]
    PermissionV2Asked { properties: PermissionV2AskedProperties },

    #[serde(rename = "permission.v2.replied")]
    PermissionV2Replied { properties: PermissionV2RepliedProperties },

    #[serde(rename = "plugin.added")]
    PluginAdded { properties: PluginAddedProperties },

    #[serde(rename = "project.directories.updated")]
    ProjectDirectoriesUpdated { properties: ProjectDirectoriesUpdatedProperties },

    #[serde(rename = "file.watcher.updated")]
    FileWatcherUpdated { properties: FileWatcherUpdatedProperties },

    #[serde(rename = "pty.created")]
    PtyCreated { properties: PtyCreatedProperties },

    #[serde(rename = "pty.updated")]
    PtyUpdated { properties: PtyCreatedProperties },

    #[serde(rename = "pty.exited")]
    PtyExited { properties: PtyExitedProperties },

    #[serde(rename = "pty.deleted")]
    PtyDeleted { properties: PtyDeletedProperties },

    #[serde(rename = "question.v2.asked")]
    QuestionV2Asked { properties: QuestionV2AskedProperties },

    #[serde(rename = "question.v2.replied")]
    QuestionV2Replied { properties: QuestionV2RepliedProperties },

    #[serde(rename = "question.v2.rejected")]
    QuestionV2Rejected { properties: QuestionRejectedProperties },

    #[serde(rename = "todo.updated")]
    TodoUpdated { properties: TodoUpdatedProperties },

    #[serde(rename = "lsp.updated")]
    LspUpdated { properties: serde_json::Value },

    #[serde(rename = "permission.asked")]
    PermissionAsked { properties: PermissionAskedProperties },

    #[serde(rename = "permission.replied")]
    PermissionReplied { properties: PermissionRepliedProperties },

    #[serde(rename = "tui.prompt.append")]
    TuiPromptAppend { properties: TuiPromptAppendProperties },

    #[serde(rename = "tui.command.execute")]
    TuiCommandExecute { properties: TuiCommandExecuteProperties },

    #[serde(rename = "tui.toast.show")]
    TuiToastShow { properties: TuiToastShowProperties },

    #[serde(rename = "tui.session.select")]
    TuiSessionSelect { properties: TuiSessionSelectProperties },

    #[serde(rename = "mcp.tools.changed")]
    McpToolsChanged { properties: McpToolsChangedProperties },

    #[serde(rename = "mcp.browser.open.failed")]
    McpBrowserOpenFailed { properties: McpBrowserOpenFailedProperties },

    #[serde(rename = "command.executed")]
    CommandExecuted { properties: CommandExecutedProperties },

    #[serde(rename = "project.updated")]
    ProjectUpdated { properties: ProjectUpdatedProperties },

    #[serde(rename = "session.status")]
    SessionStatus { properties: SessionStatusProperties },

    #[serde(rename = "session.idle")]
    SessionIdle { properties: SessionIdProperties },

    #[serde(rename = "question.asked")]
    QuestionAsked { properties: QuestionAskedProperties },

    #[serde(rename = "question.replied")]
    QuestionReplied { properties: QuestionRepliedProperties },

    #[serde(rename = "question.rejected")]
    QuestionRejected { properties: QuestionRejectedProperties },

    #[serde(rename = "session.compacted")]
    SessionCompacted { properties: SessionIdProperties },

    #[serde(rename = "vcs.branch.updated")]
    VcsBranchUpdated { properties: VcsBranchUpdatedProperties },

    #[serde(rename = "workspace.ready")]
    WorkspaceReady { properties: WorkspaceReadyProperties },

    #[serde(rename = "workspace.failed")]
    WorkspaceFailed { properties: WorkspaceFailedProperties },

    #[serde(rename = "workspace.status")]
    WorkspaceStatus { properties: WorkspaceStatusProperties },

    #[serde(rename = "worktree.ready")]
    WorktreeReady { properties: WorktreeReadyProperties },

    #[serde(rename = "worktree.failed")]
    WorktreeFailed { properties: WorkspaceFailedProperties },

    #[serde(rename = "server.connected")]
    ServerConnected { properties: serde_json::Value },

    #[serde(rename = "global.disposed")]
    GlobalDisposed { properties: serde_json::Value },

    #[serde(rename = "server.instance.disposed")]
    ServerInstanceDisposed { properties: ServerInstanceDisposedProperties },

    /// Агрегирующий (репликационный) канал: type "sync" с вложенным syncEvent.
    #[serde(rename = "sync")]
    Sync {
        #[serde(rename = "syncEvent")]
        sync_event: SyncEvent,
    },
}

/// Вложенное событие агрегата (sync). type имеет суффикс по версии (например `.1`).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SyncEvent {
    #[serde(rename = "session.created.1")]
    SessionCreated(SyncData<SessionCreatedProperties>),
    #[serde(rename = "session.updated.1")]
    SessionUpdated(SyncData<SessionCreatedProperties>),
    #[serde(rename = "session.deleted.1")]
    SessionDeleted(SyncData<SessionCreatedProperties>),
    #[serde(rename = "message.updated.1")]
    MessageUpdated(SyncData<MessageUpdatedProperties>),
    #[serde(rename = "message.removed.1")]
    MessageRemoved(SyncData<MessageRemovedProperties>),
    #[serde(rename = "message.part.updated.1")]
    MessagePartUpdated(SyncData<MessagePartUpdatedProperties>),
    #[serde(rename = "message.part.removed.1")]
    MessagePartRemoved(SyncData<MessagePartRemovedProperties>),
    #[serde(rename = "session.next.agent.switched.1")]
    SessionNextAgentSwitched(SyncData<SessionNextAgentSwitchedProperties>),
    #[serde(rename = "session.next.model.switched.1")]
    SessionNextModelSwitched(SyncData<SessionNextModelSwitchedProperties>),
    #[serde(rename = "session.next.moved.1")]
    SessionNextMoved(SyncData<SessionNextMovedProperties>),
    #[serde(rename = "session.next.prompted.1")]
    SessionNextPrompted(SyncData<SessionNextPromptedProperties>),
    #[serde(rename = "session.next.prompt.admitted.1")]
    SessionNextPromptAdmitted(SyncData<SessionNextPromptedProperties>),
    #[serde(rename = "session.next.context.updated.1")]
    SessionNextContextUpdated(SyncData<SessionNextMessageTextProperties>),
    #[serde(rename = "session.next.synthetic.1")]
    SessionNextSynthetic(SyncData<SessionNextMessageTextProperties>),
    #[serde(rename = "session.next.shell.started.1")]
    SessionNextShellStarted(SyncData<SessionNextShellStartedProperties>),
    #[serde(rename = "session.next.shell.ended.1")]
    SessionNextShellEnded(SyncData<SessionNextShellEndedProperties>),
    #[serde(rename = "session.next.step.started.1")]
    SessionNextStepStarted(SyncData<SessionNextStepStartedProperties>),
    #[serde(rename = "session.next.step.ended.2")]
    SessionNextStepEnded(SyncData<SessionNextStepEndedProperties>),
    #[serde(rename = "session.next.step.failed.2")]
    SessionNextStepFailed(SyncData<SessionNextStepFailedProperties>),
    #[serde(rename = "session.next.text.started.1")]
    SessionNextTextStarted(SyncData<SessionNextPartLifecycleProperties>),
    #[serde(rename = "session.next.text.ended.1")]
    SessionNextTextEnded(SyncData<SessionNextTextEndedProperties>),
    #[serde(rename = "session.next.reasoning.started.1")]
    SessionNextReasoningStarted(SyncData<SessionNextPartLifecycleProperties>),
    #[serde(rename = "session.next.reasoning.ended.1")]
    SessionNextReasoningEnded(SyncData<SessionNextReasoningEndedProperties>),
    #[serde(rename = "session.next.tool.input.started.1")]
    SessionNextToolInputStarted(SyncData<SessionNextToolInputStartedProperties>),
    #[serde(rename = "session.next.tool.input.ended.1")]
    SessionNextToolInputEnded(SyncData<SessionNextToolInputEndedProperties>),
    #[serde(rename = "session.next.tool.called.1")]
    SessionNextToolCalled(SyncData<SessionNextToolCalledProperties>),
    #[serde(rename = "session.next.tool.progress.1")]
    SessionNextToolProgress(SyncData<SessionNextToolProgressProperties>),
    #[serde(rename = "session.next.tool.success.1")]
    SessionNextToolSuccess(SyncData<SessionNextToolSuccessProperties>),
    #[serde(rename = "session.next.tool.failed.1")]
    SessionNextToolFailed(SyncData<SessionNextToolFailedProperties>),
    #[serde(rename = "session.next.retried.1")]
    SessionNextRetried(SyncData<SessionNextRetriedProperties>),
    #[serde(rename = "session.next.compaction.started.1")]
    SessionNextCompactionStarted(SyncData<SessionNextCompactionStartedProperties>),
    #[serde(rename = "session.next.compaction.ended.1")]
    SessionNextCompactionEnded(SyncData<SessionNextCompactionEndedProperties>),
    #[serde(rename = "session.next.revert.staged.1")]
    SessionNextRevertStaged(SyncData<SessionNextRevertStagedProperties>),
    #[serde(rename = "session.next.revert.cleared.1")]
    SessionNextRevertCleared(SyncData<SessionNextRevertClearedProperties>),
    #[serde(rename = "session.next.revert.committed.1")]
    SessionNextRevertCommitted(SyncData<SessionNextRevertCommittedProperties>),
}

/// Тело sync-события: метаданные агрегата + data (как properties у прямого события).
#[derive(Debug, Clone, Deserialize)]
pub struct SyncData<T> {
    pub id: String,
    pub seq: u64,
    pub aggregateID: String,
    pub data: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IntegrationConnectionUpdatedProperties {
    pub integrationID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionCreatedProperties {
    pub sessionID: String,
    pub info: Session,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageUpdatedProperties {
    pub sessionID: String,
    pub info: Message,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageRemovedProperties {
    pub sessionID: String,
    pub messageID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePartUpdatedProperties {
    pub sessionID: String,
    pub part: Part,
    pub time: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePartRemovedProperties {
    pub sessionID: String,
    pub messageID: String,
    pub partID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelRef {
    pub id: String,
    pub providerID: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextAgentSwitchedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextModelSwitchedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub model: ModelRef,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocationRef {
    pub directory: String,
    pub workspaceID: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextMovedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub location: LocationRef,
    pub subdirectory: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Prompt {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptDelivery {
    Steer,
    Queue,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextPromptedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub prompt: Prompt,
    pub delivery: PromptDelivery,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextMessageTextProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextShellStartedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub callID: String,
    pub command: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextShellEndedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub callID: String,
    pub output: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextStepStartedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub agent: String,
    pub model: ModelRef,
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StepTokenUsage {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache: StepTokenCache,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StepTokenCache {
    pub read: u64,
    pub write: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextStepEndedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub finish: String,
    pub cost: f64,
    pub tokens: StepTokenUsage,
    pub snapshot: Option<String>,
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextStepFailedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub error: serde_json::Value,
}

/// Объект для text.started / reasoning.started: показывается сразу после старта.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextPartLifecycleProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub textID: Option<String>,
    pub reasoningID: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextTextDeltaProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub textID: String,
    pub delta: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextTextEndedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub textID: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextReasoningDeltaProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub reasoningID: String,
    pub delta: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextReasoningEndedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub reasoningID: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextToolInputStartedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub callID: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextToolInputDeltaProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub callID: String,
    pub delta: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextToolInputEndedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub callID: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolProvider {
    pub executed: bool,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextToolCalledProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub callID: String,
    pub tool: String,
    pub input: serde_json::Value,
    pub provider: ToolProvider,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextToolProgressProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub callID: String,
    pub structured: serde_json::Value,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextToolSuccessProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub callID: String,
    pub structured: serde_json::Value,
    pub content: serde_json::Value,
    pub outputPaths: Option<Vec<String>>,
    pub result: Option<serde_json::Value>,
    pub provider: ToolProvider,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextToolFailedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub assistantMessageID: String,
    pub callID: String,
    pub error: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub provider: ToolProvider,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextRetriedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub attempt: u64,
    pub error: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactionReason {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextCompactionStartedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub reason: CompactionReason,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextCompactionDeltaProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextCompactionEndedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
    pub reason: CompactionReason,
    pub text: String,
    pub recent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextRevertStagedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub revert: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextRevertClearedProperties {
    pub timestamp: f64,
    pub sessionID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionNextRevertCommittedProperties {
    pub timestamp: f64,
    pub sessionID: String,
    pub messageID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessagePartDeltaProperties {
    pub sessionID: String,
    pub messageID: String,
    pub partID: String,
    pub field: String,
    pub delta: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotFileDiff {
    pub file: Option<String>,
    pub patch: Option<String>,
    pub additions: u64,
    pub deletions: u64,
    pub status: Option<SnapshotFileStatus>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotFileStatus {
    Added,
    Deleted,
    Modified,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionDiffProperties {
    pub sessionID: String,
    pub diff: Vec<SnapshotFileDiff>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionErrorProperties {
    pub sessionID: Option<String>,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallationUpdatedProperties {
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEditedProperties {
    pub file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionV2Source {
    #[serde(rename = "type")]
    pub kind: String,
    pub messageID: String,
    pub callID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionV2AskedProperties {
    pub id: String,
    pub sessionID: String,
    pub action: String,
    pub resources: Vec<String>,
    pub save: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    pub source: Option<PermissionV2Source>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionReplyKind {
    Once,
    Always,
    Reject,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionV2RepliedProperties {
    pub sessionID: String,
    pub requestID: String,
    pub reply: PermissionReplyKind,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginAddedProperties {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectDirectoriesUpdatedProperties {
    pub projectID: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileWatcherEvent {
    Add,
    Change,
    Unlink,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileWatcherUpdatedProperties {
    pub file: String,
    pub event: FileWatcherEvent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PtyStatus {
    Running,
    Exited,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pty {
    pub id: String,
    pub title: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: PtyStatus,
    pub pid: u64,
    pub exitCode: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PtyCreatedProperties {
    pub info: Pty,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PtyExitedProperties {
    pub id: String,
    pub exitCode: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PtyDeletedProperties {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionV2Info {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    pub multiple: Option<bool>,
    pub custom: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionTool {
    pub messageID: String,
    pub callID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionV2AskedProperties {
    pub id: String,
    pub sessionID: String,
    pub questions: Vec<QuestionV2Info>,
    pub tool: Option<QuestionTool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionV2RepliedProperties {
    pub sessionID: String,
    pub requestID: String,
    pub answers: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionRejectedProperties {
    pub sessionID: String,
    pub requestID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TodoUpdatedProperties {
    pub sessionID: String,
    pub todos: Vec<Todo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionAskedProperties {
    pub id: String,
    pub sessionID: String,
    pub permission: String,
    pub patterns: Vec<String>,
    pub metadata: serde_json::Value,
    pub always: Vec<String>,
    pub tool: Option<PermissionTool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionTool {
    pub messageID: String,
    pub callID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionRepliedProperties {
    pub sessionID: String,
    pub requestID: String,
    pub reply: PermissionReplyKind,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TuiPromptAppendProperties {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TuiCommandExecuteProperties {
    pub command: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TuiToastVariant {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TuiToastShowProperties {
    pub title: Option<String>,
    pub message: String,
    pub variant: TuiToastVariant,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TuiSessionSelectProperties {
    pub sessionID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpToolsChangedProperties {
    pub server: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpBrowserOpenFailedProperties {
    pub mcpName: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandExecutedProperties {
    pub name: String,
    pub sessionID: String,
    pub arguments: String,
    pub messageID: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectTime {
    pub created: f64,
    pub updated: f64,
    pub initialized: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectIcon {
    pub url: Option<String>,
    #[serde(rename = "override")]
    pub overrides: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectCommands {
    pub start: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectUpdatedProperties {
    pub id: String,
    pub worktree: String,
    pub time: ProjectTime,
    pub sandboxes: Vec<String>,
    pub name: Option<String>,
    pub icon: Option<ProjectIcon>,
    pub commands: Option<ProjectCommands>,
    pub vcs: Option<String>,
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
pub struct QuestionInfo {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    pub multiple: Option<bool>,
    pub custom: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionAskedProperties {
    pub id: String,
    pub sessionID: String,
    pub questions: Vec<QuestionInfo>,
    pub tool: Option<QuestionTool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionRepliedProperties {
    pub sessionID: String,
    pub requestID: String,
    pub answers: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VcsBranchUpdatedProperties {
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceReadyProperties {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceFailedProperties {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceStatusKind {
    Connected,
    Connecting,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceStatusProperties {
    pub workspaceID: String,
    pub status: WorkspaceStatusKind,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeReadyProperties {
    pub name: String,
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerInstanceDisposedProperties {
    pub directory: String,
}

impl Event {
    /// Ид + текст накопленной текстовой части (message.part.updated, text).
    /// Единственный исчерпывающий match; перечисляем все варианты без catch-all.
    pub fn text_part(&self) -> Option<(&str, &str, String)> {
        match self {
            Event::MessagePartUpdated { properties } => match &properties.part {
                Part::Text { id, messageID, text, ignored, .. } if *ignored != Some(true) => {
                    Some((messageID.as_str(), id.as_str(), text.clone()))
                }
                Part::Text { .. }
                | Part::Reasoning { .. }
                | Part::File { .. }
                | Part::Tool { .. }
                | Part::StepStart { .. }
                | Part::StepFinish { .. }
                | Part::Snapshot { .. }
                | Part::Patch { .. }
                | Part::Agent { .. }
                | Part::Retry { .. }
                | Part::Compaction { .. }
                | Part::Subtask { .. } => None,
            },
            Event::ModelsDevRefreshed { .. }
            | Event::IntegrationUpdated { .. }
            | Event::IntegrationConnectionUpdated { .. }
            | Event::CatalogUpdated { .. }
            | Event::SessionCreated { .. }
            | Event::SessionUpdated { .. }
            | Event::SessionDeleted { .. }
            | Event::MessageUpdated { .. }
            | Event::MessageRemoved { .. }
            | Event::MessagePartRemoved { .. }
            | Event::SessionNextAgentSwitched { .. }
            | Event::SessionNextModelSwitched { .. }
            | Event::SessionNextMoved { .. }
            | Event::SessionNextPrompted { .. }
            | Event::SessionNextPromptAdmitted { .. }
            | Event::SessionNextContextUpdated { .. }
            | Event::SessionNextSynthetic { .. }
            | Event::SessionNextShellStarted { .. }
            | Event::SessionNextShellEnded { .. }
            | Event::SessionNextStepStarted { .. }
            | Event::SessionNextStepEnded { .. }
            | Event::SessionNextStepFailed { .. }
            | Event::SessionNextTextStarted { .. }
            | Event::SessionNextTextDelta { .. }
            | Event::SessionNextTextEnded { .. }
            | Event::SessionNextReasoningStarted { .. }
            | Event::SessionNextReasoningDelta { .. }
            | Event::SessionNextReasoningEnded { .. }
            | Event::SessionNextToolInputStarted { .. }
            | Event::SessionNextToolInputDelta { .. }
            | Event::SessionNextToolInputEnded { .. }
            | Event::SessionNextToolCalled { .. }
            | Event::SessionNextToolProgress { .. }
            | Event::SessionNextToolSuccess { .. }
            | Event::SessionNextToolFailed { .. }
            | Event::SessionNextRetried { .. }
            | Event::SessionNextCompactionStarted { .. }
            | Event::SessionNextCompactionDelta { .. }
            | Event::SessionNextCompactionEnded { .. }
            | Event::SessionNextRevertStaged { .. }
            | Event::SessionNextRevertCleared { .. }
            | Event::SessionNextRevertCommitted { .. }
            | Event::MessagePartDelta { .. }
            | Event::SessionDiff { .. }
            | Event::SessionError { .. }
            | Event::InstallationUpdated { .. }
            | Event::InstallationUpdateAvailable { .. }
            | Event::FileEdited { .. }
            | Event::ReferenceUpdated { .. }
            | Event::PermissionV2Asked { .. }
            | Event::PermissionV2Replied { .. }
            | Event::PluginAdded { .. }
            | Event::ProjectDirectoriesUpdated { .. }
            | Event::FileWatcherUpdated { .. }
            | Event::PtyCreated { .. }
            | Event::PtyUpdated { .. }
            | Event::PtyExited { .. }
            | Event::PtyDeleted { .. }
            | Event::QuestionV2Asked { .. }
            | Event::QuestionV2Replied { .. }
            | Event::QuestionV2Rejected { .. }
            | Event::TodoUpdated { .. }
            | Event::LspUpdated { .. }
            | Event::PermissionAsked { .. }
            | Event::PermissionReplied { .. }
            | Event::TuiPromptAppend { .. }
            | Event::TuiCommandExecute { .. }
            | Event::TuiToastShow { .. }
            | Event::TuiSessionSelect { .. }
            | Event::McpToolsChanged { .. }
            | Event::McpBrowserOpenFailed { .. }
            | Event::CommandExecuted { .. }
            | Event::ProjectUpdated { .. }
            | Event::SessionStatus { .. }
            | Event::SessionIdle { .. }
            | Event::QuestionAsked { .. }
            | Event::QuestionReplied { .. }
            | Event::QuestionRejected { .. }
            | Event::SessionCompacted { .. }
            | Event::VcsBranchUpdated { .. }
            | Event::WorkspaceReady { .. }
            | Event::WorkspaceFailed { .. }
            | Event::WorkspaceStatus { .. }
            | Event::WorktreeReady { .. }
            | Event::WorktreeFailed { .. }
            | Event::ServerConnected { .. }
            | Event::GlobalDisposed { .. }
            | Event::ServerInstanceDisposed { .. }
            | Event::Sync { .. } => None,
        }
    }

    /// messageID, если событие — обновление ассистентского сообщения.
    pub fn assistant_message_id(&self) -> Option<&str> {
        match self {
            Event::MessageUpdated { properties } => match &properties.info {
                Message::Assistant { id, .. } => Some(id.as_str()),
                Message::User { .. } => None,
            },
            Event::ModelsDevRefreshed { .. }
            | Event::IntegrationUpdated { .. }
            | Event::IntegrationConnectionUpdated { .. }
            | Event::CatalogUpdated { .. }
            | Event::SessionCreated { .. }
            | Event::SessionUpdated { .. }
            | Event::SessionDeleted { .. }
            | Event::MessageRemoved { .. }
            | Event::MessagePartUpdated { .. }
            | Event::MessagePartRemoved { .. }
            | Event::SessionNextAgentSwitched { .. }
            | Event::SessionNextModelSwitched { .. }
            | Event::SessionNextMoved { .. }
            | Event::SessionNextPrompted { .. }
            | Event::SessionNextPromptAdmitted { .. }
            | Event::SessionNextContextUpdated { .. }
            | Event::SessionNextSynthetic { .. }
            | Event::SessionNextShellStarted { .. }
            | Event::SessionNextShellEnded { .. }
            | Event::SessionNextStepStarted { .. }
            | Event::SessionNextStepEnded { .. }
            | Event::SessionNextStepFailed { .. }
            | Event::SessionNextTextStarted { .. }
            | Event::SessionNextTextDelta { .. }
            | Event::SessionNextTextEnded { .. }
            | Event::SessionNextReasoningStarted { .. }
            | Event::SessionNextReasoningDelta { .. }
            | Event::SessionNextReasoningEnded { .. }
            | Event::SessionNextToolInputStarted { .. }
            | Event::SessionNextToolInputDelta { .. }
            | Event::SessionNextToolInputEnded { .. }
            | Event::SessionNextToolCalled { .. }
            | Event::SessionNextToolProgress { .. }
            | Event::SessionNextToolSuccess { .. }
            | Event::SessionNextToolFailed { .. }
            | Event::SessionNextRetried { .. }
            | Event::SessionNextCompactionStarted { .. }
            | Event::SessionNextCompactionDelta { .. }
            | Event::SessionNextCompactionEnded { .. }
            | Event::SessionNextRevertStaged { .. }
            | Event::SessionNextRevertCleared { .. }
            | Event::SessionNextRevertCommitted { .. }
            | Event::MessagePartDelta { .. }
            | Event::SessionDiff { .. }
            | Event::SessionError { .. }
            | Event::InstallationUpdated { .. }
            | Event::InstallationUpdateAvailable { .. }
            | Event::FileEdited { .. }
            | Event::ReferenceUpdated { .. }
            | Event::PermissionV2Asked { .. }
            | Event::PermissionV2Replied { .. }
            | Event::PluginAdded { .. }
            | Event::ProjectDirectoriesUpdated { .. }
            | Event::FileWatcherUpdated { .. }
            | Event::PtyCreated { .. }
            | Event::PtyUpdated { .. }
            | Event::PtyExited { .. }
            | Event::PtyDeleted { .. }
            | Event::QuestionV2Asked { .. }
            | Event::QuestionV2Replied { .. }
            | Event::QuestionV2Rejected { .. }
            | Event::TodoUpdated { .. }
            | Event::LspUpdated { .. }
            | Event::PermissionAsked { .. }
            | Event::PermissionReplied { .. }
            | Event::TuiPromptAppend { .. }
            | Event::TuiCommandExecute { .. }
            | Event::TuiToastShow { .. }
            | Event::TuiSessionSelect { .. }
            | Event::McpToolsChanged { .. }
            | Event::McpBrowserOpenFailed { .. }
            | Event::CommandExecuted { .. }
            | Event::ProjectUpdated { .. }
            | Event::SessionStatus { .. }
            | Event::SessionIdle { .. }
            | Event::QuestionAsked { .. }
            | Event::QuestionReplied { .. }
            | Event::QuestionRejected { .. }
            | Event::SessionCompacted { .. }
            | Event::VcsBranchUpdated { .. }
            | Event::WorkspaceReady { .. }
            | Event::WorkspaceFailed { .. }
            | Event::WorkspaceStatus { .. }
            | Event::WorktreeReady { .. }
            | Event::WorktreeFailed { .. }
            | Event::ServerConnected { .. }
            | Event::GlobalDisposed { .. }
            | Event::ServerInstanceDisposed { .. }
            | Event::Sync { .. } => None,
        }
    }

    /// sessionID, если сессия ушла в idle (session.idle).
    pub fn idle_session_id(&self) -> Option<&str> {
        match self {
            Event::SessionIdle { properties } => Some(properties.sessionID.as_str()),
            Event::ModelsDevRefreshed { .. }
            | Event::IntegrationUpdated { .. }
            | Event::IntegrationConnectionUpdated { .. }
            | Event::CatalogUpdated { .. }
            | Event::SessionCreated { .. }
            | Event::SessionUpdated { .. }
            | Event::SessionDeleted { .. }
            | Event::MessageUpdated { .. }
            | Event::MessageRemoved { .. }
            | Event::MessagePartUpdated { .. }
            | Event::MessagePartRemoved { .. }
            | Event::SessionNextAgentSwitched { .. }
            | Event::SessionNextModelSwitched { .. }
            | Event::SessionNextMoved { .. }
            | Event::SessionNextPrompted { .. }
            | Event::SessionNextPromptAdmitted { .. }
            | Event::SessionNextContextUpdated { .. }
            | Event::SessionNextSynthetic { .. }
            | Event::SessionNextShellStarted { .. }
            | Event::SessionNextShellEnded { .. }
            | Event::SessionNextStepStarted { .. }
            | Event::SessionNextStepEnded { .. }
            | Event::SessionNextStepFailed { .. }
            | Event::SessionNextTextStarted { .. }
            | Event::SessionNextTextDelta { .. }
            | Event::SessionNextTextEnded { .. }
            | Event::SessionNextReasoningStarted { .. }
            | Event::SessionNextReasoningDelta { .. }
            | Event::SessionNextReasoningEnded { .. }
            | Event::SessionNextToolInputStarted { .. }
            | Event::SessionNextToolInputDelta { .. }
            | Event::SessionNextToolInputEnded { .. }
            | Event::SessionNextToolCalled { .. }
            | Event::SessionNextToolProgress { .. }
            | Event::SessionNextToolSuccess { .. }
            | Event::SessionNextToolFailed { .. }
            | Event::SessionNextRetried { .. }
            | Event::SessionNextCompactionStarted { .. }
            | Event::SessionNextCompactionDelta { .. }
            | Event::SessionNextCompactionEnded { .. }
            | Event::SessionNextRevertStaged { .. }
            | Event::SessionNextRevertCleared { .. }
            | Event::SessionNextRevertCommitted { .. }
            | Event::MessagePartDelta { .. }
            | Event::SessionDiff { .. }
            | Event::SessionError { .. }
            | Event::InstallationUpdated { .. }
            | Event::InstallationUpdateAvailable { .. }
            | Event::FileEdited { .. }
            | Event::ReferenceUpdated { .. }
            | Event::PermissionV2Asked { .. }
            | Event::PermissionV2Replied { .. }
            | Event::PluginAdded { .. }
            | Event::ProjectDirectoriesUpdated { .. }
            | Event::FileWatcherUpdated { .. }
            | Event::PtyCreated { .. }
            | Event::PtyUpdated { .. }
            | Event::PtyExited { .. }
            | Event::PtyDeleted { .. }
            | Event::QuestionV2Asked { .. }
            | Event::QuestionV2Replied { .. }
            | Event::QuestionV2Rejected { .. }
            | Event::TodoUpdated { .. }
            | Event::LspUpdated { .. }
            | Event::PermissionAsked { .. }
            | Event::PermissionReplied { .. }
            | Event::TuiPromptAppend { .. }
            | Event::TuiCommandExecute { .. }
            | Event::TuiToastShow { .. }
            | Event::TuiSessionSelect { .. }
            | Event::McpToolsChanged { .. }
            | Event::McpBrowserOpenFailed { .. }
            | Event::CommandExecuted { .. }
            | Event::ProjectUpdated { .. }
            | Event::SessionStatus { .. }
            | Event::QuestionAsked { .. }
            | Event::QuestionReplied { .. }
            | Event::QuestionRejected { .. }
            | Event::SessionCompacted { .. }
            | Event::VcsBranchUpdated { .. }
            | Event::WorkspaceReady { .. }
            | Event::WorkspaceFailed { .. }
            | Event::WorkspaceStatus { .. }
            | Event::WorktreeReady { .. }
            | Event::WorktreeFailed { .. }
            | Event::ServerConnected { .. }
            | Event::GlobalDisposed { .. }
            | Event::ServerInstanceDisposed { .. }
            | Event::Sync { .. } => None,
        }
    }
}