import QtQuick 2.6
import Sailfish.Silica 1.0
import "pages"

ApplicationWindow {
    id: app

    property var messages: []
    property string statusText: "connecting"
    property string inputMode: "text_ptt"
    property var sessions: []
    property bool showTools: true
    property string pttPosition: "left"
    property string ttsMode: "text"
    property bool soundOnFinish: true
    property bool soundOnPermission: true
    property bool sendImmediately: false
    property string uiLanguage: "ru"
    property bool sttModelReady: false
    property bool ttsModelReady: false

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
        var msgs = bridge.drain_messages()
        if (msgs === "" || msgs === undefined)
            return
        var lines = msgs.split("\n")
        var acc = app.messages
        for (var i = 0; i < lines.length; i++) {
            if (lines[i] !== "")
                acc = acc.concat(lines[i])
        }
        app.messages = acc
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
    function startPtt() { console.log("PTT: not implemented yet") }
    function newSession() { console.log("newSession: TODO") }
    function openSession(id) { console.log("openSession: " + id) }

    initialPage: Component {
        Page {
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
