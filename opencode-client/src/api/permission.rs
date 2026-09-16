use crate::api::OpenCodeClient;

#[derive(serde::Serialize)]
pub struct PermissionReply {
    pub response: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remember: Option<bool>,
}

impl OpenCodeClient {
    /// POST /session/{sessionID}/permissions/{permissionID} — ответить на запрос разрешения.
    pub async fn answer_permission(
        &self,
        session_id: &str,
        permission_id: &str,
        response: &str,
        remember: Option<bool>,
    ) -> Result<bool, String> {
        let url = format!(
            "{}/session/{session_id}/permissions/{permission_id}",
            self.base
        );
        let body = PermissionReply {
            response: response.to_string(),
            remember,
        };
        let resp = self
            .authed(self.http.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.status().is_success())
    }
}