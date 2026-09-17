import QtQuick 2.6
import Sailfish.Silica 1.0

ApplicationWindow {
    id: app

    property var messages: []

    Timer {
        id: pollTimer
        interval: 300
        running: true
        repeat: true
        onTriggered: app.poll()
    }

    function poll() {
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

                header: Label {
                    x: Theme.horizontalPageMargin
                    width: chatList.width - 2 * Theme.horizontalPageMargin
                    text: "● opencode — " + bridge.session_status
                    color: bridge.session_status === "error"
                           ? Theme.errorColor : Theme.highlightColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    truncationMode: TruncationMode.Fade
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
