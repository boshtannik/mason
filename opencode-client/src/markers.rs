//! Worker <-> QML protocol markers: the single place describing the
//! service prefixes/labels used in chat feed lines.
//!
//! Mirrored on the QML side in `harbour-opencode.qml` (readonly constants
//! `kTsOpen`, `kTsClose`, `kThinkTag`, `kUserPrefix`, `kSvcOpen`, `kErrOpen`,
//! `kTagClose`, `kErrTag`, `kSttTag`, `kTtsTag`, `kSttDoneTag`, `kArgSep`,
//! plus helpers `stripTs` and `lineIs`). Keep both files in sync: only change
//! a marker here and in the QML mirror.

/// Line timestamp opener: `[[t:<ms>]]`.
pub const TS_OPEN: &str = "[[t:";
/// Line timestamp closer: `]]`.
pub const TS_CLOSE: &str = "]]";
/// Model-reasoning label. Placed AFTER the timestamp, BEFORE the answer text.
pub const THINK_TAG: &str = "[[think]]";
/// Live model-reasoning update. The worker streams `message.part.delta` chunks
/// as `[[thinklive]]<text>`; QML replaces the trailing `[[think]]` bubble in
/// place (or starts a new one), so thinking is visible WHILE the model works.
/// At `message.part.updated`/idle the worker sends the final reasoning once
/// more via `[[think]]`/`[[thinklive]]` — never duplicated.
pub const THINK_LIVE_TAG: &str = "[[thinklive]]";
/// User-line prefix (goes after the timestamp).
pub const USER_PREFIX: &str = ">>> ";
/// Recognized STT-text label (worker sends `[[stt]] <text>`).
pub const STT_TAG: &str = "[[stt]]";
/// Finished TTS-WAV label (worker sends `[[tts]]<path>`).
pub const TTS_TAG: &str = "[[tts]]";
/// Control: STT cycle finished (worker emits; QML stops the mic loader).
pub const STT_DONE_TAG: &str = "[[sttdone]]";

// i18n tokens (see `i18n.rs`): the worker cannot call QML `qsTr`, so instead
// of building display text it emits a translation KEY plus placeholder args:
//   `[[t:<ms>]][[s:KEY]]`        service notice
//   `[[t:<ms>]][[s:KEY\u001fARG1]]`  notice with args (unit-separator)
//   `[[t:<ms>]][[e:KEY\u001fARG1]]`  error (red bubble + in-app notification)
// QML resolves the active translation via `qsTr(key)` in `app.trSvc()`.
/// Service-notice token opener.
pub const SVC_OPEN: &str = "[[s:";
/// Error token opener.
pub const ERR_OPEN: &str = "[[e:";
/// Generic token closer (`]]`, same as the timestamp closer).
pub const TAG_CLOSE: &str = "]]";
/// Argument separator inside i18n tokens (unit separator; safe in chat lines).
pub const ARG_SEP: char = '\u{1f}';

/// Bubble marker for error lines. Built by QML only (red bubble styling);
/// the worker never sends it — it sends `[["e":…]]` tokens instead.
pub const ERR_TAG: &str = "[[e]]";

/// Build a feed line with a timestamp: `[[t:<ts>]]<body>`.
pub fn proto_line(ts: u64, body: &str) -> String {
    format!("{TS_OPEN}{ts}{TS_CLOSE}{body}")
}