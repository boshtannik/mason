use std::collections::HashMap;

use crate::types::tool::ToolState;

/// Хранилище состояний тулов: callID -> ToolState.
#[derive(Debug, Default)]
pub struct ToolStore {
    pub states: HashMap<String, ToolState>,
}

impl ToolStore {
    pub fn upsert(&mut self, call_id: String, state: ToolState) {
        self.states.insert(call_id, state);
    }

    /// Число тулов в каждом состоянии — удобно для индикаторов в QML.
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        let mut pending = 0;
        let mut running = 0;
        let mut completed = 0;
        let mut failed = 0;
        for s in self.states.values() {
            match s {
                ToolState::Pending { .. } => pending += 1,
                ToolState::Running { .. } => running += 1,
                ToolState::Completed { .. } => completed += 1,
                ToolState::Error { .. } => failed += 1,
            }
        }
        (pending, running, completed, failed)
    }
}