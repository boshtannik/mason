use serde::Deserialize;

use super::event::PermissionV2Source;

/// Запрос разрешения, ожидающий ответа пользователя.
/// Поля соответствуют официальному событию permission.v2.asked (SDK 1.18.31).
#[derive(Debug, Clone, Deserialize)]
pub struct Permission {
    pub id: String,
    pub sessionID: String,
    pub action: String,
    pub resources: Vec<String>,
    pub save: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
    pub source: Option<PermissionV2Source>,
}