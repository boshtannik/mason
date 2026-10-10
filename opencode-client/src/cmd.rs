//! Single source of the QML <-> worker command protocol.
//!
//! Every variant carries its payload fields, and `serde` (with `tag = "cmd"`)
//! generates BOTH the command name and all JSON field names from this one
//! definition. The bridge (writer) and the worker (reader) are therefore tied
//! together: a renamed or typo'd key is impossible, so no scattered
//! `cmd["..."]` magic strings exist anywhere in the codebase.
//!
//! QML mirrors the command *names* as readonly property-constants in the root
//! `harbour-opencode.qml` (they must equal the snake_case serde names); the QML
//! side is hand-checked against this enum.

use serde::{Deserialize, Serialize};

use crate::settings::Settings;
use crate::types::event::PermissionReplyKind;

/// Commands that QML sends to the worker as JSON `{"cmd": <name>, ...fields}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Cmd {
    New,
    Open {
        id: String,
    },
    Rename {
        id: String,
        title: String,
    },
    Delete {
        id: String,
    },
    DeleteAll,
    Fork {
        id: String,
    },
    Permission {
        session: String,
        id: String,
        response: PermissionReplyKind,
    },
    /// Answer (or reject) a question from the model.
    Question {
        /// requestID of the `question.asked` event.
        id: String,
        /// Chosen option labels per question, in question order.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        answers: Vec<Vec<String>>,
        /// Reject the request instead of answering.
        #[serde(default, skip_serializing_if = "is_false")]
        reject: bool,
    },
    /// Set the working directory for opencode serve (cwd).
    SetWorkdir {
        workdir: String,
    },
    /// Request current settings (the worker refreshes shared).
    GetSettings,
    /// Save the whole settings object (working directory etc.)
    SaveSettings {
        value: Settings,
    },
    /// Dev-only (see `bridge/mock_permission`): the variant is absent in release builds.
    #[cfg(debug_assertions)]
    MockPermission,
    Share {
        id: String,
    },
    Unshare {
        id: String,
    },
    Summarize {
        id: String,
    },
    Abort {
        id: String,
    },
    SetModel {
        id: String,
        provider: String,
        model: String,
    },
    /// Switch the session agent/mode: Build | Plan.
    SetMode {
        id: String,
        mode: String,
    },
    VoiceLang {
        id: String,
    },
    VoiceSelectStt {
        id: String,
    },
    VoiceSelectTts {
        id: String,
    },
    VoiceDownload {
        id: String,
    },
    VoiceDownloadCancel {
        id: String,
    },
    VoiceDelete {
        id: String,
    },
    VoiceRecordStart,
    VoiceRecordStop,
    VoiceStt,
    VoiceTts {
        id: String,
    },
    VoiceTtsMode {
        id: String,
    },
    VoiceTtsCancel,
    VoiceCatalogUpdate,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Cmd {
    /// Voice commands (handled by `voice::run_command`).
    pub fn is_voice(&self) -> bool {
        matches!(
            self,
            Cmd::VoiceLang { .. }
                | Cmd::VoiceSelectStt { .. }
                | Cmd::VoiceSelectTts { .. }
                | Cmd::VoiceDownload { .. }
                | Cmd::VoiceDownloadCancel { .. }
                | Cmd::VoiceDelete { .. }
                | Cmd::VoiceRecordStart
                | Cmd::VoiceRecordStop
                | Cmd::VoiceStt
                | Cmd::VoiceTts { .. }
                | Cmd::VoiceTtsMode { .. }
                | Cmd::VoiceTtsCancel
                | Cmd::VoiceCatalogUpdate
        )
    }

    /// Serialize into the JSON envelope carried by the command queue.
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Build a command from its name + the single `id`/value payload used by the
    /// QML `voice_command(name, value)` bridge API. The JSON envelope is produced
    /// by serde (field names never appear as literals). Returns `None` for an
    /// unknown name.
    pub fn from_name_id(name: &str, id: &str) -> Option<Cmd> {
        #[derive(Serialize)]
        struct Envelope<'a> {
            cmd: &'a str,
            id: &'a str,
        }
        let v = serde_json::to_value(Envelope { cmd: name, id }).ok()?;
        serde_json::from_value(v).ok()
    }
}
