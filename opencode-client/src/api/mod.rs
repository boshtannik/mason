pub mod permission;
pub mod question;
pub mod session;

use reqwest::Client;

pub struct OpenCodeClient {
    pub http: Client,
    pub base: String,
    pub auth: Option<(String, String)>,
}

impl OpenCodeClient {
    pub fn new(base: String, auth: Option<(String, String)>) -> Self {
        Self {
            http: Client::new(),
            base,
            auth,
        }
    }

    fn authed(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            Some((u, p)) => req.basic_auth(u, Some(p)),
            None => req,
        }
    }

    /// GET /global/health
    pub async fn health(&self) -> Result<bool, String> {
        let url = format!("{}/global/health", self.base);
        let resp = self.authed(self.http.get(&url)).send().await.map_err(|e| e.to_string())?;
        Ok(resp.status().is_success())
    }
}