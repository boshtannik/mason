use std::sync::Arc;

use tokio::sync::Mutex;

use crate::state::permission::PermissionQueue;
use crate::state::session::SessionFsm;
use crate::state::tool::ToolStore;
use crate::types::event::Event;

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
        Event::PermissionUpdated { properties: perm } => {
            state.permissions.push(perm.clone());
            true
        }
        Event::PermissionReplied { properties } => {
            state.permissions.pop(&properties.permissionID);
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
        _ => false,
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