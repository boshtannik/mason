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
    property string inputMode: "text_ptt"
    property var sessions: []
    property var todos: []
    property var models: []
    property string currentSessionId: ""
    property bool showTools: true
    property string pttPosition: "left"
    property string ttsMode: "text"
    property bool soundOnFinish: true
    property bool soundOnPermission: true
    property bool sendImmediately: false
    property string uiLanguage: "ru"
    property bool sttModelReady: false
    property bool ttsModelReady: false

    // Для детекта перехода "агент занят → свободен".
    property bool wasBusy: false

    // Системные звуки для уведомлений (сканирует воркер, `[{name,path}]`).
    property var sounds: []
    // Выбранный звук уведомления (сохраняется в настройках DConf).
    property string dingSound: dingSetting.value

    // Голосовой модуль: статус из воркера (`voice_status_json`).
    property var voiceModels: []
    property variant voiceLangOptions: ["auto"]
    property string chosenStt: ""
    property string chosenTts: ""
    property string voiceLang: "auto"
    // Идёт ли запись с микрофона (PTT-переключатель).
    property bool recording: false

    // Запрос переключить страницу карусели (0..3).
    signal requestPage(int index)

    ConfigurationValue {
        id: dingSetting
        key: "/apps/harbour-opencode/dingSound"
    }

    // Держим процесс живым, пока агент работает (иначе Sailfish усыпит его в фоне).
    KeepAlive {
        id: agentKeepAlive
        enabled: app.statusText === "busy"
    }

    // "Дзинь" на переднем плане (в фоне играет система по уведомлению).
    SoundEffect {
        id: ding
        source: app.dingSound
    }

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
            var lines = msgs.split("\n")
            var acc = app.messages
            for (var i = 0; i < lines.length; i++) {
                var line = lines[i]
                if (line !== "") {
                    // Служебные метки воркера сразу убираем из текста.
                    line = line.replace(/^\[\[stt\]\] /, "")
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
                app.voiceLang = vo.lang || "auto"
                if (vo.langs && vo.langs.length)
                    app.voiceLangOptions = ["auto"].concat(vo.langs)
            } catch (e) {
                console.log("voice parse error: " + e)
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
        acc = acc.concat(">>> " + t)
        app.messages = acc
        bridge.send_prompt(t)
    }

    function clearHistory() { app.messages = [] }

    // --- Голосовой модуль -------------------------------------------------
    function voiceCmd(cmd, val) {
        bridge.voice_command(cmd, val === undefined ? "" : val)
    }
    function startPtt() {
        if (!app.recording) {
            app.recording = true
            app.voiceCmd("voice_record_start", "")
        } else {
            app.recording = false
            app.voiceCmd("voice_record_stop", "")
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
        return app.modelName(app.chosenStt, app.voiceModelsFor("stt_whisper"))
    }
    function ttsModelName() {
        return app.modelName(app.chosenTts, app.voiceModelsFor("tts_piper"))
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

    function playDing() {
        ding.play()
    }

    function setDingSound(path) {
        app.dingSound = path
        dingSetting.value = path
        dingSetting.sync()
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

    function notifyAgentFinished() {
        if (!app.soundOnFinish)
            return
        if (Qt.application.state === Qt.ApplicationActive) {
            app.playDing()
        } else {
            app.publishNotification(qsTr("opencode"), qsTr("Агент завершил работу"))
        }
    }

    function notifyPermission() {
        if (!app.soundOnPermission)
            return
        if (Qt.application.state === Qt.ApplicationActive) {
            app.playDing()
        } else {
            app.publishNotification(qsTr("opencode"), qsTr("Агент запрашивает разрешение"))
        }
    }
    function stopAgent() {
        if (app.currentSessionId !== "")
            app.abortSession(app.currentSessionId)
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
        }
    }
}
