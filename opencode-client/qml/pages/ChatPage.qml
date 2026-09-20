import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Item {
    id: chatPage

    property var appWindow

    readonly property bool busy: appWindow.statusText === "busy"
    readonly property bool micRight: appWindow.pttPosition === "right"

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

            property bool isUser: ("" + modelData).substring(0, 4) === ">>> "
            property string rawBody: isUser ? ("" + modelData).substring(4) : ("" + modelData)

            property string ts: {
                var m = /\[\[t:(\d+)\]\]/.exec(rawBody)
                return m ? chatPage.tsLabel(m[1]) : ""
            }
            property string body: rawBody.replace(/\[\[t:\d+\]\]/, "").replace(/^\n+/, "")

            readonly property real maxW: chatList.width - 2 * Theme.horizontalPageMargin
            property real bubbleH: textLabel.implicitHeight + tsLabel.visible
                                ? tsLabel.height : 0
                                + 2 * Theme.paddingSmall

            Rectangle {
                id: bubble
                radius: Theme.paddingMedium
                color: line.isUser
                       ? Theme.highlightColor
                       : Theme.rgba(Theme.primaryColor, 0.12)
                width: Math.min(line.maxW, textLabel.implicitWidth + 2 * Theme.paddingSmall)
                height: textLabel.height + 2 * Theme.paddingSmall
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
