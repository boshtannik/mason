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
    // True только в debug-сборке: включает кнопки моков (ошибки/пермишны).
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
    // Показывать ли в ленте «мышление» модели (строки [[think]]), а не только
    // готовые ответы. Включено по умолчанию.
    property bool showReasoning: showReasoningSetting.value !== undefined
                                 ? showReasoningSetting.value : true
    // Ожидающие запросы разрешений агента (`[{id,sessionID,action,resources,options}]`).
    property var pendingPermissions: []
    // Дать согласие «Разрешить один раз»: true, если запрос действительно новый
    // (для срабатывания звука только на появившихся).
    property var knownPermissionIds: []
    property bool sttModelReady: false
    // Рабочая директория opencode (настройка). Загружается из bridge.settings_json().
    property string workdir: ""
    property bool ttsModelReady: chosenTts !== ""

    // Для детекта перехода "агент занят → свободен".
    property bool wasBusy: false

    // Системные звуки для уведомлений (сканирует воркер, `[{name,path}]`).
    property var sounds: []
    // Выбранный звук уведомления (сохраняется в настройках DConf).
    property string dingSound: dingSetting.value

    // Голосовой модуль: статус из воркера (`voice_status_json`).
    property var voiceModels: []
    property variant voiceLangOptions: [ { id: "auto", name: "Auto (detect)" } ]
    property string chosenStt: ""
    property string chosenTts: ""
    property string voiceLang: "auto"
    // Идёт ли запись с микрофона (PTT-переключатель).
    property bool recording: false
    // Идёт ли распознавание речи после остановки записи.
    property bool recognizing: false
    // Идёт ли озвучка ответа (для отображения кнопки «стоп»).
    property bool ttsPlaying: false
    // Текст баббла, который сейчас синтезируется/озвучивается (для лоадера
    // на соответствующей кнопке озвучки). Пустая строка — ничего не озвучиваем.
    property string speakingBubble: ""
    // Идёт ли синтез речи: команда отправлена воркеру, WAV ещё не пришёл
    // в очередь (это самая «тихая» фаза, когда нужна обратная связь).
    property bool ttsSynth: false

    // Протокольные константы голосового модуля.
    // Команды зеркалят `voice::cmd` в src/voice.rs, движки — `voice::engine`,
    // фазы — `voice::Phase` (as_str). Менять их можно только с обеих сторон.
    // Внимание: QML запрещает имена свойств с большой буквы — только lowercase.
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

    // Запрос переключить страницу карусели (0..3).
    signal requestPage(int index)

    // Распознанный речевой текст, который надо показать в поле ввода чата.
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

    // Держим процесс живым, пока агент работает (иначе Sailfish усыпит его в фоне).
    KeepAlive {
        id: agentKeepAlive
        enabled: app.statusText === "busy"
    }

    // Пул "диней": SoundEffect нельзя перезапустить, пока играет, поэтому
