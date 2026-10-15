use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::i18n::{err_line, now_ms};
use crate::markers::{proto_line, proto_tool_line, ANSWER_LIVE_TAG, SET_MSG_ID_TAG, THINK_LIVE_TAG};
use crate::state::permission::PermissionQueue;
use crate::state::question::QuestionQueue;
use crate::state::session::SessionFsm;
use crate::state::tool::ToolStore;
use crate::types::event::{
    human_session_error, Event, MessagePartDeltaProperties, MessageUpdatedProperties,
    SessionErrorProperties, SessionNextShellEndedProperties, SessionNextShellStartedProperties,
    SessionNextToolCalledProperties, SessionNextToolFailedProperties,
    SessionNextToolInputDeltaProperties, SessionNextToolInputEndedProperties,
    SessionNextToolInputStartedProperties, SessionNextToolProgressProperties,
    SessionNextToolSuccessProperties,
};
use crate::types::message::Message;
use crate::types::permission::Permission;
use crate::types::tool::ToolState;
use crate::types::QuestionRequest;

/// Minimum interval between two live pushes (reasoning or answer) — throttles a
/// long model turn so the QML queue is not flooded with tiny chunks. The final
/// full text is always sent once at `message.part.updated`/idle.
const REASONING_LIVE_MS: u64 = 150;

/// Display name for the agent's persistent shell (`session.next.shell.*`),
/// which carries a command but no tool name of its own.
const SHELL_TOOL_NAME: &str = "bash";

/// Handles the streaming accumulators need: the QML out-queue and the status
/// string, plus the per-connection accumulators for the live chat rendering.
///
/// Built by the SSE loop for every event. It is deliberately *this* module,
/// and not the caller, that decides how a message event touches these
/// accumulators — the caller only lends them. This keeps the "which server
/// event does what" responsibility in one place (`dispatch`, below).
pub struct StreamState<'a> {
    /// Lines queued for QML (`worker_pending`).
    pub pending: &'a std::sync::Mutex<Vec<String>>,
    /// Status token shown by QML (`idle`/`busy`/`error`/…).
    pub status: &'a std::sync::Mutex<String>,
    /// IDs of assistant messages — only their text parts feed the answer.
    pub assistant_messages: &'a mut HashSet<String>,
    /// Reasoning parts accumulated by part.id.
    pub reasonings: &'a mut HashMap<String, String>,
    /// part.ids confirmed (via `message.part.updated`) to be `type:"reasoning"`,
    /// so `message.part.delta` (which carries only partID + field) routes right.
    pub reasoning_part_ids: &'a HashSet<String>,
    /// Reasoning part.ids already streamed live (`[[thinklive]]` pushed once).
    pub live_reasoned_ids: &'a mut HashSet<String>,
    /// Timestamp of the last live reasoning push (throttle).
    pub last_reasoning_push: &'a mut u64,
    /// Answer text parts accumulated live by part.id (`[[answerlive]]`).
    pub text_live: &'a mut HashMap<String, String>,
    /// Timestamp of the last live answer push (throttle).
    pub last_answer_push: &'a mut u64,
    /// Session currently shown in the UI (only its user messages are tracked
    /// for revert/fork — a user message of another session is ignored).
    pub current_session: Option<&'a str>,
    /// Set when the worker has just sent a prompt and is waiting for the
    /// user-message id from `message.updated`; cleared once captured (see
    /// `on_message_updated`). Prevents attaching a replayed id to the wrong
    /// bubble.
    pub expect_user_id: &'a mut bool,
}

