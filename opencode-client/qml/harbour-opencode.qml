import QtQuick 2.6
import Sailfish.Silica 1.0
import QtMultimedia 5.4
import Nemo.Notifications 1.0
import Nemo.KeepAlive 1.2
import Nemo.Configuration 1.0
import "pages"

ApplicationWindow {
    id: app

    property var messages: []
    property string statusText: "connecting"
    property string inputMode: inputModeSetting.value !== undefined
                               ? inputModeSetting.value : "text_ptt"
    property var sessions: []
    property var todos: []
    property var models: []
    property string currentSessionId: ""
    property bool showTools: showToolsSetting.value !== undefined
                            ? showToolsSetting.value : true
    // True only in a debug build: enables the mock buttons (errors/permissions).
    readonly property bool devTools: bridge !== undefined
                                     && bridge.dev_tools()
    property string pttPosition: pttPositionSetting.value !== undefined
                                 ? pttPositionSetting.value : "left"
    property string ttsMode: ttsModeSetting.value !== undefined
                             ? ttsModeSetting.value : "button"
    property bool soundOnFinish: soundOnFinishSetting.value !== undefined
                                 ? soundOnFinishSetting.value : true
    property bool soundOnPermission: soundOnPermissionSetting.value !== undefined
                                     ? soundOnPermissionSetting.value : true
    property bool sendImmediately: sendImmediatelySetting.value !== undefined
                                   ? sendImmediatelySetting.value : false
    // ── Worker ↔ QML protocol markers ─────────────────────────────────────────
    // Mirror of opencode-client/src/markers.rs. Change both files in sync:
    // conditions compare via these constants, never via raw strings — this
    // removes a whole class of bugs (there used to be substring(0,8) against
    // the 9-char [[think]]).
    readonly property string kTsOpen: "[[t:"
    readonly property string kTsClose: "]]"
    readonly property string kThinkTag: "[[think]]"
    // Live reasoning chunk (`[[thinklive]]<text>`): the worker streams model
    // reasoning deltas; QML replaces the trailing [[think]] bubble in place.
    readonly property string kThinkLiveTag: "[[thinklive]]"
    readonly property string kUserPrefix: ">>> "
    readonly property string kSttTag: "[[stt]]"
    readonly property string kTtsTag: "[[tts]]"
    readonly property string kSttDoneTag: "[[sttdone]]"
    // i18n tokens (worker sends a translation KEY; QML resolves qsTr): see
    // opencode-client/src/i18n.rs. `[[s:KEY]]` — service notice,
    // `[[e:KEY]]` — error (red bubble + notification), args after \u001f.
    readonly property string kSvcOpen: "[[s:"
    readonly property string kErrOpen: "[[e:"
    readonly property string kTagClose: "]]"
    readonly property string kArgSep: "\u001f"
    // Bubble markers (built by QML for system styling; the worker itself
    // never sends them). kErrTag — error (red), kSvcTag — service notice.
    readonly property string kErrTag: "[[e]]"
    readonly property string kSvcTag: "[[s]]"
    // Escape regex special characters (built from the marker constants).
    function reEscape(s) {
        return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
    }
    // Timestamp regexp `[[t:<ms>]]`, built from the constants (markers.rs:
    // proto_line). Group 1 — milliseconds. Conditions/slices go through the
    // constants, never raw strings.
    readonly property var tsRe: new RegExp(
        "^" + reEscape(app.kTsOpen) + "(\\d+)" + reEscape(app.kTsClose))
    // Strip `[[t:<ms>]]` from the start of a feed line.
    function stripTs(s) {
        return s.replace(app.tsRe, "")
    }
    // Feed line starts exactly with the marker-prefix (timestamp excluded).
    function lineIs(line, tag) {
        return line.indexOf(tag) === 0
    }
    // Millisecond ts from `[[t:<ms>]]`, or current time if missing.
    function msOf(line) {
        var m = app.tsRe.exec(line)
        return m ? m[1] : String(Date.now())
    }
    // Translate a worker i18n token (`[[s:KEY\u001fARG…]]` / `[[e:KEY…]]`)
    // into a display string using the app's active language (qsTr/.ts).
    function trSvc(tag, token) {
        var end = token.indexOf(app.kTagClose, tag.length)
        var inner = token.substring(tag.length, end > 0 ? end : token.length)
        var parts = inner.split(app.kArgSep)
        var text = qsTr(parts[0])
        for (var i = 1; i < parts.length && i <= 9; i++)
            text = text.split("%" + i).join(parts[i])
        return text
    }
    // Whether to show the model's "thinking" ([[think]] lines) in the feed,
    // not just final answers. Enabled by default. This is a client DISPLAY
    // setting (unlike the session mode), so it lives in the global settings,
    // not in the settings of a particular session.
    property bool showReasoning: showReasoningSetting.value !== undefined
                                 ? showReasoningSetting.value : true
    // Per-session chat mode: `{ "<sessionID>": "build"|"plan" }`. The mode is a
    // property of each session (agent: build — active work, plan —
    // planning); the choice is stored per session and applied when it is
    // opened (see sessionMode/setSessionMode and its use in poll()).
    property var sessionModes: sessionModesSetting.value
                               ? JSON.parse(sessionModesSetting.value)
                               : ({})
    // Sessions whose saved mode has already been applied (we don't poke the
    // server again on every poll with the same agent switch).
    property var sessionModesApplied: ({})
    // Pending agent permission requests (`[{id,sessionID,action,resources,options}]`).
    property var pendingPermissions: []
    // Granting "Allow once": true if the request is really new
    // (so the sound fires only for newly appeared ones).
    property var knownPermissionIds: []
    property bool sttModelReady: false
    // opencode working directory (setting). Loaded from bridge.settings_json().
    property string workdir: ""
    property bool ttsModelReady: chosenTts !== ""

    // For detecting the "agent busy → free" transition.
    property bool wasBusy: false

    // System sounds for notifications (scanned by the worker, `[{name,path}]`).
    property var sounds: []
    // Selected notification sound (saved in DConf settings).
    property string dingSound: dingSetting.value

    // Voice module: status from the worker (`voice_status_json`).
    property var voiceModels: []
    property variant voiceLangOptions: [ { id: "auto", name: "Auto (detect)" } ]
    property string chosenStt: ""
    property string chosenTts: ""
    property string voiceLang: "auto"
    // Whether the microphone is recording (PTT toggle).
    property bool recording: false
    // Whether speech recognition is running after the recording stopped.
    property bool recognizing: false
    // Whether the answer is being spoken (to show the "stop" button).
    property bool ttsPlaying: false
    // Text of the bubble currently being synthesised/spoken (for the loader
    // on the corresponding TTS button). Empty string — we speak nothing.
    property string speakingBubble: ""
    // Whether speech synthesis is in progress: the command was sent to the
    // worker, the WAV is not in the queue yet (the quietest phase, where feedback matters).
    property bool ttsSynth: false

    // Protocol constants of the voice module.
    // The commands mirror `voice::cmd` in src/voice.rs, the engines — `voice::engine`,
    // the phases — `voice::Phase` (as_str). They may only be changed on both sides.
    // Note: QML forbids property names starting with an uppercase letter — lowercase only.
    readonly property string cmdLang: "voice_lang"
    readonly property string cmdSelectStt: "voice_select_stt"
    readonly property string cmdSelectTts: "voice_select_tts"
    readonly property string cmdDownload: "voice_download"
    readonly property string cmdDownloadCancel: "voice_download_cancel"
    readonly property string cmdDelete: "voice_delete"
    readonly property string cmdRecordStart: "voice_record_start"
    readonly property string cmdRecordStop: "voice_record_stop"
    readonly property string cmdStt: "voice_stt"
    readonly property string cmdTts: "voice_tts"
    readonly property string cmdTtsMode: "voice_tts_mode"
    readonly property string cmdTtsCancel: "voice_tts_cancel"
    readonly property string cmdCatalogUpdate: "voice_catalog_update"
    readonly property string engineStt: "stt_whisper"
    readonly property string engineTts: "tts_piper"
    readonly property string stateDownloading: "downloading"
    readonly property string stateError: "error"

    // Request to switch the carousel page (0..3).
    signal requestPage(int index)

    // Recognised speech text that should be shown in the chat input field.
    signal dictated(string text)

    ConfigurationValue {
        id: dingSetting
        key: "/apps/harbour-opencode/dingSound"
    }

    ConfigurationValue {
        id: inputModeSetting
        key: "/apps/harbour-opencode/inputMode"
    }
    ConfigurationValue {
        id: showToolsSetting
        key: "/apps/harbour-opencode/showTools"
    }
    ConfigurationValue {
        id: pttPositionSetting
        key: "/apps/harbour-opencode/pttPosition"
    }
    ConfigurationValue {
        id: ttsModeSetting
        key: "/apps/harbour-opencode/ttsMode"
    }
    ConfigurationValue {
        id: soundOnFinishSetting
        key: "/apps/harbour-opencode/soundOnFinish"
    }
    ConfigurationValue {
        id: soundOnPermissionSetting
        key: "/apps/harbour-opencode/soundOnPermission"
    }
    ConfigurationValue {
        id: sendImmediatelySetting
        key: "/apps/harbour-opencode/sendImmediately"
    }
    ConfigurationValue {
        id: showReasoningSetting
        key: "/apps/harbour-opencode/showReasoning"
    }
    // Per-session mode registry: JSON string `{"<sessionID>":"build"|"plan",...}`.
    ConfigurationValue {
        id: sessionModesSetting
        key: "/apps/harbour-opencode/sessionModes"
    }

    // Keep the process alive while the agent works (otherwise Sailfish puts it to sleep in the background).
    KeepAlive {
        id: agentKeepAlive
        enabled: app.statusBase(app.statusText) === "busy"
    }

    // Pool of "dings": SoundEffect cannot be restarted while playing, so
// on each call we take the next free slot (rotation).
    property int dingSlot: 0
    SoundEffect { id: ding1; source: app.dingSound }
    SoundEffect { id: ding2; source: app.dingSound }
    SoundEffect { id: ding3; source: app.dingSound }
    SoundEffect { id: ding4; source: app.dingSound }

    Notification {
        id: notify
        appName: "opencode"
        urgency: Notification.Normal
        expireTimeout: 6000
    }

    // Transient "copied to clipboard" feedback. IMPORTANT: this Silica module
    // has NO `Toast`/`InfoBanner` types (both fail with "is not a type" and
    // cause a white screen) — so the banner is a plain Rectangle+Label built
    // from guaranteed-available primitives only.
    Rectangle {
        id: copyToast
        visible: false
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.paddingLarge
        radius: Theme.paddingMedium
        color: Qt.rgba(0.05, 0.05, 0.10, 0.92)
        width: copyToastTxt.implicitWidth + 2 * Theme.paddingMedium
        height: copyToastTxt.implicitHeight + Theme.paddingSmall
        Label {
            id: copyToastTxt
            anchors.centerIn: parent
            text: copyToast.bannerText
            color: Theme.highlightColor
            font.pixelSize: Theme.fontSizeSmall
        }
        property string bannerText: ""
        Timer {
            id: copyToastTimer
            interval: 1300
            repeat: false
            onTriggered: copyToast.visible = false
        }
    }

    // Copy `text` to the system clipboard and flash a confirmation banner.
    function copyText(text) {
        if (text === undefined || text === "")
            return
        Clipboard.text = text
        copyToast.bannerText = qsTr("Copied")
        copyToast.visible = true
        copyToastTimer.restart()
    }

    Timer {
        id: pollTimer
        interval: 300
        running: true
        repeat: true
        onTriggered: app.poll()
    }

    // TTS queue: WAV files are synthesised by the worker asynchronously
    // (several messages in a row = several files). We play strictly in
    // order of arrival: mediaplayer finished → take the next from the queue.
    // Each item is `{path, body}`: body is the text of the bubble that
    // owns the file, so the "stop/speak" button follows the currently
    // playing message (see speakingBubble).
    property var ttsQueue: []

    // Body of the last agent answer — binds auto-synthesised files
    // to a bubble: the worker sends `[[tts]]<path>` after the corresponding text
    // lines, so we remember the last answer. See ttsEnqueue/playNextTts.
    property string pendingTtsBody: ""

    MediaPlayer {
        id: ttsPlayer
        onPlaybackStateChanged: {
            if (ttsPlayer.playbackState === MediaPlayer.StoppedState)
                app.playNextTts()
        }
    }
    function playNextTts() {
        if (app.ttsQueue.length === 0) {
            app.ttsPlaying = false
            app.ttsSynth = false
            app.speakingBubble = ""
            return
        }
        var item = app.ttsQueue.shift()
        ttsPlayer.source = "file://" + item.path
        // The TTS indicator moves with the queue: the "stop" button turns on
        // for the bubble whose sound is actually playing right now.
        app.speakingBubble = item.body
        app.ttsPlaying = true
        app.ttsSynth = false
        ttsPlayer.play()
    }
    function ttsEnqueue(path) {
        if (path === "" || path === undefined)
            return
        // Which bubble the file belongs to: for the manual button this is the text
        // we sent to synthesis (speakingBubble); for auto-TTS — the last
        // agent answer (pendingTtsBody).
        var body = app.speakingBubble
        if ((body === "" || body === undefined) && app.pendingTtsBody !== "")
            body = app.pendingTtsBody
        app.ttsQueue.push({ path: path, body: body })
        if (!app.ttsPlaying)
            app.playNextTts()
    }
    function stopTts() {
        app.ttsQueue = []
        app.ttsPlaying = false
        app.ttsSynth = false
        app.speakingBubble = ""
        // Ask the worker not to hand over the WAV if piper is still synthesising,
        // otherwise the file "ripens" and playback switches on again.
        app.voiceCmd(app.cmdTtsCancel, "")
        ttsPlayer.stop()
    }
    function speakText(text) {
        app.speakingBubble = text
        app.ttsSynth = true
        app.voiceCmd(app.cmdTts, text)
    }

    function poll() {
        var st = bridge.status_text()
        if (st !== undefined && st !== "")
            app.statusText = st
        if (app.statusBase(app.statusText) === "busy") {
            app.wasBusy = true
        } else if (app.wasBusy) {
            app.wasBusy = false
            app.notifyAgentFinished()
        }
        var msgs = bridge.drain_messages()
        if (msgs !== "" && msgs !== undefined) {
            var lines = msgs.split("\u001e")
            var acc = app.messages
            for (var i = 0; i < lines.length; i++) {
                var line = lines[i]
                if (line !== "") {
                    // Line without the timestamp — used for [[think]] and for
                    // binding auto-TTS to the bubble.
                    var bodyTmp = app.stripTs(line)

                    // Server error (e.g. "free usage exceeded") — notify right
                    // away and push a red bubble, so it does not look like the
                    // agent is silently working. Worker sends `[[e:KEY]]`.
                    if (app.lineIs(bodyTmp, app.kErrOpen)) {
                        app.recognizing = false
                        var errText = app.trSvc(app.kErrOpen, bodyTmp)
                        app.publishError(errText)
                        acc = acc.concat(app.kTsOpen + app.msOf(line) + app.kTsClose
                                         + app.kErrTag + errText)
                        continue
                    }

                    // Service notice (voice/TTS/STT/catalog messages): worker
                    // sends `[[s:KEY\u001fARG…]]`, we translate by the app
                    // language and render as a SYSTEM bubble (no speak button,
                    // no auto-TTS binding — see isSvc in ChatPage).
                    if (app.lineIs(bodyTmp, app.kSvcOpen)) {
                        acc = acc.concat(app.kTsOpen + app.msOf(line) + app.kTsClose
                                         + app.kSvcTag + app.trSvc(app.kSvcOpen, bodyTmp))
                        continue
                    }

                    // Control line: the STT cycle finished (terminal message
                    // was sent before it). Stop the mic loader; not a bubble.
                    if (app.lineIs(bodyTmp, app.kSttDoneTag)) {
                        app.recognizing = false
                        continue
                    }

                    // Live model reasoning ([[thinklive]]): the worker streams
                    // deltas while the model thinks. Replace the trailing
                    // [[think]] bubble in place (or start a new one) so the text
                    // grows live instead of stacking a bubble per chunk.
                    if (app.lineIs(bodyTmp, app.kThinkLiveTag)) {
                        if (!app.showReasoning)
                            continue
                        var rText = bodyTmp.substring(app.kThinkLiveTag.length)
                        var rLine = app.kTsOpen + app.msOf(line) + app.kTsClose
                                    + app.kThinkTag + rText
                        if (acc.length > 0
                            && app.lineIs(app.stripTs(acc[acc.length - 1]), app.kThinkTag)) {
                            acc[acc.length - 1] = rLine
                        } else {
                            acc = acc.concat(rLine)
                        }
                        continue
                    }

                    // Model reasoning ([[think]]) — Rust sends it as a line
                    // before the answer; hidden when the setting is off.
                    if (app.lineIs(bodyTmp, app.kThinkTag) && !app.showReasoning)
                        continue

                    // Terminal recognition result — drop the loader.
                    if (app.lineIs(line, app.kSttTag))
                        app.recognizing = false
                        app.recognizing = false

                    // Recognised (STT): into the input or straight to the agent — depending on settings.
                    if (app.lineIs(line, app.kSttTag)) {
                        var text = line.substring(app.kSttTag.length).replace(/^\s+/, "")
                        if (text !== "") {
                            if (app.inputMode === "voice" || app.sendImmediately) {
                                // Bubble into acc: app.messages = acc comes after the loop,
                                // otherwise the bubble added by send() would be overwritten.
                                acc = acc.concat(app.kTsOpen + Date.now() + app.kTsClose
                                                + app.kUserPrefix + text)
                                bridge.send_prompt(text)
                            } else {
                                app.dictated(text)
                            }
                        }
                        continue
                    }

                    // TTS synthesis done: put the file into the queue, do not add it to the feed.
                    if (app.lineIs(line, app.kTtsTag)) {
                        app.recognizing = false
                        var ttsPath = line.substring(app.kTtsTag.length).replace(/^\s+/, "")
                        if (ttsPath !== "")
                            app.ttsEnqueue(ttsPath)
                        continue
                    }

                    // Remember the last agent answer — auto-synthesised
                    // WAVs are bound to it (they come after the text). User
                    // lines, reasoning and system bubbles (service/error)
                    // are not counted as an answer.
                    if (bodyTmp !== "" && !app.lineIs(bodyTmp, app.kUserPrefix)
                        && !app.lineIs(bodyTmp, app.kThinkTag)
                        && !app.lineIs(bodyTmp, app.kErrTag)
                        && !app.lineIs(bodyTmp, app.kSvcTag))
                        app.pendingTtsBody = bodyTmp.replace(/^\n+/, "")

                    acc = acc.concat(line)
                }
            }
            app.messages = acc
        }
        var sess = bridge.sessions_json()
        if (sess !== "" && sess !== undefined) {
            try {
                app.sessions = JSON.parse(sess)
            } catch (e) {
                console.log("sessions parse error: " + e)
            }
        }
        var t = bridge.todo_json()
        if (t !== "" && t !== undefined) {
            try {
                app.todos = JSON.parse(t)
            } catch (e) {
                console.log("todo parse error: " + e)
            }
        }
        var md = bridge.models_json()
        if (md !== "" && md !== undefined) {
            try {
                app.models = JSON.parse(md)
            } catch (e) {
                console.log("models parse error: " + e)
            }
        }
        if (app.sounds.length === 0) {
            var snd = bridge.sounds_json()
            if (snd !== "" && snd !== undefined) {
                try {
                    app.sounds = JSON.parse(snd)
                    if (app.sounds.length > 0
                        && (dingSetting.value === undefined || dingSetting.value === ""))
                        dingSetting.value = app.sounds[0].path
                } catch (e) {
                    console.log("sounds parse error: " + e)
                }
            }
        }
        var cid = bridge.current_session_id()
        if (cid !== undefined && cid !== null && cid !== app.currentSessionId) {
            app.currentSessionId = cid
            // A new session became active — apply the mode saved for it
            // (Build/Plan) if any; sessions without a saved choice live
            // in the server's default mode.
            if (cid !== "" && cid in app.sessionModes
                    && !(cid in app.sessionModesApplied)) {
                app.sessionModesApplied[cid] = true
                bridge.set_mode(cid, app.sessionModes[cid])
            }
        }
        var nav = bridge.take_nav()
        if (nav !== undefined && nav >= 0)
            app.requestPage(nav)
        var v = bridge.voice_status_json()
        if (v !== "" && v !== undefined) {
            try {
                var vo = JSON.parse(v)
                app.voiceModels = vo.models || []
                app.chosenStt = vo.stt || ""
                app.chosenTts = vo.tts || ""
                app.ttsModelReady = app.chosenTts !== ""
                app.voiceLang = vo.lang || "auto"
                if (vo.langs && vo.langs.length) {
                    var opts = [{ id: "auto", name: qsTr("Auto (detect)") }]
                    app.voiceLangOptions = opts.concat(vo.langs)
                }
            } catch (e) {
                console.log("voice parse error: " + e)
            }
        }
        var pj = bridge.permissions_json()
        if (pj !== "" && pj !== undefined) {
            try {
                var parsed = JSON.parse(pj)
                app.pendingPermissions = parsed
                // New requests (not shown yet) — signal the user.
                var fresh = false
                for (var pi = 0; pi < parsed.length; pi++) {
                    var pid = parsed[pi].id
                    if (app.knownPermissionIds.indexOf(pid) < 0) {
                        app.knownPermissionIds = app.knownPermissionIds.concat(pid)
                        fresh = true
                    }
                }
                if (fresh) {
                    app.notifyPermission()
                    app.showPermissionOverlay()
                }
            } catch (e) {
                console.log("permissions parse error: " + e)
            }
        }
        var sj = bridge.settings_json()
        if (sj !== "" && sj !== undefined) {
            try {
                var sjson = JSON.parse(sj)
                if (sjson.workdir !== undefined && sjson.workdir !== "")
                    app.workdir = sjson.workdir
            } catch (e) {
                console.log("settings parse error: " + e)
            }
        }
    }

    function statusColor(s) {
        if (s.indexOf("retry|") === 0) return "#ff9800"
        switch (s) {
        case "connecting": return "#2196f3"
        case "idle":       return "#4caf50"
        case "busy":       return "#ffc107"
        case "error":      return "#f44336"
        default:           return Theme.secondaryColor
        }
    }

    // Base status independent of the retry detail token (`retry|…` counts as
    // busy). The worker sets `retry|<next_ms>|<attempt>|<message>` while the
    // server is retrying a failed model call (see main.rs `session.status`).
    function statusBase(s) {
        return s.indexOf("retry|") === 0 ? "busy" : s
    }

    function send(text) {
        var t = ("" + text).trim()
        if (t === "")
            return
        var acc = app.messages
        acc = acc.concat(app.kTsOpen + Date.now() + app.kTsClose
                        + app.kUserPrefix + t)
        app.messages = acc
        bridge.send_prompt(t)
    }

    // Session mode (build/plan agent) — a property of a particular session. The
    // choice is remembered per session in sessionModes and applied when it is
    // opened (in poll()) or immediately if the session is active.
    function sessionMode(sid) {
        if (sid && sid in app.sessionModes && app.sessionModes[sid])
            return app.sessionModes[sid]
        return "build"
    }
    function setSessionMode(sid, mode) {
        if (!sid || (mode !== "build" && mode !== "plan"))
            return
        app.sessionModes[sid] = mode
        sessionModesSetting.value = JSON.stringify(app.sessionModes)
        sessionModesSetting.sync()
        if (sid === app.currentSessionId) {
            app.sessionModesApplied[sid] = true
            bridge.set_mode(sid, mode)
        }
    }

    function clearHistory() { app.messages = [] }

    // --- Voice module -----------------------------------------------------
    function voiceCmd(cmd, val) {
        bridge.voice_command(cmd, val === undefined ? "" : val)
    }
    function startPttHold() {
        if (!app.recording) {
            app.recognizing = false
            app.recording = true
            app.voiceCmd(app.cmdRecordStart, "")
        }
    }
    function stopPttHold() {
        if (app.recording) {
            app.recording = false
            app.recognizing = true
            app.voiceCmd(app.cmdRecordStop, "")
        }
    }
    function voiceModelsFor(engine) {
        var out = []
        for (var i = 0; i < app.voiceModels.length; i++)
            if (app.voiceModels[i].engine === engine)
                out.push(app.voiceModels[i])
        return out
    }
    function modelName(id, list) {
        for (var i = 0; i < list.length; i++)
            if (list[i].model_id === id)
                return list[i].name
        return "—"
    }
    function sttModelName() {
        return app.modelName(app.chosenStt, app.voiceModelsFor(app.engineStt))
    }
    function ttsModelName() {
        return app.modelName(app.chosenTts, app.voiceModelsFor(app.engineTts))
    }
    function humanSize(b) {
        if (!b) return ""
        var g = b / (1024*1024*1024)
        if (g >= 1) return g.toFixed(1) + " GB"
        var m = b / (1024*1024)
        if (m >= 1) return Math.round(m) + " MB"
        return Math.round(b / 1024) + " KB"
    }

    function newSession() {
        app.messages = []
        bridge.new_session()
        app.requestPage(2)
    }

    function openSession(id) {
        app.messages = []
        bridge.open_session(id)
        app.requestPage(2)
    }

    function openSessionSettings(id) {
        app.messages = []
        bridge.open_session(id)
        app.requestPage(3)
    }

    function renameSession(id, title) {
        bridge.rename_session(id, title)
    }

    function deleteSession(id) {
        bridge.delete_session(id)
    }

    function deleteAllSessions() {
        // Empty new session: reset the feed, otherwise the old history would
        // remain after delete_all (Rust creates the new session asynchronously).
        app.messages = []
        app.currentSessionId = ""
        bridge.delete_all_sessions()
        app.requestPage(2)
    }

    function forkSession(id) {
        app.messages = []
        app.currentSessionId = ""
        bridge.fork_session(id)
    }

    function shareSession(id) { bridge.share_session(id) }
    function unshareSession(id) { bridge.unshare_session(id) }
    function summarizeSession(id) { bridge.summarize_session(id) }
    function abortSession(id) { bridge.abort_session(id) }
    function setModel(id, provider, model) { bridge.set_model(id, provider, model) }

    function setShowReasoning(v) {
        app.showReasoning = !!v
        showReasoningSetting.value = app.showReasoning
        showReasoningSetting.sync()
    }

    onInputModeChanged: { inputModeSetting.value = app.inputMode; inputModeSetting.sync() }
    onShowToolsChanged: { showToolsSetting.value = app.showTools; showToolsSetting.sync() }
    onPttPositionChanged: { pttPositionSetting.value = app.pttPosition; pttPositionSetting.sync() }
    onTtsModeChanged: {
        ttsModeSetting.value = app.ttsMode; ttsModeSetting.sync()
        // Sync the mode with the Rust side (tts_mode), otherwise auto-TTS will not turn on.
        app.voiceCmd(app.cmdTtsMode, app.ttsMode)
    }
    onSoundOnFinishChanged: { soundOnFinishSetting.value = app.soundOnFinish; soundOnFinishSetting.sync() }
    onSoundOnPermissionChanged: { soundOnPermissionSetting.value = app.soundOnPermission; soundOnPermissionSetting.sync() }
    onSendImmediatelyChanged: { sendImmediatelySetting.value = app.sendImmediately; sendImmediatelySetting.sync() }
    onShowReasoningChanged: { showReasoningSetting.value = app.showReasoning; showReasoningSetting.sync() }
    onDingSoundChanged: { if (app.dingSound) dingSetting.value = app.dingSound; dingSetting.sync() }

    // At startup we sync the saved TTS mode (auto/button/off)
    // with the Rust side — otherwise `onTtsModeChanged` will not fire for the initial value.
    Component.onCompleted: {
        app.voiceCmd(app.cmdTtsMode, app.ttsMode)
    }

    function playDing() {
        var slots = [ding1, ding2, ding3, ding4]
        var slot = null
        for (var i = 0; i < slots.length; i++) {
            var s = slots[(app.dingSlot + i) % slots.length]
            if (!s.playing) {
                slot = s
                app.dingSlot = (app.dingSlot + i + 1) % slots.length
                break
            }
        }
        if (!slot) {
            slot = slots[app.dingSlot]
            slot.stop()
            app.dingSlot = (app.dingSlot + 1) % slots.length
        }
        slot.source = app.dingSound
        slot.play()
    }

    function stopDing() {
        var slots = [ding1, ding2, ding3, ding4]
        for (var i = 0; i < slots.length; i++)
            slots[i].stop()
    }

    function setDingSound(path) {
        app.dingSound = path
        dingSetting.value = path
        dingSetting.sync()
    }

    // Preview of the selected tune: first mute all slots, then play
    // with a small pause — an instant play after stop often fails
    // (the sound works "every other time").
    Timer {
        id: dingPreviewTimer
        interval: 80
        onTriggered: app.playDing()
    }
    function previewDing(path) {
        app.setDingSound(path)
        app.stopDing()
        dingPreviewTimer.start()
    }

    function dingSoundName() {
        for (var i = 0; i < app.sounds.length; i++)
            if (app.sounds[i].path === app.dingSound)
                return app.sounds[i].name
        return app.dingSound
    }

    function publishNotification(summary, body) {
        notify.summary = summary
        notify.body = body
        notify.previewSummary = summary
        notify.previewBody = body
        notify.sound = app.dingSound
        notify.publish()
    }

    // Server error: always-visible notification + status.
    function publishError(msg) {
        app.statusText = "error"
        if (msg === "") msg = qsTr("Server error")
        app.playDing()
        app.publishNotification(qsTr("Server error"), msg)
    }

    // Dev (debug build only): generate a mock server error into the feed.
    function mockServerError(kind) {
        if (app.devTools)
            bridge.mock_error(kind)
    }

    // Dev (only in a debug build): generate a mock agent access request.
    function mockPermission() {
        if (app.devTools)
            bridge.mock_permission()
    }

    function notifyAgentFinished() {
        if (!app.soundOnFinish)
            return
        if (Qt.application.state === Qt.ApplicationActive) {
            app.playDing()
        } else {
            app.publishNotification(qsTr("opencode"), qsTr("Agent finished"))
        }
    }

    function notifyPermission() {
        if (!app.soundOnPermission)
            return
        if (Qt.application.state === Qt.ApplicationActive) {
            app.playDing()
        } else {
            app.publishNotification(qsTr("opencode"), qsTr("Agent requests permission"))
        }
    }
    function stopAgent() {
        if (app.currentSessionId !== "")
            app.abortSession(app.currentSessionId)
    }

    // Answer to an agent permission request: once / always / reject.
    function answerPermission(session, id, response) {
        bridge.answer_permission(session, id, response)
    }

    // show/hide the permission request overlay.
    property bool permissionOverlayVisible: false
    function showPermissionOverlay() {
        app.permissionOverlayVisible = app.pendingPermissions.length > 0
    }

    initialPage: Component {
        Page {
            Connections {
                target: app
                onRequestPage: pager.currentIndex = index
            }

            // Carousel: [0] Global | [1] Sessions | [2] Chat | [3] Session
            // PagedView gives snap and horizontal swipe (snapMode on Flickable is a bug, see memory).
            PagedView {
                id: pager
                anchors.fill: parent
                currentIndex: 2
                wrapMode: PagedView.NoWrap
                model: [globalPage, sessionsPage, chatPage, sessionSettingsPage]

                delegate: Item {
                    width: PagedView.contentWidth
                    height: PagedView.contentHeight

                    Loader {
                        anchors.fill: parent
                        sourceComponent: modelData
                    }
                }

                Component { id: globalPage; GlobalSettingsPage { appWindow: app } }
                Component { id: sessionsPage; SessionsPage { appWindow: app } }
                Component { id: chatPage; ChatPage { appWindow: app } }
                Component { id: sessionSettingsPage; SessionSettingsPage { appWindow: app } }
            }

            // Dot indicator (SFOS 5.1 has no ready-made PageIndicator — we draw it ourselves).
            Row {
                id: dots
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.top
                anchors.topMargin: Theme.paddingMedium
                spacing: Theme.paddingSmall
                visible: pager.count > 1
                opacity: pager.dragging ? 1.0 : Theme.opacityHigh
                z: 10

                Repeater {
                    model: pager.count
                    Rectangle {
                        width: Theme.paddingSmall
                        height: width
                        radius: width / 2
                        color: index === pager.currentIndex ? Theme.primaryColor : Theme.secondaryColor
                    }
                }
            }

            // Agent permission request overlay (on top of the carousel).
            // We put the background dimming in a separate Rectangle WITHOUT opacity
            // on the container: otherwise the text and buttons become transparent.
            Rectangle {
                id: permissionOverlay
                visible: app.permissionOverlayVisible && app.pendingPermissions.length > 0
                anchors.fill: parent
                color: Qt.rgba(0, 0, 0, 0.65)
                z: 20

                // The card with the request text is almost opaque so it stays readable.
                Rectangle {
                    id: permissionCard
                    anchors.centerIn: parent
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    color: Qt.rgba(0.05, 0.05, 0.08, 0.95)
                    radius: Theme.paddingMedium
                    border.color: Theme.primaryColor
                    border.width: 1

                    Column {
                        anchors {
                            top: parent.top
                            left: parent.left
                            right: parent.right
                            bottom: parent.bottom
                            margins: Theme.paddingMedium
                        }
                        spacing: Theme.paddingMedium

                        Label {
                            text: qsTr("Agent requests permission")
                            anchors.horizontalCenter: parent.horizontalCenter
                            color: Theme.primaryColor
                            font.pixelSize: Theme.fontSizeLarge
                            font.bold: true
                            wrapMode: Text.Wrap
                        }

                        Label {
                            text: app.pendingPermissions[0]
                                  ? (app.pendingPermissions[0].action || "")
                                    + (app.pendingPermissions[0].resources
                                       ? (": " + app.pendingPermissions[0].resources
                                            .map(function (r) { return r }).join(", "))
                                       : "")
                                  : ""
                            anchors.horizontalCenter: parent.horizontalCenter
                            color: Theme.highlightColor
                            width: parent.width
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.Wrap
                            font.pixelSize: Theme.fontSizeMedium
                        }

                        Item {
                            width: parent.width
                            height: Theme.paddingLarge
                        }

                        Button {
                            text: qsTr("Allow once")
                            anchors.horizontalCenter: parent.horizontalCenter
                            onClicked: {
                                app.answerPermission(
                                    app.pendingPermissions[0].sessionID,
                                    app.pendingPermissions[0].id,
                                    "once")
                                app.permissionOverlayVisible = false
                            }
                        }
                        Button {
                            text: qsTr("Always allow")
                            anchors.horizontalCenter: parent.horizontalCenter
                            onClicked: {
                                app.answerPermission(
                                    app.pendingPermissions[0].sessionID,
                                    app.pendingPermissions[0].id,
                                    "always")
                                app.permissionOverlayVisible = false
                            }
                        }
                        Button {
                            text: qsTr("Deny")
                            anchors.horizontalCenter: parent.horizontalCenter
                            onClicked: {
                                app.answerPermission(
                                    app.pendingPermissions[0].sessionID,
                                    app.pendingPermissions[0].id,
                                    "reject")
                                app.permissionOverlayVisible = false
                            }
                        }
                    }
                }
            }
        }
    }
}
