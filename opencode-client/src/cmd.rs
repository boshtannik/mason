//! Единый источник строк команд протокола QML <-> воркер.
//!
//! Значения — snake_case варианты (serde), поэтому мост и воркер связаны
//! компилятором: опечатка в строке протокола не пройдёт незамеченной.

use serde::{Deserialize, Serialize};

/// Команды, которые QML отправляет воркеру через JSON `{"cmd": ..., ...}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cmd {
    New,
    Open,
    Rename,
    Delete,
    DeleteAll,
    Fork,
    Permission,
    /// Dev-only (см. `bridge/mock_permission`): в релиз-сборке варианта нет.
    #[cfg(debug_assertions)]
    MockPermission,
    Share,
    Unshare,
    Summarize,
    Abort,
    SetModel,
    VoiceLang,
    VoiceSelectStt,
    VoiceSelectTts,
    VoiceDownload,
    VoiceDownloadCancel,
    VoiceDelete,
    VoiceRecordStart,
    VoiceRecordStop,
    VoiceStt,
    VoiceTts,
    VoiceTtsMode,
    VoiceTtsCancel,
    VoiceCatalogUpdate,
}

impl Cmd {
    /// Голосовые команды (обрабатывает `voice::run_command`).
    pub fn is_voice(self) -> bool {
        matches!(
            self,
            Cmd::VoiceLang
                | Cmd::VoiceSelectStt
                | Cmd::VoiceSelectTts
                | Cmd::VoiceDownload
                | Cmd::VoiceDownloadCancel
                | Cmd::VoiceDelete
                | Cmd::VoiceRecordStart
                | Cmd::VoiceRecordStop
                | Cmd::VoiceStt
                | Cmd::VoiceTts
                | Cmd::VoiceTtsMode
                | Cmd::VoiceTtsCancel
                | Cmd::VoiceCatalogUpdate
        )
    }
}