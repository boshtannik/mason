use qmetaobject::*;

/// Мост Rust-core ←→ QML.
///
/// Свойства/сигналы для QML. Обновления из Rust-ядра (SSE) обязаны приходить
/// в поток Qt (main), т.к. emit сигналов из других потоков небезопасен:
/// планируется использование `QMetaObject::invoke_method` для кросс-потока.
#[derive(Default, QObject)]
pub struct AppBridge {
    base: qt_base_class!(trait QObject),
    /// Текущий статус: "idle" | "busy" | "connecting" | "error".
    session_status: qt_property!(String; NOTIFY session_status_changed),
    session_status_changed: qt_signal!(),
    last_text: qt_property!(String; NOTIFY last_text_changed),
    last_text_changed: qt_signal!(),
    message_received: qt_signal!(message: String),
    /// Путь до бинарника opencode serve (из настроек).
    server_bin: qt_property!(String; NOTIFY server_bin_changed),
    server_bin_changed: qt_signal!(),
}

/// Доступ из Rust-задачи (не из QML): обновить статус и сигналить в QML.
impl AppBridge {
    pub fn set_session_status_rs(&mut self, s: String) {
        self.session_status = s.into();
        self.session_status_changed();
    }

    /// Push нового текста (SSE-событие) → QML.
    pub fn push_message(&mut self, text: &str) {
        self.last_text = text.to_string().into();
        self.last_text_changed();
        self.message_received(text.to_string().into());
    }

    pub fn set_server_bin_rs(&mut self, b: String) {
        self.server_bin = b.into();
        self.server_bin_changed();
    }
}