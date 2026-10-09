//! Маркеры протокола «воркер ↔ QML» — единственное место, где описаны
//! служебные префиксы/метки строк ленты.
//!
//! Зеркальная копия на стороне QML: `harbour-opencode.qml`
//! (readonly-константы `kTsOpen`, `kTsClose`, `kThinkTag`, `kUserPrefix`,
//! `kErrPrefix`, `kSttTag`, `kTtsTag`, `kVoicePrefix` и функция `stripTs`).
//! Держать оба файла в синхроне: менять маркер только здесь и в QML.

/// Открывающий таймстамп строки: `[[t:<мс>]]`.
pub const TS_OPEN: &str = "[[t:";
/// Закрывающий таймстамп строки: `]]`.
pub const TS_CLOSE: &str = "]]";
/// Метка «мышления» модели. Ставится ПОСЛЕ таймстампа, ПЕРЕД текстом ответа.
pub const THINK_TAG: &str = "[[think]]";
/// Префикс строк пользователя (идёт после таймстампа).
pub const USER_PREFIX: &str = ">>> ";
/// Префикс строки-ошибки соединения/ответа сервера.
pub const ERR_PREFIX: &str = "[ошибка сервера]";
/// Метка распознанного STT-текста (воркер шлёт `[[stt]] <текст>`).
pub const STT_TAG: &str = "[[stt]]";
/// Метка готового TTS-WAV (воркер шлёт `[[tts]]<путь>`).
pub const TTS_TAG: &str = "[[tts]]";
/// Префикс служебных голосовых уведомлений воркера.
pub const VOICE_PREFIX: &str = "[голос]";

/// Собрать строку ленты с таймстампом: `[[t:<ts>]]<body>`.
pub fn proto_line(ts: u64, body: &str) -> String {
    format!("{TS_OPEN}{ts}{TS_CLOSE}{body}")
}