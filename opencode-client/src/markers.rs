//! Worker <-> QML protocol markers: the single place describing the
//! service prefixes/labels used in chat feed lines.
//!
//! Mirrored on the QML side in `harbour-opencode.qml` (readonly constants
//! `kTsOpen`, `kTsClose`, `kThinkTag`, `kUserPrefix`, `kSvcOpen`, `kErrOpen`,
//! `kTagClose`, `kErrTag`, `kSttTag`, `kTtsTag`, `kTtsStopTag`, `kSttDoneTag`, `kArgSep`,
//! `kMsgIdOpen`, `kSetMsgIdTag`, `kRevertedTag`, `kToolTag`,
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
/// Live answer update. The worker streams `message.part.delta` chunks of every
/// answer text part as `[[answerlive]]<text>` (reasoning, if any, travels in
/// separate `type:"reasoning"` parts as `[[thinklive]]`). QML grows/replaces the
/// trailing answer bubble in place, so the answer is visible WHILE the model
/// types — the same way opencode's TUI renders a text part on each delta.
pub const ANSWER_LIVE_TAG: &str = "[[answerlive]]";
/// User-line prefix (goes after the timestamp).
pub const USER_PREFIX: &str = ">>> ";
/// Message-id marker, appended to the TAIL of a user-history line:
/// `[[t:<ms>]]>>> <text>[[m:<messageID>]]`. The id is the server `messageID`,
/// needed to offer revert/fork on that bubble. Only the worker attaches it
/// (from history — the live path learns the id via [`SET_MSG_ID_TAG`]).
pub const MSG_ID_OPEN: &str = "[[m:";
/// Control line carrying the id of the just-sent user message
/// (`[[msgid]]<messageID>`): the worker learns it from `message.updated`
/// (role=user) and QML attaches it to the last user bubble that lacks an id.
pub const SET_MSG_ID_TAG: &str = "[[msgid]]";
/// Control line carrying the session revert flag (`[[reverted]]<0|1>`): QML
/// shows a "Restore" banner while the tail is reverted.
pub const REVERTED_TAG: &str = "[[reverted]]";
/// Tool-call line, rendered INLINE in the transcript (like opencode's TUI):
/// `[[t:<ms>]][[tool]]<callID>\u001f<name>\u001f<status>\u001f<text>`.
/// The worker re-emits it on EVERY state change; QML keeps one row per
/// `callID` and replaces it in place. `status` ∈ `running` | `completed` |
/// `error`; `text` is the command/path/pattern (newlines flattened).
pub const TOOL_OPEN: &str = "[[tool]]";
/// Recognized STT-text label (worker sends `[[stt]] <text>`).
pub const STT_TAG: &str = "[[stt]]";
/// Finished TTS-WAV label (worker sends `[[tts]]<path>`).
pub const TTS_TAG: &str = "[[tts]]";
/// Control: stop playback and drop the whole speech queue (worker emits when the
/// user starts dictating or presses a stop button; QML silences its player and
/// does not report the stop back as a finished playback).
pub const TTS_STOP_TAG: &str = "[[ttsstop]]";
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

/// Build a user feed line, appending the message-id marker when `id` is known:
/// `[[t:<ts>]]>>> <text>[[m:<id>]]`. An empty `id` yields a plain user line.
pub fn proto_user_line(ts: u64, id: &str, text: &str) -> String {
    let base = format!("{TS_OPEN}{ts}{TS_CLOSE}{USER_PREFIX}{text}");
    if id.is_empty() {
        base
    } else {
        format!("{base}{MSG_ID_OPEN}{id}{TAG_CLOSE}")
    }
}

/// Build an inline tool-call line. Newlines and the arg separator are flattened
/// to spaces so the line survives the `\u001e`-joined / `\u001f`-split protocol.
pub fn proto_tool_line(ts: u64, call_id: &str, name: &str, status: &str, text: &str) -> String {
    let clean: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' || c == ARG_SEP { ' ' } else { c })
        .collect();
    let body = format!("{TOOL_OPEN}{call_id}{ARG_SEP}{name}{ARG_SEP}{status}{ARG_SEP}{clean}");
    proto_line(ts, &body)
}