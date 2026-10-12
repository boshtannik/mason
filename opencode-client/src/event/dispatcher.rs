use std::sync::Arc;

use tokio::sync::Mutex;

use crate::state::permission::PermissionQueue;
use crate::state::question::QuestionQueue;
use crate::state::session::SessionFsm;
use crate::state::tool::ToolStore;
use crate::types::event::Event;
use crate::types::permission::Permission;
use crate::types::QuestionRequest;

/// Applies an event to the state. Returns bool — "state changed" (redraw the UI).
pub async fn dispatch(event: &Event, state: &mut DispatcherState) -> bool {
    match event {
        Event::SessionStatus { properties } => {
            let id = properties.sessionID.clone();
            let fsm = state.sessions.entry(id.clone()).or_insert_with(SessionFsm::default);
            fsm.apply(properties.status.clone());
            let _ = id;
            true
        }
        Event::SessionIdle { properties } => {
            if let Some(fsm) = state.sessions.get_mut(&properties.sessionID) {
                fsm.apply(crate::types::session::SessionStatus::Idle);
            }
            true
        }
        Event::PermissionV2Asked { properties } => {
            let perm = Permission {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                action: properties.action.clone(),
                resources: properties.resources.clone(),
                save: properties.save.clone(),
                metadata: properties.metadata.clone(),
                source: properties.source.clone(),
            };
            state.permissions.push(perm);
            true
        }
        Event::PermissionAsked { properties } => {
            // Legacy permission.asked → the same Permission model.
            let perm = Permission {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                action: properties.permission.clone(),
                resources: properties.patterns.clone(),
                save: Some(properties.always.clone()),
                metadata: Some(properties.metadata.clone()),
                source: properties.tool.as_ref().map(|t| crate::types::event::PermissionV2Source {
                    kind: "tool".to_string(),
                    messageID: t.messageID.clone(),
                    callID: t.callID.clone(),
                }),
            };
            state.permissions.push(perm);
            true
        }
        Event::PermissionV2Replied { properties: p } => {
            state.permissions.pop(&p.requestID);
            true
        }
        Event::PermissionReplied { properties: p } => {
            state.permissions.pop(&p.requestID);
            true
        }
        Event::MessagePartUpdated { properties } => {
            if let crate::types::message::Part::Tool { callID, tool, state: ts, .. } = &properties.part
            {
                state.tools.upsert(callID.clone(), tool.clone(), ts.clone());
            }
            true
        }
        Event::QuestionAsked { properties } => {
            state.questions.push(QuestionRequest {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                questions: properties.questions.clone(),
                tool: properties.tool.clone(),
            });
            true
        }
        Event::QuestionV2Asked { properties } => {
            state.questions.push(QuestionRequest {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                questions: properties.questions.clone(),
                tool: properties.tool.clone(),
            });
            true
        }
        Event::QuestionReplied { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::QuestionV2Replied { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::QuestionRejected { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::QuestionV2Rejected { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::SessionCreated { properties } | Event::SessionUpdated { properties } => {
            state.session_meta.insert(properties.info.id.clone(), properties.info.clone());
            true
        }
        Event::SessionDeleted { properties } => {
            state.session_meta.remove(&properties.info.id);
            state.sessions.remove(&properties.info.id);
            true
        }
        // Handled elsewhere in the event loop (text/reasoning streaming and
        // error classification in main.rs) — intentionally no warning here.
        Event::MessageUpdated { .. }
        | Event::MessagePartDelta { .. }
        | Event::SessionError { .. } => false,
        // Everything else is not yet implemented in the UI. These branches are
        // listed exhaustively (no catch-all) so the compiler forces us to
        // revisit them when the event union grows. Each one is logged so the
        // logs make the degree of agreement between our UI and the native
        // server visible. See AGENTS.md ("Принципы").
        Event::ModelsDevRefreshed { .. }
        | Event::IntegrationUpdated { .. }
        | Event::IntegrationConnectionUpdated { .. }
        | Event::CatalogUpdated { .. }
        | Event::SessionCompacted { .. }
        | Event::SessionDiff { .. }
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
        | Event::InstallationUpdated { .. }
        | Event::InstallationUpdateAvailable { .. }
        | Event::FileEdited { .. }
        | Event::ReferenceUpdated { .. }
        | Event::PluginAdded { .. }
        | Event::ProjectDirectoriesUpdated { .. }
        | Event::FileWatcherUpdated { .. }
        | Event::PtyCreated { .. }
        | Event::PtyUpdated { .. }
        | Event::PtyExited { .. }
        | Event::PtyDeleted { .. }
        | Event::TodoUpdated { .. }
        | Event::LspUpdated { .. }
        | Event::TuiPromptAppend { .. }
        | Event::TuiCommandExecute { .. }
        | Event::TuiToastShow { .. }
        | Event::TuiSessionSelect { .. }
        | Event::McpToolsChanged { .. }
        | Event::McpBrowserOpenFailed { .. }
        | Event::CommandExecuted { .. }
        | Event::ProjectUpdated { .. }
        | Event::VcsBranchUpdated { .. }
        | Event::WorkspaceReady { .. }
        | Event::WorkspaceFailed { .. }
        | Event::WorkspaceStatus { .. }
        | Event::WorktreeReady { .. }
        | Event::WorktreeFailed { .. }
        | Event::ServerConnected { .. } | Event::ServerHeartbeat { .. }
        | Event::GlobalDisposed { .. }
        | Event::ServerInstanceDisposed { .. }
        | Event::Sync { .. } => {
            note_unimplemented(event);
            false
        }
    }
}

/// Logs (once per event type) that a dispatched event is not yet implemented in
/// the UI. Warning on the first occurrence keeps the "not implemented" surface
/// visible in the logs without flooding them on high-frequency events.
fn note_unimplemented(event: &Event) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};

    static SEEN: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let name = event.type_name();
    let first_time = seen.lock().map(|mut set| set.insert(name)).unwrap_or(true);
    if first_time {
        log::warn!("event '{name}' is not implemented in the UI yet — dispatch branch ignores it");
    } else {
        log::debug!("event '{name}' (not implemented) ignored again");
    }
}

#[derive(Default)]
pub struct DispatcherState {
    pub sessions: std::collections::HashMap<String, SessionFsm>,
    pub session_meta: std::collections::HashMap<String, crate::types::session::Session>,
    pub tools: ToolStore,
    pub permissions: PermissionQueue,
    pub questions: QuestionQueue,
}

pub type SharedState = Arc<Mutex<DispatcherState>>;