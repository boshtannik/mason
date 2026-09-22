import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Item {
    id: chatPage

    property var appWindow

    readonly property bool busy: appWindow.statusText === "busy"

    // ms (unix) → «HH:MM» местного времени.
    function tsLabel(ms) {
        if (!ms)
            return ""
        var d = new Date(Number(ms))
        var h = d.getHours(), m = d.getMinutes()
        return (h < 10 ? "0" : "") + h + ":" + (m < 10 ? "0" : "") + m
    }

    Connections {
        target: chatPage.appWindow
        onDictated: {
            if (text === "" || text === undefined)
                return
            var cur = input.text
            input.text = (cur.length > 0 ? cur + " " : "") + text
            input.focus = true
        }
    }

    StatusHeader {
        id: header
        appWindow: chatPage.appWindow
        width: parent.width
        anchors.top: parent.top
    }

    SilicaListView {
        id: chatList
        anchors {
            top: header.bottom
            left: parent.left
            right: parent.right
            bottom: inputPanel.top
        }
        model: appWindow.messages
        clip: true
        spacing: Theme.paddingSmall
        delegate: messageDelegate
        onCountChanged: positionViewAtEnd()
        VerticalScrollDecorator {}
    }

    Component {
        id: messageDelegate

        Item {
            id: line
            width: chatList.width
            height: bubble.height + Theme.paddingSmall

            // Сырая строка от воркера: `[[t:<ms>]]>>> текст` (юзер) или
            // `[[t:<ms>]]текст` (агент); живой юзер от `send()` — `[[t:<ms>]]>>> текст`.
            property string raw: "" + modelData
            property string tsMs: {
                var m = /\[\[t:(\d+)\]\]/.exec(line.raw)
                return m ? m[1] : ""
            }
            // Юзер-строки помечены префиксом «>>> » ПОСЛЕ таймстампа.
            property bool isUser: {
                var b = line.raw.replace(/^\[\[t:\d+\]\]/, "")
                return b.substring(0, 4) === ">>> "
            }
            property string body: {
                var b = line.raw.replace(/^\[\[t:\d+\]\]/, "")
                if (b.substring(0, 4) === ">>> ")
                    b = b.substring(4)
                return b.replace(/^\n+/, "")
            }
            readonly property real maxW: chatList.width - 2 * Theme.horizontalPageMargin

            Rectangle {
                id: bubble
                radius: Theme.paddingMedium
                color: line.isUser
                       ? Theme.highlightColor
                       : Theme.rgba(Theme.primaryColor, 0.12)
                width: Math.min(line.maxW,
                                Math.max(textLabel.implicitWidth, tsLabel.implicitWidth)
                                + 2 * Theme.paddingSmall)
                height: textLabel.height
                        + (tsLabel.visible ? tsLabel.height + 2 : 0)
                        + 2 * Theme.paddingSmall
                anchors.right: line.isUser ? parent.right : undefined
                anchors.left: line.isUser ? undefined : parent.left
                anchors.rightMargin: line.isUser ? Theme.horizontalPageMargin : 0
                anchors.leftMargin: line.isUser ? 0 : Theme.horizontalPageMargin

                Label {
                    id: textLabel
                    x: Theme.paddingSmall
                    y: Theme.paddingSmall
                    width: Math.min(line.maxW - 2 * Theme.paddingSmall, implicitWidth)
                    text: line.body
                    wrapMode: Text.Wrap
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.primaryColor
                }
                Label {
                    id: tsLabel
                    visible: line.tsMs !== ""
                    x: line.isUser
                       ? Theme.paddingSmall
                       : bubble.width - implicitWidth - Theme.paddingSmall
                    y: textLabel.y + textLabel.height + 2
                    text: chatPage.tsLabel(line.tsMs)
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                }
            }
        }
    }

    Item {
        id: inputPanel
        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
        height: input.visible
                ? (Math.max(input.height, Theme.itemSizeMedium)
                   + Theme.paddingMedium)
                : (Theme.itemSizeMedium + Theme.paddingMedium)

        // Максимальная высота поля ввода: 3 строки, чтобы не закрывало весь чат.
        readonly property real maxInputHeight:
            Theme.fontSizeSmall * 3 + Theme.paddingMedium * 2

        // ── Константные раскладки панели ввода ──────────────────────────────
        // Вариантов мало — задаём каждый явно через State (анкеры), без
        // вычисления координат (расстановка «плавающими» X ломалась).
        //  - text:      [input, send]
        //  - ptt left:  [mic, input, send]
        //  - ptt center:[input, mic, send]
        //  - ptt right: [input, mic, send]  (mic у кнопки send)
        //  - voice left/center/right: [mic, send] / [mic·центр, send] / [mic, send]
        property string layoutKey: {
            if (appWindow.inputMode === "voice")
                return "voice" + appWindow.pttPosition
            if (appWindow.inputMode === "text")
                return "text"
            // В режиме «текст + диктовка» центр недопустим (только слева/справа).
            if (appWindow.pttPosition === "center")
                return "pttleft"
            return "ptt" + appWindow.pttPosition
        }
        states: [
            State {
                name: "text"
                when: inputPanel.layoutKey === "text"
                AnchorChanges {
                    target: input
                    anchors.left: inputPanel.left
                    anchors.right: send.left
                }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "pttleft"
                when: inputPanel.layoutKey === "pttleft"
                AnchorChanges {
                    target: mic
                    anchors.left: inputPanel.left
                }
                AnchorChanges {
                    target: input
                    anchors.left: mic.right
                    anchors.right: send.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "pttcenter"
                when: inputPanel.layoutKey === "pttcenter"
                AnchorChanges {
                    target: mic
                    anchors.horizontalCenter: inputPanel.horizontalCenter
                }
                AnchorChanges {
                    target: input
                    anchors.left: inputPanel.left
                    anchors.right: mic.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "pttright"
                when: inputPanel.layoutKey === "pttright"
                AnchorChanges {
                    target: mic
                    anchors.right: send.left
                }
                AnchorChanges {
                    target: input
                    anchors.left: inputPanel.left
                    anchors.right: mic.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.rightMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "voiceleft"
                when: inputPanel.layoutKey === "voiceleft"
                AnchorChanges {
                    target: mic
                    anchors.left: inputPanel.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.leftMargin: Theme.paddingSmall }
            },
            State {
                name: "voicecenter"
                when: inputPanel.layoutKey === "voicecenter"
                AnchorChanges {
                    target: mic
                    anchors.horizontalCenter: inputPanel.horizontalCenter
                }
                PropertyChanges { target: mic; visible: true }
            },
            State {
                name: "voiceright"
                when: inputPanel.layoutKey === "voiceright"
                AnchorChanges {
                    target: mic
                    anchors.right: send.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.rightMargin: Theme.paddingSmall }
            }
        ]

        IconButton {
            id: mic
            visible: false
            width: Theme.itemSizeMedium
            height: Theme.itemSizeMedium
            anchors.bottom: inputPanel.bottom
            anchors.bottomMargin: Theme.paddingSmall
            // Горизонтальную привязку задаёт только state (left/центр/right),
            // чтобы не возникало конфликта анкеров при центровке.
            icon.source: (appWindow.recording || appWindow.recognizing)
                          ? "" : "image://theme/icon-m-mic"

            // Лоадер вместо иконки, пока идёт запись или распознавание.
            BusyIndicator {
                anchors.fill: parent
                running: appWindow.recording || appWindow.recognizing
                visible: running
            }

            // PTT-холд: зажал → запись, отпустил → стоп + распознавание.
            MouseArea {
                anchors.fill: parent
                onPressed: appWindow.startPttHold()
                onReleased: appWindow.stopPttHold()
                onCanceled: appWindow.stopPttHold()
            }
        }

        IconButton {
            id: send
            visible: !chatPage.busy
            width: Theme.itemSizeMedium
            height: Theme.itemSizeMedium
            anchors {
                right: inputPanel.right
                rightMargin: Theme.paddingSmall
                bottom: inputPanel.bottom
                bottomMargin: Theme.paddingSmall
            }
            icon.source: "image://theme/icon-m-enter-accept"
            enabled: input.text.length > 0
            onClicked: {
                appWindow.send(input.text)
                input.text = ""
            }
        }

        IconButton {
            id: stop
            visible: chatPage.busy
            width: Theme.itemSizeMedium
            height: Theme.itemSizeMedium
            anchors {
                right: inputPanel.right
                rightMargin: Theme.paddingSmall
                bottom: inputPanel.bottom
                bottomMargin: Theme.paddingSmall
            }
            icon.source: "image://theme/icon-m-stop"
            onClicked: appWindow.stopAgent()
        }

        TextArea {
            id: input
            visible: appWindow.inputMode !== "voice"
            enabled: !chatPage.busy
            background: null
            horizontalAlignment: Text.AlignLeft
            // Высота растёт до 3 строк, дальше не растёт.
            height: Math.min(implicitHeight, inputPanel.maxInputHeight)
            anchors.bottom: inputPanel.bottom
            anchors.bottomMargin: Theme.paddingSmall
            // Дефолтные анкеры (ширина в стартовой раскладке [input, send]);
            // state переопределяет их под выбранную схему.
            anchors.left: inputPanel.left
            anchors.leftMargin: Theme.paddingSmall
            anchors.right: send.left
            anchors.rightMargin: Theme.paddingSmall
            placeholderText: chatPage.busy
                             ? qsTr("Agent is working…")
                             : qsTr("Agent prompt…")
            EnterKey.enabled: text.length > 0 && !chatPage.busy
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: {
                appWindow.send(input.text)
                input.text = ""
            }
        }
    }
}
