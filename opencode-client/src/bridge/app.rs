use qmetaobject::*;
use std::sync::{Arc, Mutex};

use crate::cmd::Cmd;

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
    /// id текущей сессии (пишет воркер, читает QML). Пустая строка — нет сессии.
    #[allow(dead_code)]
    current_session_shared: Arc<Mutex<String>>,
    /// Запрос QML сменить страницу карусели (`take_nav` забирает и очищает; <0 — нет).
    #[allow(dead_code)]
    nav_shared: Arc<Mutex<i32>>,
    /// TODO-задачи текущей сессии (JSON), пишет воркер, читает QML.
    #[allow(dead_code)]
    todo_shared: Arc<Mutex<String>>,
    /// Плоский список моделей (JSON), пишет воркер, читает QML.
    #[allow(dead_code)]
    models_shared: Arc<Mutex<String>>,
    /// Список доступных системных звуков (JSON `[{name,path}]`), пишет воркер, читает QML.
    #[allow(dead_code)]
    sounds_shared: Arc<Mutex<String>>,
    /// JSON-статус голосового модуля (каталог моделей + прогресс), пишет воркер.
    #[allow(dead_code)]
    voice_status_shared: Arc<Mutex<String>>,
    /// Снимок активных тулов для ленты чата (JSON), пишет воркер.
    #[allow(dead_code)]
    tools_shared: Arc<Mutex<String>>,
    /// JSON-статус настроек (работа с директорией и т.п.), пишет воркер/бэкенд.
    #[allow(dead_code)]
    settings_shared: Arc<Mutex<String>>,

    /// QML: забрать и очистить накопленные сообщения (polling).
    /// Элементы разделяем контрольным символом Record Separator (`\x1e`), а не
    /// `\n`: тексты ответов ИИ могут содержать переносы строк, которые при
    /// `join("/`split`("\n")` рвали бы сообщение на куски в ленте.
    drain_messages: qt_method!(fn drain_messages(&self) -> QString {
        const RS: char = '\u{1e}';
        let out = self
            .pending_messages
            .lock()
            .map(|mut q| std::mem::take(&mut *q).join(&RS.to_string()))
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
        self.push_command(serde_json::json!({ "cmd": Cmd::New }));
    }),
    /// QML: открыть сессию по id (загрузит историю).
    open_session: qt_method!(fn open_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML open_session -> {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Open, "id": id }));
        }
    }),
    /// QML: переименовать сессию.
    rename_session: qt_method!(fn rename_session(&self, id: QString, title: QString) {
        let id = id.to_string();
        let title = title.to_string();
        log::info!("QML rename_session {id:?} -> {title:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Rename, "id": id, "title": title }));
        }
    }),
    /// QML: удалить сессию.
    delete_session: qt_method!(fn delete_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML delete_session {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Delete, "id": id }));
        }
    }),
    /// QML: удалить все сессии.
    delete_all_sessions: qt_method!(fn delete_all_sessions(&self) {
        log::info!("QML delete_all_sessions");
        self.push_command(serde_json::json!({ "cmd": Cmd::DeleteAll }));
    }),
    /// QML: форкнуть сессию (создать ветку).
    fork_session: qt_method!(fn fork_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML fork_session {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Fork, "id": id }));
        }
    }),
    /// QML: поделиться сессией (получить ссылку).
    share_session: qt_method!(fn share_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML share_session {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Share, "id": id }));
        }
    }),
    /// QML: снять доступ по ссылке.
    unshare_session: qt_method!(fn unshare_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML unshare_session {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Unshare, "id": id }));
        }
    }),
    /// QML: суммировать (сжать) историю сессии.
    summarize_session: qt_method!(fn summarize_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML summarize_session {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Summarize, "id": id }));
        }
    }),
    /// QML: прервать выполнение в сессии.
    abort_session: qt_method!(fn abort_session(&self, id: QString) {
        let id = id.to_string();
        log::info!("QML abort_session {id:?}");
        if !id.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::Abort, "id": id }));
        }
    }),
    /// QML: id текущей сессии (read-only, обновляется воркером).
    current_session_id: qt_method!(fn current_session_id(&self) -> QString {
        let v = self.current_session_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: TODO-задачи текущей сессии как JSON (read-only).
    todo_json: qt_method!(fn todo_json(&self) -> QString {
        let v = self.todo_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: плоский список моделей как JSON (read-only).
    models_json: qt_method!(fn models_json(&self) -> QString {
        let v = self.models_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: список доступных системных звуков как JSON (read-only).
    sounds_json: qt_method!(fn sounds_json(&self) -> QString {
        let v = self.sounds_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: статус голосового модуля как JSON (read-only).
    voice_status_json: qt_method!(fn voice_status_json(&self) -> QString {
        let v = self.voice_status_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: переключить модель сессии.
    set_model: qt_method!(fn set_model(&self, id: QString, provider: QString, model: QString) {
        let id = id.to_string();
        let provider = provider.to_string();
        let model = model.to_string();
        log::info!("QML set_model {id:?} -> {provider}/{model}");
        if !id.is_empty() && !provider.is_empty() && !model.is_empty() {
            self.push_command(serde_json::json!({
                "cmd": Cmd::SetModel, "id": id, "provider": provider, "model": model
            }));
        }
    }),
    /// QML: забрать запрошенную страницу карусели (или -1).
    take_nav: qt_method!(fn take_nav(&self) -> i32 {
        self.nav_shared
            .lock()
            .map(|mut n| {
                let v = *n;
                *n = -1;
                v
            })
            .unwrap_or(-1)
    }),
    /// QML: голосовая команда (cmd + payload: id модели / lang). Async — воркер.
    /// Строка от QML проверяется через `Cmd` — неизвестные значения отбрасываются.
    voice_command: qt_method!(fn voice_command(&self, cmd: QString, val: QString) {
        let val = val.to_string();
        let cmd: Option<Cmd> = serde_json::from_value(serde_json::json!(cmd.to_string())).ok();
        match cmd {
            Some(action) if action.is_voice() => {
                log::info!("QML voice {action:?} {val:?}");
                self.push_command(serde_json::json!({ "cmd": action, "id": val }));
            }
            Some(action) => log::warn!("QML voice: не голосовая команда {action:?}"),
            None => log::warn!("QML voice: неизвестная команда"),
        }
    }),
    /// QML: активные тулы текущего хода как JSON `[{id,name,status,text}]`.
    tools_json: qt_method!(fn tools_json(&self) -> QString {
        let v = self.tools_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: запросы разрешений как JSON `[{id,sessionID,action,resources,options}]`.
    permissions_json: qt_method!(fn permissions_json(&self) -> QString {
        let v = self.permissions_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: настройки приложения как JSON `{"workdir":"...","..."}`.
    settings_json: qt_method!(fn settings_json(&self) -> QString {
        let v = self.settings_shared.lock().map(|s| s.clone()).unwrap_or_default();
        QString::from(v)
    }),
    /// QML: запросить текущие настройки.
    get_settings: qt_method!(fn get_settings(&self) {
        log::info!("QML get_settings");
        self.push_command(serde_json::json!({ "cmd": Cmd::GetSettings }));
    }),
    /// QML: сохранить настройки (JSON-объект настроек).
    save_settings: qt_method!(fn save_settings(&self, json: QString) {
        let json = json.to_string();
        log::info!("QML save_settings -> {json:?}");
        if !json.is_empty() {
            self.push_command(serde_json::json!({ "cmd": Cmd::SaveSettings, "value": json }));
        }
    }),
    /// QML: установить рабочую директорию (строка пути).
    set_workdir: qt_method!(fn set_workdir(&self, path: QString) {
        let path = path.to_string();
        log::info!("QML set_workdir -> {path:?}");
        self.push_command(serde_json::json!({ "cmd": Cmd::SetWorkdir, "value": path }));
    }),
    /// QML: ответить на запрос разрешения (`response`: once | always | reject).
    answer_permission: qt_method!(fn answer_permission(&self, session: QString, id: QString, response: QString) {
        let session = session.to_string();
        let id = id.to_string();
        let response = response.to_string();
        log::info!("QML answer_permission {session:?} {id:?} -> {response:?}");
        if !session.is_empty() && !id.is_empty() && !response.is_empty() {
            self.push_command(serde_json::json!({
                "cmd": Cmd::Permission, "session": session, "id": id, "response": response
            }));
        }
    }),
    /// QML (dev): сгенерировать мок-ошибку сервера в ленту, как будто пришёл
    /// `session.error` с соответствующим типом. Полезно для отладки GUI без
    /// реального сервера. Строки кладём через ту же `human_session_error`,
    /// что и реальные события, — формат бабблов гарантированно совпадает.
    /// Доступен только в debug-сборке (`cargo build` без `--release`).
    #[cfg(debug_assertions)]
    mock_error: qt_method!(fn mock_error(&self, kind: QString) {
        let kind = kind.to_string();
        let payload = match kind.as_str() {
            "ProviderAuthError" => serde_json::json!({
                "name": "ProviderAuthError",
                "data": { "providerID": "anthropic", "message": "No API key found" }
            }),
            "APIError401" => serde_json::json!({
                "name": "APIError",
                "data": { "message": "Unauthorized", "statusCode": 401, "isRetryable": false }
            }),
            "APIError429" => serde_json::json!({
                "name": "APIError",
                "data": {
                    "message": "Rate limit exceeded",
                    "statusCode": 429,
                    "isRetryable": true,
                    "responseBody": "Free usage exceeded. Resets at 14:32 UTC"
                }
            }),
            "MessageOutputLengthError" => serde_json::json!({
                "name": "MessageOutputLengthError",
                "data": {}
            }),
            "MessageAbortedError" => serde_json::json!({
                "name": "MessageAbortedError",
                "data": { "message": "Response aborted by user" }
            }),
            "UnknownError" => serde_json::json!({
                "name": "UnknownError",
                "data": { "message": "Unexpected upstream failure" }
            }),
            other => {
                log::warn!("QML mock_error: неизвестный тип {other:?}");
                return;
            }
        };
        let msg = crate::types::event::human_session_error(Some(&payload));
        log::info!("QML mock_error {kind:?} -> {msg:?}");
        if let Ok(mut q) = self.pending_messages.lock() {
            q.push(format!("[ошибка сервера] {msg}"));
        }
    }),
    /// QML (dev): сгенерировать мок-запрос доступа, как будто агент запросил
    /// разрешение. Идёт через воркер (Cmd::MockPermission), чтобы пермишн
    /// попал в очередь диспетчера и не был затёрт периодическим снимком.
    /// Доступен только в debug-сборке.
    #[cfg(debug_assertions)]
    mock_permission: qt_method!(fn mock_permission(&self) {
        log::info!("QML mock_permission");
        self.push_command(serde_json::json!({ "cmd": Cmd::MockPermission }));
    }),
    /// QML (dev): идёт ли debug-сборка (true = кнопки моков доступны).
    // Метод есть всегда, чтобы результат можно было прочитать и в release
    // (там он просто false); сами мок-методы в release отсутствуют.
    dev_tools: qt_method!(fn dev_tools(&self) -> bool {
        cfg!(debug_assertions)
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

    /// Хэндл id текущей сессии для воркера.
    pub fn current_session_handle(&self) -> Arc<Mutex<String>> {
        self.current_session_shared.clone()
    }

    /// Хэндл запроса навигации для воркера.
    pub fn nav_handle(&self) -> Arc<Mutex<i32>> {
        self.nav_shared.clone()
    }

    /// Хэндл TODO-задач для воркера.
    pub fn todo_handle(&self) -> Arc<Mutex<String>> {
        self.todo_shared.clone()
    }

    /// Хэндл списка моделей для воркера.
    pub fn models_handle(&self) -> Arc<Mutex<String>> {
        self.models_shared.clone()
    }

    /// Хэндл списка системных звуков для воркера.
    pub fn sounds_handle(&self) -> Arc<Mutex<String>> {
        self.sounds_shared.clone()
    }

    /// Хэндл статуса голосового модуля для воркера.
    pub fn voice_status_handle(&self) -> Arc<Mutex<String>> {
        self.voice_status_shared.clone()
    }

    /// Хэндл снимка тулов для воркера.
    pub fn tools_handle(&self) -> Arc<Mutex<String>> {
        self.tools_shared.clone()
    }

    /// Хэндл очереди разрешений для воркера.
    pub fn permissions_handle(&self) -> Arc<Mutex<String>> {
        self.permissions_shared.clone()
    }

    /// Хэндл настроек для воркера.
    pub fn settings_handle(&self) -> Arc<Mutex<String>> {
        self.settings_shared.clone()
    }

    /// Положить JSON-команду в очередь для воркера.
    fn push_command(&self, v: serde_json::Value) {
        if let Ok(mut q) = self.commands_shared.lock() {
            q.push(v.to_string());
        }
    }
}
