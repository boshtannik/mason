use crate::api::OpenCodeClient;
use crate::types::event::PermissionReplyKind;

impl OpenCodeClient {
    /// POST /session/{sessionID}/permissions/{permissionID} — answer a permission request.
    pub async fn answer_permission(
        &self,
        session_id: &str,
        permission_id: &str,
        response: &PermissionReplyKind,
    ) -> Result<bool, String> {
        let url = format!(
            "{}/session/{session_id}/permissions/{permission_id}",
            self.base
        );
        #[derive(serde::Serialize)]
        struct Body {
            response: &'static str,
        }
        let body = Body {
            response: match response {
                PermissionReplyKind::Once => "once",
                PermissionReplyKind::Always => "always",
                PermissionReplyKind::Reject => "reject",
            },
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