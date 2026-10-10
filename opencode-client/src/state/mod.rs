pub mod permission;
pub mod question;
pub mod session;
pub mod tool;

use std::collections::HashMap;

use crate::types::session::Session;
use crate::types::tool::ToolState;

/// Global application state: what QML sees.
#[derive(Debug, Default)]
pub struct AppState {
    pub sessions: HashMap<String, Session>,
    pub statuses: HashMap<String, crate::types::session::SessionStatus>,
    pub tools: HashMap<String, ToolState>,
    pub pending_permissions: Vec<crate::types::Permission>,
    pub active_session_id: Option<String>,
    pub messages: Vec<crate::types::message::Message>,
    pub parts: HashMap<String, Vec<crate::types::message::Part>>,
}

impl AppState {
    pub fn active_session(&self) -> Option<&Session> {
        let id = self.active_session_id.as_ref()?;
        self.sessions.get(id)
    }

    pub fn active_status(&self) -> &crate::types::session::SessionStatus {
        let id = self.active_session_id.as_ref();
        id.and_then(|id| self.statuses.get(id))
            .unwrap_or(&crate::types::session::SessionStatus::Idle)
    }

    /// Latest text content of the active session (for streaming to QML).
    pub fn active_text(&self) -> String {
        let id = match &self.active_session_id {
            Some(id) => id.clone(),
            None => return String::new(),
        };
        self.parts
            .get(&id)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| match p {
                        crate::types::message::Part::Text { text, .. } => Some(text.clone()),
                        crate::types::message::Part::Reasoning { .. }
                        | crate::types::message::Part::File { .. }
                        | crate::types::message::Part::Tool { .. }
                        | crate::types::message::Part::StepStart { .. }
                        | crate::types::message::Part::StepFinish { .. }
                        | crate::types::message::Part::Snapshot { .. }
                        | crate::types::message::Part::Patch { .. }
                        | crate::types::message::Part::Agent { .. }
                        | crate::types::message::Part::Retry { .. }
                        | crate::types::message::Part::Compaction { .. }
                        | crate::types::message::Part::Subtask { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default()
    }
}