// на каждый вызов берём следующий свободный слот (ротация).
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

    Timer {
        id: pollTimer
        interval: 300
        running: true
        repeat: true
        onTriggered: app.poll()
    }

    // Очередь озвучки: WAV-файлы синтезируются воркером асинхронно
    // (несколько сообщений подряд = несколько файлов). Играем строго по
    // порядку появления: mediaplayer закончил → берём следующий из очереди.
    // Каждый элемент — `{path, body}`: body — текст баббла, которому
    // принадлежит файл, чтобы кнопка «стоп/озвучка» следовала за текущим
    // воспроизводимым сообщением (см. speakingBubble).
    property var ttsQueue: []

    // Тело последнего ответа агента — привязка автосинтезированных файлов
    // к бабблу: воркер шлёт `[[tts]]<путь>` после соответствующих текстовых
    // строк, поэтому запоминаем последний ответ. См. ttsEnqueue/playNextTts.
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
        // Индикатор озвучки едет вместе с очередью: кнопка «стоп» включается
        // на том баббле, чей звук реально сейчас играет.
        app.speakingBubble = item.body
        app.ttsPlaying = true
        app.ttsSynth = false
        ttsPlayer.play()
    }
    function ttsEnqueue(path) {
        if (path === "" || path === undefined)
            return
        // К какому бабблу относится файл: для ручной кнопки это текст, который
        // мы отправили на синтез (speakingBubble); для автоозвучки — последний
        // ответ агента (pendingTtsBody).
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
        // Просим воркер не отдавать WAV, если piper ещё синтезирует,
        // иначе файл «дозреет» и озвучка снова включится.
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
        if (app.statusText === "busy") {
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
                    // Строка без временной метки — используем для маркеров
                    // [[think]] и для привязки автоозвучки к бабблу.
                    var bodyTmp = line.replace(/^\[\[t:\d+\]\]/, "")

                    // Ошибка сервера (например "free usage exceeded") — сразу в нотификацию
                    // и в ленту, чтобы не выглядело, будто агент молча работает.
                    if (line.indexOf("[ошибка сервера]") === 0) {
                        app.recognizing = false
                        var errText = line.substring("[ошибка сервера]".length).replace(/^\s+/, "")
                        app.publishError(errText)
                        acc = acc.concat(line)
                        continue
                    }

                    // Рассуждения модели ([[think]]) — при настройке «показывать
                    // мышление» Rust присылает их строкой перед ответом; если
                    // отключено — просто не показываем.
                    if (bodyTmp.substring(0, 8) === "[[think]]" && !app.showReasoning)
                        continue

                    // Терминальный результат распознавания — снимаем лоадер.
                    if (line.substring(0, 7) === "[[stt]]"
                        || line.indexOf("[голос] ошибка") === 0
                        || line.indexOf("[голос] распознано пусто") === 0
                        || line.indexOf("[голос] модель не скачана") === 0)
                        app.recognizing = false

                    // Распознанное (STT): в инпут или сразу агенту — по настройкам.
                    if (line.substring(0, 7) === "[[stt]]") {
                        var text = line.substring(7).replace(/^\s+/, "")
                        if (text !== "") {
                            if (app.inputMode === "voice" || app.sendImmediately) {
                                // Баббл в acc: после цикла идёт app.messages = acc,
                                // иначе добавленный send() баббл затёрся бы.
                                acc = acc.concat("[[t:" + Date.now() + "]]>>> " + text)
                                bridge.send_prompt(text)
                            } else {
                                app.dictated(text)
                            }
                        }
                        continue
                    }

                    // Синтез озвучки готов: файл в стопку очереди, в ленту не добавляем.
                    if (line.substring(0, 7) === "[[tts]]") {
                        app.recognizing = false
                        var ttsPath = line.substring(7).replace(/^\s+/, "")
                        if (ttsPath !== "")
                            app.ttsEnqueue(ttsPath)
                        continue
                    }

                    // Запоминаем последний ответ агента — к нему привязываем
                    // автосинтезированные WAV (идут после текста). Строчки
                    // пользователя и рассуждения не считаем ответом.
                    if (bodyTmp !== "" && bodyTmp.substring(0, 4) !== ">>> "
                        && bodyTmp.substring(0, 8) !== "[[think]]")
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
        if (cid !== undefined && cid !== null && cid !== app.currentSessionId)
            app.currentSessionId = cid
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
                // Новые запросы (ещё не показанные) — просигналить пользователю.
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
        switch (s) {
        case "connecting": return "#2196f3"
        case "idle":       return "#4caf50"
        case "busy":       return "#ffc107"
        case "error":      return "#f44336"
        default:           return Theme.secondaryColor
        }
    }

    function send(text) {
        var t = ("" + text).trim()
        if (t === "")
            return
        var acc = app.messages
        acc = acc.concat("[[t:" + Date.now() + "]]>>> " + t)
        app.messages = acc
        bridge.send_prompt(t)
    }

    function clearHistory() { app.messages = [] }

    // --- Голосовой модуль -------------------------------------------------
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
        // Пустая новая сессия: сбросить ленту, иначе после delete_all останется
        // старая история (новую сессию Rust создаёт асинхронно).
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
        // Синхронизируем режим с Rust-стороной (tts_mode), иначе автоозвучка не включится.
        app.voiceCmd(app.cmdTtsMode, app.ttsMode)
    }
    onSoundOnFinishChanged: { soundOnFinishSetting.value = app.soundOnFinish; soundOnFinishSetting.sync() }
    onSoundOnPermissionChanged: { soundOnPermissionSetting.value = app.soundOnPermission; soundOnPermissionSetting.sync() }
    onSendImmediatelyChanged: { sendImmediatelySetting.value = app.sendImmediately; sendImmediatelySetting.sync() }
    onShowReasoningChanged: { showReasoningSetting.value = app.showReasoning; showReasoningSetting.sync() }
    onDingSoundChanged: { if (app.dingSound) dingSetting.value = app.dingSound; dingSetting.sync() }

    // При старте синхронизируем сохранённый режим озвучки (auto/button/off)
    // с Rust-стороной — иначе `onTtsModeChanged` не сработает для стартового значения.
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

    // Превью выбранной мелодии: сначала глушим все слоты, затем играем
    // с небольшой паузой — мгновенный play после stop часто не срабатывает
    // (звук «через раз»).
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

    // Ошибка сервера: всегда видимое уведомление + статус.
    function publishError(msg) {
        app.statusText = "error"
        if (msg === "") msg = qsTr("Server error")
        app.playDing()
        app.publishNotification(qsTr("Server error"), msg)
    }

    // Dev (только debug-сборка): сгенерировать мок-ошибку сервера в ленту.
    function mockServerError(kind) {
        if (app.devTools)
            bridge.mock_error(kind)
    }

    // Dev (только debug-сборка): сгенерировать мок-запрос доступа агента.
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

    // Ответ на запрос разрешения агента: once / always / reject.
    function answerPermission(session, id, response) {
        bridge.answer_permission(session, id, response)
    }

    // показать/скрыть оверлей с запросом разрешения.
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

            // Карусель: [0] Global | [1] Sessions | [2] Chat | [3] Session
            // PagedView даёт снап и горизонтальный свайп (snapMode у Flickable — баг, см. память).
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

            // Индикатор точками (в SFOS 5.1 готового PageIndicator нет — рисуем сами).
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

            // Оверлей запроса разрешения от агента (поверх карусели).
            // Затемнение фона выносим в отдельный Rectangle БЕЗ opacity на
            // контейнере: иначе прозрачными становятся текст и кнопки.
            Rectangle {
                id: permissionOverlay
                visible: app.permissionOverlayVisible && app.pendingPermissions.length > 0
                anchors.fill: parent
                color: Qt.rgba(0, 0, 0, 0.65)
                z: 20

                // Плашка с текстом запроса — почти непрозрачная, чтобы читалось.
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
