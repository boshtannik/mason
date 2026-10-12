use crate::api::OpenCodeClient;

impl OpenCodeClient {
    /// PUT /auth/{provider_id} — store an API-key credential for a provider.
    /// The server persists it to `auth.json` and applies it to new requests.
    pub async fn set_api_key(&self, provider_id: &str, key: &str) -> Result<(), String> {
        let url = format!("{}/auth/{provider_id}", self.base);
        let resp = self
            .authed(self.http.put(&url))
            .json(&serde_json::json!({ "type": "api", "key": key }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("auth status: {}", resp.status()));
        }
        Ok(())
    }
}
