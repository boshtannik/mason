use crate::types::Permission;

/// Queue of permission requests waiting for a user response.
#[derive(Debug, Default)]
pub struct PermissionQueue {
    pub pending: Vec<Permission>,
}

impl PermissionQueue {
    pub fn push(&mut self, p: Permission) {
        if !self.pending.iter().any(|x| x.id == p.id) {
            self.pending.push(p);
        }
    }

    pub fn pop(&mut self, id: &str) -> Option<Permission> {
        let idx = self.pending.iter().position(|x| x.id == id)?;
        Some(self.pending.remove(idx))
    }

    pub fn first(&self) -> Option<&Permission> {
        self.pending.first()
    }
}