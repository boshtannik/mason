use crate::api::OpenCodeClient;

impl OpenCodeClient {
    /// POST /question/{requestID}/reply — answer a question from the model.
    /// `answers` is in the order of the questions; each entry is the list of
    /// selected labels (or a custom answer) for that question.
    pub async fn answer_question(
        &self,
        request_id: &str,
        answers: Vec<Vec<String>>,
    ) -> Result<bool, String> {
        let url = format!("{}/question/{request_id}/reply", self.base);
        let body = serde_json::json!({ "answers": answers });
        let resp = self
            .authed(self.http.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.status().is_success())
    }

    /// POST /question/{requestID}/reject — decline a question from the model.
    pub async fn reject_question(&self, request_id: &str) -> Result<bool, String> {
        let url = format!("{}/question/{request_id}/reject", self.base);
        let resp = self
            .authed(self.http.post(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(resp.status().is_success())
    }
}
