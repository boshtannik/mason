import QtQuick 2.6
import Sailfish.Silica 1.0

ApplicationWindow {
    id: app

    property var messages: []
    property string statusText: "connecting"

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
        app.messages = app.messages.concat(">>> " + t)
        bridge.send_prompt(t)
    }

    initialPage: Component {
        Page {
            id: page

            SilicaListView {
                id: chatList
                anchors {
                    left: parent.left
                    right: parent.right
                    top: parent.top
                    bottom: inputPanel.top
                }
                model: app.messages
                clip: true
                spacing: Theme.paddingSmall

                header: Item {
                    width: chatList.width
                    height: Theme.itemSizeExtraSmall

                    Row {
                        anchors {
                            left: parent.left
                            leftMargin: Theme.horizontalPageMargin
                            verticalCenter: parent.verticalCenter
                        }
                        spacing: Theme.paddingSmall

                        Label {
                            text: "●"
                            color: app.statusColor(app.statusText)
                            font.pixelSize: Theme.fontSizeSmall
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Label {
                            text: "opencode — " + app.statusText
                            color: Theme.secondaryColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                }

                delegate: Label {
                    x: Theme.horizontalPageMargin
                    width: chatList.width - 2 * Theme.horizontalPageMargin
                    text: modelData
                    wrapMode: Text.Wrap
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.primaryColor
                }

                onCountChanged: positionViewAtEnd()

                VerticalScrollDecorator {}
            }

            Item {
                id: inputPanel
                anchors {
                    left: parent.left
                    right: parent.right
                    bottom: parent.bottom
                }
                height: input.height + Theme.paddingMedium

                Row {
                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        leftMargin: Theme.paddingSmall
                        rightMargin: Theme.paddingSmall
                    }
                    spacing: Theme.paddingSmall

                    TextField {
                        id: input
                        width: parent.width - sendButton.width - Theme.paddingSmall
                        placeholderText: qsTr("Промпт агенту…")
                        EnterKey.enabled: text.length > 0
                        EnterKey.iconSource: "image://theme/icon-m-enter-accept"
                        EnterKey.onClicked: {
                            app.send(input.text)
                            input.text = ""
                        }
                    }

                    Button {
                        id: sendButton
                        text: "→"
                        onClicked: {
                            app.send(input.text)
                            input.text = ""
                        }
                    }
                }
            }
        }
    }
}
