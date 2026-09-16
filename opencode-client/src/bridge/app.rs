use qmetaobject::*;
use std::sync::{Arc, Mutex};

/// Мост Rust-core ←→ QML.
///
/// Потоки: QML-движок живёт в главном потоке и вызывает методы `drain_messages`
/// / `send_prompt`. tokio-воркер (SSE) параллельно пишет в `pending_messages`
/// через `Arc<Mutex<Vec>>`. Qt-сигналы из других потоков не эмитятся.
#[derive(Default, QObject)]
pub struct AppBridge {
    base: qt_base_class!(trait QObject),
    /// Текущий статус: "idle" | "busy" | "connecting" | "error".
    session_status: qt_property!(String; NOTIFY session_status_changed),
    session_status_changed: qt_signal!(),
    last_text: qt_property!(String; NOTIFY last_text_changed),
    last_text_changed: qt_signal!(),
    message_received: qt_signal!(message: String),

    /// Очередь готовых строк для QML (воркер → QML).
    #[allow(dead_code)]
    pending_messages: Arc<Mutex<Vec<String>>>,
    /// Очередь исходящих промптов (QML → воркер).
    #[allow(dead_code)]
    outgoing_prompts: Arc<Mutex<Vec<String>>>,

    /// QML: забрать и очистить накопленные сообщения (polling).
    drain_messages: qt_method!(fn drain_messages(&self) -> String {
        self.pending_messages.lock().map(|mut q| std::mem::take(&mut *q).join("\n")).unwrap_or_default()
    }),
    /// QML: принять промпт от пользователя.
    send_prompt: qt_method!(fn send_prompt(&self, text: String) {
        let t = text.trim().to_string();
        if !t.is_empty() {
            if let Ok(mut q) = self.outgoing_prompts.lock() {
                q.push(t);
            }
        }
    }),
}

/// Доступ из Rust-задач (не из QML, безопасно из любого потока).
impl AppBridge {
    /// Главный поток: обновить статус + сигнал QML.
    pub fn set_session_status_rs(&mut self, s: String) {
        self.session_status = s.into();
        self.session_status_changed();
    }

    /// Воркер (не-Qt-поток): положить строку в очередь для QML.
    pub fn push_message(&self, text: &str) {
        if let Ok(mut q) = self.pending_messages.lock() {
            q.push(text.to_string());
        }
    }

    /// Воркер: забрать исходящие промпты от QML.
    pub fn take_prompts(&self) -> Vec<String> {
        self.outgoing_prompts.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
    }

    /// Клоны очередей для передачи в tokio-воркер (Send + 'static).
    pub fn queues(&self) -> (Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<String>>>) {
        (self.pending_messages.clone(), self.outgoing_prompts.clone())
    }
}