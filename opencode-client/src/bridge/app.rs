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
    /// Статус, который пишет воркер (не-Qt-поток) и читает QML через polling.
    /// Qt-сигналы из чужих потоков не эмитим — поэтому отдельный Arc.
    #[allow(dead_code)]
    status_shared: Arc<Mutex<String>>,
    /// Список сессий (JSON `[{id,title,updated}]`), пишет воркер, читает QML.
    #[allow(dead_code)]
    sessions_shared: Arc<Mutex<String>>,
    /// Команды от QML воркеру: `"new"` | `"open:<sessionID>"`.
    #[allow(dead_code)]
    commands_shared: Arc<Mutex<Vec<String>>>,

    /// QML: забрать и очистить накопленные сообщения (polling).
    drain_messages: qt_method!(fn drain_messages(&self) -> QString {
        let out = self
            .pending_messages
            .lock()
            .map(|mut q| std::mem::take(&mut *q).join("\n"))
            .unwrap_or_default();
        QString::from(out)
    }),
    /// QML: принять промпт от пользователя.
    send_prompt: qt_method!(fn send_prompt(&self, text: QString) {
        let t = text.to_string().trim().to_string();
        log::info!("QML send_prompt -> {t:?}");
        if !t.is_empty() {
            if let Ok(mut q) = self.outgoing_prompts.lock() {
                q.push(t);
            }
        }
    }),
    /// QML: текущий статус (read-only, обновляется воркером).
    status_text: qt_method!(fn status_text(&self) -> QString {
        let v = self.status_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: список сессий как JSON (read-only, обновляется воркером).
    sessions_json: qt_method!(fn sessions_json(&self) -> QString {
        let v = self.sessions_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: создать новую сессию.
    new_session: qt_method!(fn new_session(&self) {
        log::info!("QML new_session");
        self.push_command(serde_json::json!({ "cmd": "new" }));
    }),
    /// QML: открыть сессию по id (загрузит историю).
    open_session: qt_method!(fn open_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML open_session -> {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": "open", "id": id }));
        }
    }),
    /// QML: переименовать сессию.
    rename_session: qt_method!(fn rename_session(&self, id: QString, title: QString) {
        let id = id.to_string();
        let title = title.to_string();
        log::info!("QML rename_session {id:?} -> {title:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": "rename", "id": id, "title": title }));
        }
    }),
    /// QML: удалить сессию.
    delete_session: qt_method!(fn delete_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML delete_session {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": "delete", "id": id }));
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

    /// Хэндл статуса для воркера (можно писать из любого потока).
    pub fn status_handle(&self) -> Arc<Mutex<String>> {
        self.status_shared.clone()
    }

    /// Потокобезопасно выставить статус (без Qt-сигналов).
    pub fn set_status_shared(&self, s: &str) {
        if let Ok(mut g) = self.status_shared.lock() {
            *g = s.to_string();
        }
    }

    /// Хэндл списка сессий для воркера.
    pub fn sessions_handle(&self) -> Arc<Mutex<String>> {
        self.sessions_shared.clone()
    }

    /// Хэндл очереди команд для воркера.
    pub fn commands_handle(&self) -> Arc<Mutex<Vec<String>>> {
        self.commands_shared.clone()
    }

    /// Положить JSON-команду в очередь для воркера.
    fn push_command(&self, v: serde_json::Value) {
        if let Ok(mut q) = self.commands_shared.lock() {
            q.push(v.to_string());
        }
    }
}
