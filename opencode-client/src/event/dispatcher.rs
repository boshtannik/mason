use std::sync::Arc;

use tokio::sync::Mutex;

use crate::state::permission::PermissionQueue;
use crate::state::session::SessionFsm;
use crate::state::tool::ToolStore;
use crate::types::event::Event;
use crate::types::permission::Permission;

/// Применяет событие к состоянию. Возвращает bool — «состояние изменилось» (перерисовать UI).
pub async fn dispatch(event: &Event, state: &mut DispatcherState) -> bool {
    match event {
        Event::SessionStatus { properties } => {
            let id = properties.sessionID.clone();
            let mut fsm = state.sessions.entry(id.clone()).or_insert_with(SessionFsm::default);
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
            // Legacy permission.asked → та же модель Permission.
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
            if let crate::types::message::Part::Tool { callID, state: ts, .. } = &properties.part {
                state.tools.upsert(callID.clone(), ts.clone());
            }
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
        // Прочие события пока не меняют состояние — перечислены исчерпывающе.
        Event::ModelsDevRefreshed { .. }
        | Event::IntegrationUpdated { .. }
        | Event::IntegrationConnectionUpdated { .. }
        | Event::CatalogUpdated { .. }
        | Event::SessionCompacted { .. }
        | Event::SessionDiff { .. }
        | Event::SessionError { .. }
        | Event::MessageUpdated { .. }
        | Event::MessageRemoved { .. }
        | Event::MessagePartRemoved { .. }
        | Event::MessagePartDelta { .. }
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
        | Event::QuestionV2Asked { .. }
        | Event::QuestionV2Replied { .. }
        | Event::QuestionV2Rejected { .. }
        | Event::QuestionAsked { .. }
        | Event::QuestionReplied { .. }
        | Event::QuestionRejected { .. }
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
        | Event::ServerConnected { .. }
        | Event::GlobalDisposed { .. }
        | Event::ServerInstanceDisposed { .. }
        | Event::Sync { .. } => false,
    }
}

#[derive(Default)]
pub struct DispatcherState {
    pub sessions: std::collections::HashMap<String, SessionFsm>,
    pub session_meta: std::collections::HashMap<String, crate::types::session::Session>,
    pub tools: ToolStore,
    pub permissions: PermissionQueue,
}

pub type SharedState = Arc<Mutex<DispatcherState>>;