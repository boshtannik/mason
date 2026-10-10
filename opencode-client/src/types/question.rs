use super::event::{QuestionInfo, QuestionTool};

/// A question request from the model awaiting a user answer.
/// Corresponds to the `question.asked` / `question.v2.asked` events (SDK 1.18.x).
/// `id` is the requestID used to reply (`POST /question/{id}/reply`) or reject
/// (`POST /question/{id}/reject`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct QuestionRequest {
    pub id: String,
    pub sessionID: String,
    pub questions: Vec<QuestionInfo>,
    pub tool: Option<QuestionTool>,
}
