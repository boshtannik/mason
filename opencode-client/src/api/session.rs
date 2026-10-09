use crate::api::OpenCodeClient;

#[derive(serde::Serialize)]
pub struct PromptBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messageID: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    pub parts: Vec<PromptPart>,
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
pub enum PromptPart {
    #[serde(rename = "text")]
    Text { text: String },
}

impl OpenCodeClient {
    /// POST /session -> создать новую сессию.
    pub async fn create_session(&self) -> Result<serde_json::Value, String> {
        let url = format!("{}/session", self.base);
        let resp = self
            .authed(self.http.post(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// GET /session -> список сессий (свежие обычно сверху).
    pub async fn list_sessions(&self) -> Result<serde_json::Value, String> {
        let url = format!("{}/session", self.base);
        let resp = self
            .authed(self.http.get(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// PATCH /session/{id} -> переименовать сессию.
    pub async fn rename_session(&self, session_id: &str, title: &str) -> Result<(), String> {
        let url = format!("{}/session/{session_id}", self.base);
        let resp = self
            .authed(self.http.patch(&url))
            .json(&serde_json::json!({ "title": title }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("rename status: {}", resp.status()));
        }
        Ok(())
    }

    /// DELETE /session/{id} -> удалить сессию.
    pub async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let url = format!("{}/session/{session_id}", self.base);
        let resp = self
            .authed(self.http.delete(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("delete status: {}", resp.status()));
        }
        Ok(())
    }

    /// GET /session/{id} -> одна сессия (в т.ч. `model.providerID`/`model.id`, `share.url`).
    pub async fn get_session(&self, session_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/session/{session_id}", self.base);
        let resp = self
            .authed(self.http.get(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// POST /session/{id}/fork -> новая сессия-ветка (тело `{messageID?}` опционально).
    pub async fn fork_session(&self, session_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/session/{session_id}/fork", self.base);
        let resp = self
            .authed(self.http.post(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// POST /session/{id}/share -> сессия со ссылкой в `share.url`.
    pub async fn share_session(&self, session_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/session/{session_id}/share", self.base);
        let resp = self
            .authed(self.http.post(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// DELETE /session/{id}/share -> снять доступ по ссылке.
    pub async fn unshare_session(&self, session_id: &str) -> Result<(), String> {
        let url = format!("{}/session/{session_id}/share", self.base);
        let resp = self
            .authed(self.http.delete(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("unshare status: {}", resp.status()));
        }
        Ok(())
    }

    /// POST /session/{id}/summarize -> сжать историю (нужны providerID и modelID сессии).
    pub async fn summarize_session(
        &self,
        session_id: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let url = format!("{}/session/{session_id}/summarize", self.base);
        let resp = self
            .authed(self.http.post(&url))
            .json(&serde_json::json!({
                "providerID": provider_id,
                "modelID": model_id,
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("summarize status: {}", resp.status()));
        }
        Ok(())
    }

    /// GET /config/providers -> `{providers:[{id,name,models:{...}}]}`.
    pub async fn providers(&self) -> Result<serde_json::Value, String> {
        let url = format!("{}/config/providers", self.base);
        let resp = self
            .authed(self.http.get(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// GET /session/{id}/todo -> чеклист задач агента.
    pub async fn session_todo(&self, session_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/session/{session_id}/todo", self.base);
        let resp = self
            .authed(self.http.get(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// POST /api/session/{id}/model -> переключить модель сессии.
    pub async fn set_model(
        &self,
        session_id: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let url = format!("{}/api/session/{session_id}/model", self.base);
        let resp = self
            .authed(self.http.post(&url))
            .json(&serde_json::json!({
                "model": { "id": model_id, "providerID": provider_id }
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("set_model status: {}", resp.status()));
        }
        Ok(())
    }

    /// POST /api/session/{id}/agent -> переключить агента сессии (Build/Plan).
    /// Меняет алгоритм последующих ходов: build — активная работа
    /// (файлы/команды), plan — планирование без изменений.
    pub async fn set_session_agent(&self, session_id: &str, agent: &str) -> Result<(), String> {
        let url = format!("{}/api/session/{session_id}/agent", self.base);
        let resp = self
            .authed(self.http.post(&url))
            .json(&serde_json::json!({ "agent": agent }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("set_session_agent status: {}", resp.status()));
        }
        Ok(())
    }

    /// GET /session/{id}/message -> `[{ info: Message, parts: [...] }]`.
    pub async fn session_messages(&self, session_id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/session/{session_id}/message", self.base);
        let resp = self
            .authed(self.http.get(&url))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let body = resp.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&body).map_err(|e| e.to_string())
    }

    /// POST /session/{id}/prompt_async — отправить промпт, не ждать ответа (SSE принесёт результат).
    pub async fn prompt_async(&self, session_id: &str, text: &str) -> Result<(), String> {
        let url = format!("{}/session/{session_id}/prompt_async", self.base);
        let body = PromptBody {
            messageID: None,
            model: None,
            agent: None,
            parts: vec![PromptPart::Text { text: text.to_string() }],
        };
        let resp = self
            .authed(self.http.post(&url))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("prompt_async status: {}", resp.status()));
        }
        Ok(())
    }

    /// POST /session/{id}/abort — прервать выполнение.
    pub async fn abort(&self, session_id: &str) -> Result<bool, String> {
        let url = format!("{}/session/{session_id}/abort", self.base);
        let resp = self.authed(self.http.post(&url)).send().await.map_err(|e| e.to_string())?;
        Ok(resp.status().is_success())
    }
}