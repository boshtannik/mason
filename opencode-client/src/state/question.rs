use crate::types::QuestionRequest;

/// Queue of question requests from the model awaiting a user answer.
#[derive(Debug, Default)]
pub struct QuestionQueue {
    pub pending: Vec<QuestionRequest>,
}

impl QuestionQueue {
    pub fn push(&mut self, q: QuestionRequest) {
        if !self.pending.iter().any(|x| x.id == q.id) {
            self.pending.push(q);
        }
    }

    pub fn pop(&mut self, id: &str) -> Option<QuestionRequest> {
        let idx = self.pending.iter().position(|x| x.id == id)?;
        Some(self.pending.remove(idx))
    }

    pub fn first(&self) -> Option<&QuestionRequest> {
        self.pending.first()
    }
}
