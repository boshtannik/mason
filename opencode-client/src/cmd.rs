//! Single source of QML <-> worker protocol command strings.
//!
//! Values are snake_case variants (serde), so the bridge and the worker are tied
//! together by the compiler: a typo in a protocol string won't go unnoticed.

use serde::{Deserialize, Serialize};

/// Commands that QML sends to the worker via JSON `{"cmd": ..., ...}`.
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
    /// Set the working directory for opencode serve (cwd).
    #[serde(rename = "set_workdir")]
    SetWorkdir,
    /// Request current settings (the worker refreshes shared).
    #[serde(rename = "get_settings")]
    GetSettings,
    /// Save settings (working directory etc.)
    #[serde(rename = "save_settings")]
    SaveSettings,
    /// Dev-only (see `bridge/mock_permission`): the variant is absent in release builds.
    #[cfg(debug_assertions)]
    MockPermission,
    Share,
    Unshare,
    Summarize,
    Abort,
    SetModel,
    /// Switch the session agent/mode: Build | Plan.
    SetMode,
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
    /// Voice commands (handled by `voice::run_command`).
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