/// Applies an event to the state. Returns bool — "state changed" (redraw the UI).
///
/// This is the single hub for "what a server event does": every branch either
/// mutates state or delegates to a handler that does. Branches listed after the
/// explicit ones are not implemented in the UI yet — they warn (see
/// `note_unimplemented`). Coverage of the server's event union is still
/// incomplete; new handling is added *here* first.
pub async fn dispatch(event: &Event, state: &mut DispatcherState, stream: &mut StreamState<'_>) -> bool {
    match event {
        Event::SessionStatus { properties } => {
            let id = properties.sessionID.clone();
            let fsm = state.sessions.entry(id.clone()).or_insert_with(SessionFsm::default);
            fsm.apply(properties.status.clone());
            let _ = id;
            true
        }
        Event::SessionIdle { properties } => {
            if let Some(fsm) = state.sessions.get_mut(&properties.sessionID) {
                fsm.apply(crate::types::session::SessionStatus::Idle);
            }
            true
        }
        Event::PermissionV2Asked { properties } => {
            let perm = Permission {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                action: properties.action.clone(),
                resources: properties.resources.clone(),
                save: properties.save.clone(),
                metadata: properties.metadata.clone(),
                source: properties.source.clone(),
            };
            state.permissions.push(perm);
            true
        }
        Event::PermissionAsked { properties } => {
            // Legacy permission.asked → the same Permission model.
            let perm = Permission {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                action: properties.permission.clone(),
                resources: properties.patterns.clone(),
                save: Some(properties.always.clone()),
                metadata: Some(properties.metadata.clone()),
                source: properties.tool.as_ref().map(|t| crate::types::event::PermissionV2Source {
                    kind: "tool".to_string(),
                    messageID: t.messageID.clone(),
                    callID: t.callID.clone(),
                }),
            };
            state.permissions.push(perm);
            true
        }
        Event::PermissionV2Replied { properties: p } => {
            state.permissions.pop(&p.requestID);
            true
        }
        Event::PermissionReplied { properties: p } => {
            state.permissions.pop(&p.requestID);
            true
        }
        Event::MessagePartUpdated { properties } => {
            if let crate::types::message::Part::Tool { callID, tool, state: ts, .. } = &properties.part
            {
                state.tools.upsert(callID.clone(), tool.clone(), ts.clone());
                // Emit an inline tool row: one feed line per callID, re-sent on
                // every state change so QML can replace it in place (TUI-style
                // inline tool calls in the transcript).
                if let Ok(mut q) = stream.pending.lock() {
                    q.push(proto_tool_line(
                        now_ms(),
                        callID,
                        tool,
                        ts.ui_status(),
                        &ts.description(),
                    ));
                }
            }
            true
        }
        Event::QuestionAsked { properties } => {
            state.questions.push(QuestionRequest {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                questions: properties.questions.clone(),
                tool: properties.tool.clone(),
            });
            true
        }
        Event::QuestionV2Asked { properties } => {
            state.questions.push(QuestionRequest {
                id: properties.id.clone(),
                sessionID: properties.sessionID.clone(),
                questions: properties.questions.clone(),
                tool: properties.tool.clone(),
            });
            true
        }
        Event::QuestionReplied { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::QuestionV2Replied { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::QuestionRejected { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::QuestionV2Rejected { properties: p } => {
            state.questions.pop(&p.requestID);
            true
        }
        Event::SessionCreated { properties } | Event::SessionUpdated { properties } => {
            state.session_meta.insert(properties.info.id.clone(), properties.info.clone());
            true
        }
        Event::SessionDeleted { properties } => {
            state.session_meta.remove(&properties.info.id);
            state.sessions.remove(&properties.info.id);
            true
        }
        // Streaming / error events. Their handling used to live inline in the
        // SSE loop (main.rs), which smeared the "event → what happens"
        // responsibility across files. The roots grow here now; the handlers
        // (`on_message_updated`/`on_message_part_delta`/`on_session_error`)
        // mutate the accumulators carried in `StreamState`.
        Event::MessageUpdated { properties } => {
            on_message_updated(stream, properties);
            true
        }
        Event::MessagePartDelta { properties } => {
            on_message_part_delta(stream, properties);
            true
        }
        Event::SessionError { properties } => {
            on_session_error(stream, properties);
            true
        }
        // Tool / shell activity of the agent (`session.next.tool.*` and
        // `session.next.shell.*`). These feed `state.tools`, which the UI polls
        // as a compact activity strip so bash / read / edit / webfetch are
        // visible WHILE the agent works instead of the chat going silent.
        Event::SessionNextToolInputStarted { properties } => {
            on_tool_input_started(state, properties);
            true
        }
        Event::SessionNextToolInputDelta { properties } => {
            on_tool_input_delta(state, properties);
            true
        }
        Event::SessionNextToolInputEnded { properties } => {
            on_tool_input_ended(state, properties);
            true
        }
        Event::SessionNextToolCalled { properties } => {
            on_tool_called(state, properties);
            true
        }
        Event::SessionNextToolProgress { properties } => {
            on_tool_progress(state, properties);
            true
        }
        Event::SessionNextToolSuccess { properties } => {
            on_tool_success(state, properties);
            true
        }
        Event::SessionNextToolFailed { properties } => {
            on_tool_failed(state, properties);
            true
        }
        Event::SessionNextShellStarted { properties } => {
            on_shell_started(state, properties);
            true
        }
        Event::SessionNextShellEnded { properties } => {
            on_shell_ended(state, properties);
            true
        }
        // Everything else is not yet implemented in the UI. These branches are
        // listed exhaustively (no catch-all) so the compiler forces us to
        // revisit them when the event union grows. Each one is logged so the
        // logs make the degree of agreement between our UI and the native
        // server visible. See AGENTS.md ("Принципы").
        Event::ModelsDevRefreshed { .. }
        | Event::IntegrationUpdated { .. }
        | Event::IntegrationConnectionUpdated { .. }
        | Event::CatalogUpdated { .. }
        | Event::SessionCompacted { .. }
        | Event::SessionDiff { .. }
        | Event::MessageRemoved { .. }
        | Event::MessagePartRemoved { .. }
        | Event::SessionNextAgentSwitched { .. }
        | Event::SessionNextModelSwitched { .. }
        | Event::SessionNextMoved { .. }
        | Event::SessionNextPrompted { .. }
        | Event::SessionNextPromptAdmitted { .. }
        | Event::SessionNextContextUpdated { .. }
        | Event::SessionNextSynthetic { .. }
        | Event::SessionNextStepStarted { .. }
        | Event::SessionNextStepEnded { .. }
        | Event::SessionNextStepFailed { .. }
        | Event::SessionNextTextStarted { .. }
        | Event::SessionNextTextDelta { .. }
        | Event::SessionNextTextEnded { .. }
        | Event::SessionNextReasoningStarted { .. }
        | Event::SessionNextReasoningDelta { .. }
        | Event::SessionNextReasoningEnded { .. }
        | Event::SessionNextRetried { .. }
        | Event::SessionNextCompactionStarted { .. }
        | Event::SessionNextCompactionDelta { .. }
        | Event::SessionNextCompactionEnded { .. }
        | Event::SessionNextRevertStaged { .. }
        | Event::SessionNextRevertCleared { .. }
        | Event::SessionNextRevertCommitted { .. }
        | Event::InstallationUpdated { .. }
        | Event::InstallationUpdateAvailable { .. }
        | Event::FileEdited { .. }
        | Event::ReferenceUpdated { .. }
        | Event::PluginAdded { .. }
        | Event::ProjectDirectoriesUpdated { .. }
        | Event::FileWatcherUpdated { .. }
        | Event::PtyCreated { .. }
        | Event::PtyUpdated { .. }
        | Event::PtyExited { .. }
        | Event::PtyDeleted { .. }
        | Event::TodoUpdated { .. }
        | Event::LspUpdated { .. }
        | Event::TuiPromptAppend { .. }
        | Event::TuiCommandExecute { .. }
        | Event::TuiToastShow { .. }
        | Event::TuiSessionSelect { .. }
        | Event::McpToolsChanged { .. }
        | Event::McpBrowserOpenFailed { .. }
        | Event::CommandExecuted { .. }
        | Event::ProjectUpdated { .. }
        | Event::VcsBranchUpdated { .. }
        | Event::WorkspaceReady { .. }
        | Event::WorkspaceFailed { .. }
        | Event::WorkspaceStatus { .. }
        | Event::WorktreeReady { .. }
        | Event::WorktreeFailed { .. }
        | Event::ServerConnected { .. } | Event::ServerHeartbeat { .. }
        | Event::GlobalDisposed { .. }
        | Event::ServerInstanceDisposed { .. }
        | Event::Sync { .. } => {
            note_unimplemented(event);
            false
        }
    }
}

/// `message.updated` — register assistant message ids, so that only the text
/// parts of assistant messages feed the answer (user/echo parts are ignored).
/// A user message of the *shown* session, arriving right after we sent a prompt,
/// yields its `messageID` to QML (control line `[[msgid]]<id>`), so the freshly
/// created user bubble can offer revert/fork like the ones loaded from history.
fn on_message_updated(stream: &mut StreamState<'_>, props: &MessageUpdatedProperties) {
    match &props.info {
        Message::Assistant { id, .. } => {
            stream.assistant_messages.insert(id.clone());
        }
        Message::User { id, sessionID, .. } => {
            if *stream.expect_user_id && stream.current_session == Some(sessionID.as_str()) {
                *stream.expect_user_id = false;
                if let Ok(mut q) = stream.pending.lock() {
                    q.push(format!("{SET_MSG_ID_TAG}{id}"));
                }
            }
        }
    }
}

/// `message.part.delta` — append an on-the-fly text/reasoning chunk to its
/// accumulator and (throttled) stream it to QML so the bubble grows WHILE the
/// model works. `partID` is routed to the reasoning accumulator if the part was
/// confirmed `type:"reasoning"` via `message.part.updated`, otherwise to the
/// answer accumulator. Unknown-message and non-`text` deltas are ignored.
fn on_message_part_delta(stream: &mut StreamState<'_>, props: &MessagePartDeltaProperties) {
    if props.field != "text" || !stream.assistant_messages.contains(&props.messageID) {
        return;
    }
    let id = &props.partID;
    if stream.reasoning_part_ids.contains(id) {
        let entry = stream.reasonings.entry(id.clone()).or_default();
        entry.push_str(&props.delta);
        if entry.trim().is_empty() {
            return;
        }
        let now = now_ms();
        if now.saturating_sub(*stream.last_reasoning_push) < REASONING_LIVE_MS {
            return;
        }
        *stream.last_reasoning_push = now;
        stream.live_reasoned_ids.insert(id.clone());
        if let Ok(mut q) = stream.pending.lock() {
            let body = format!("{THINK_LIVE_TAG}{}", entry.trim_end());
            q.push(proto_line(now, &body));
        }
    } else {
        let entry = stream.text_live.entry(id.clone()).or_default();
        entry.push_str(&props.delta);
        if entry.trim().is_empty() {
            return;
        }
        let now = now_ms();
        if now.saturating_sub(*stream.last_answer_push) < REASONING_LIVE_MS {
            return;
        }
        *stream.last_answer_push = now;
        if let Ok(mut q) = stream.pending.lock() {
            let body = format!("{ANSWER_LIVE_TAG}{}", entry.trim_end());
            q.push(proto_line(now, &body));
        }
    }
}

/// `session.error` — classify the server error into a human-readable message
/// (`human_session_error`) and surface it as an error line in the chat feed,
/// switching the status to `error`. An event without an `error` field (e.g.
/// empty sessionID) is skipped.
fn on_session_error(stream: &mut StreamState<'_>, props: &SessionErrorProperties) {
    let msg = human_session_error(props.error.as_ref());
    if msg.is_empty() {
        log::debug!("session.error without error field, skipping");
        return;
    }
    log::error!("session.error: {msg}");
    if let Ok(mut q) = stream.pending.lock() {
        q.push(err_line(now_ms(), crate::i18n::K_SERVER_ERROR, &[msg]));
    }
    if let Ok(mut s) = stream.status.lock() {
        *s = "error".to_string();
    }
}

// ── Tool / shell activity ────────────────────────────────────────────────
//
// The agent's tool calls are tracked in `state.tools` (callID -> name+state).
// The UI polls a JSON snapshot of it every second and renders a compact strip,
// so bash / read / edit / webfetch stay visible WHILE the turn runs instead of
// the chat going silent.

/// `session.next.tool.input.started` — the model begins streaming a tool call;
/// we know the tool name, not yet its arguments.
fn on_tool_input_started(
    state: &mut DispatcherState,
    props: &SessionNextToolInputStartedProperties,
) {
    state.tools.upsert(
        props.callID.clone(),
        props.name.clone(),
        ToolState::Pending {
            input: serde_json::Value::Null,
        },
    );
}

/// `session.next.tool.input.delta` — a fragment of the tool-call arguments
/// (JSON being streamed). Accumulate it into the pending entry's description.
fn on_tool_input_delta(state: &mut DispatcherState, props: &SessionNextToolInputDeltaProperties) {
    let call = props.callID.as_str();
    if state.tools.name_of(call).is_none() {
        state.tools.ensure(call, "");
    }
    let prev = state.tools.input_of(call);
    let mut s = prev.as_str().unwrap_or("").to_string();
    s.push_str(&props.delta);
    state.tools.set_state(
        call,
        ToolState::Pending {
            input: serde_json::Value::String(s),
        },
    );
}

/// `session.next.tool.input.ended` — the arguments are complete as a raw JSON
/// string; parse them so the description can pick `command`/`filePath`/…
fn on_tool_input_ended(state: &mut DispatcherState, props: &SessionNextToolInputEndedProperties) {
    let call = props.callID.as_str();
    if state.tools.name_of(call).is_none() {
        state.tools.ensure(call, "");
    }
    let input = serde_json::from_str(&props.text)
        .unwrap_or_else(|_| serde_json::Value::String(props.text.clone()));
    state.tools.set_state(call, ToolState::Pending { input });
}

/// `session.next.tool.called` — the tool is invoked with parsed `input`; it is
/// now running.
fn on_tool_called(state: &mut DispatcherState, props: &SessionNextToolCalledProperties) {
    state.tools.upsert(
        props.callID.clone(),
        props.tool.clone(),
        ToolState::Running {
            input: props.input.clone(),
        },
    );
}

/// `session.next.tool.progress` — intermediate progress; keep it running.
fn on_tool_progress(state: &mut DispatcherState, props: &SessionNextToolProgressProperties) {
    let call = props.callID.as_str();
    if state.tools.name_of(call).is_none() {
        return;
    }
    let input = state.tools.input_of(call);
    state.tools.set_state(call, ToolState::Running { input });
}

/// `session.next.tool.success` — the tool finished; store a short title (from
/// the input) plus its output.
fn on_tool_success(state: &mut DispatcherState, props: &SessionNextToolSuccessProperties) {
    let call = props.callID.as_str();
    let input = state.tools.input_of(call);
    let title = short_input(&input)
        .or_else(|| state.tools.name_of(call).map(str::to_string))
        .unwrap_or_else(|| "tool".to_string());
    state.tools.set_state(
        call,
        ToolState::Completed {
            input,
            output: content_to_string(&props.content),
            title,
        },
    );
}

/// `session.next.tool.failed` — the tool errored.
fn on_tool_failed(state: &mut DispatcherState, props: &SessionNextToolFailedProperties) {
    let call = props.callID.as_str();
    let input = state.tools.input_of(call);
    state.tools.set_state(
        call,
        ToolState::Error {
            input,
            error: content_to_string(&props.error),
        },
    );
}

/// `session.next.shell.started` — the agent's persistent shell starts a command.
fn on_shell_started(state: &mut DispatcherState, props: &SessionNextShellStartedProperties) {
    state.tools.upsert(
        props.callID.clone(),
        SHELL_TOOL_NAME.to_string(),
        ToolState::Running {
            input: serde_json::json!({ "command": props.command }),
        },
    );
}

/// `session.next.shell.ended` — the shell command finished.
fn on_shell_ended(state: &mut DispatcherState, props: &SessionNextShellEndedProperties) {
    let call = props.callID.as_str();
    let input = state.tools.input_of(call);
    let title = short_input(&input)
        .or_else(|| state.tools.name_of(call).map(str::to_string))
        .unwrap_or_else(|| SHELL_TOOL_NAME.to_string());
    state.tools.set_state(
        call,
        ToolState::Completed {
            input,
            output: props.output.clone(),
            title,
        },
    );
}

/// A short human label pulled from a tool-call input (command / path / …).
fn short_input(input: &serde_json::Value) -> Option<String> {
    for key in [
        "command", "filePath", "path", "pattern", "query", "url", "prompt", "description",
    ] {
        if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Render a tool `content`/`error` value: plain string as-is, an array of
/// `{"text": …}` parts joined, anything else as compact JSON.
fn content_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|it| {
                it.get("text")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| it.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n"),
        other => other.to_string(),
    }
}

/// Logs (once per event type) that a dispatched event is not yet implemented in
/// the UI. Warning on the first occurrence keeps the "not implemented" surface
/// visible in the logs without flooding them on high-frequency events.
fn note_unimplemented(event: &Event) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};

    static SEEN: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let name = event.type_name();
    let first_time = seen.lock().map(|mut set| set.insert(name)).unwrap_or(true);
    if first_time {
        log::warn!("event '{name}' is not implemented in the UI yet — dispatch branch ignores it");
    } else {
        log::debug!("event '{name}' (not implemented) ignored again");
    }
}

#[derive(Default)]
pub struct DispatcherState {
    pub sessions: std::collections::HashMap<String, SessionFsm>,
    pub session_meta: std::collections::HashMap<String, crate::types::session::Session>,
    pub tools: ToolStore,
    pub permissions: PermissionQueue,
    pub questions: QuestionQueue,
}

pub type SharedState = Arc<Mutex<DispatcherState>>;