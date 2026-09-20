import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Item {
    id: chatPage

    property var appWindow

    readonly property bool busy: appWindow.statusText === "busy"
    readonly property bool micRight: appWindow.pttPosition === "right"

    // ms (unix) → «HH:MM» местного времени.
    function tsLabel(ms) {
        if (!ms)
            return ""
        var d = new Date(Number(ms))
        var h = d.getHours(), m = d.getMinutes()
        return (h < 10 ? "0" : "") + h + ":" + (m < 10 ? "0" : "") + m
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
        height: Math.max(input.implicitHeight, mic.implicitHeight, send.implicitHeight)
                + Theme.paddingMedium

        IconButton {
            id: mic
            visible: appWindow.inputMode !== "text" && !chatPage.busy
            anchors {
                left: chatPage.micRight ? undefined : parent.left
                leftMargin: chatPage.micRight ? 0 : Theme.paddingSmall
                right: chatPage.micRight ? send.left : undefined
                rightMargin: chatPage.micRight ? Theme.paddingSmall : 0
                verticalCenter: parent.verticalCenter
            }
            icon.source: appWindow.recording
                          ? "image://theme/icon-m-stop" : "image://theme/icon-m-mic"
            onClicked: appWindow.startPtt()
        }

        IconButton {
            id: send
            visible: !chatPage.busy
            anchors {
                right: parent.right
                rightMargin: Theme.paddingSmall
                verticalCenter: parent.verticalCenter
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
            anchors {
                right: parent.right
                rightMargin: Theme.paddingSmall
                verticalCenter: parent.verticalCenter
            }
            icon.source: "image://theme/icon-m-stop"
            onClicked: appWindow.stopAgent()
        }

        TextField {
            id: input
            visible: appWindow.inputMode !== "voice"
            enabled: !chatPage.busy
            anchors {
                left: chatPage.micRight
                      ? parent.left
                      : (mic.visible ? mic.right : parent.left)
                right: chatPage.busy
                       ? stop.left
                       : (chatPage.micRight ? mic.left : send.left)
                leftMargin: Theme.paddingSmall
                rightMargin: Theme.paddingSmall
                verticalCenter: parent.verticalCenter
            }
            placeholderText: chatPage.busy
                             ? qsTr("Агент работает…")
                             : qsTr("Промпт агенту…")
            EnterKey.enabled: text.length > 0 && !chatPage.busy
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: {
                appWindow.send(input.text)
                input.text = ""
            }
        }
    }
}
