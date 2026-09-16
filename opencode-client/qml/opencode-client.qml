import QtQuick 2.6
import QtQuick.Window 2.2
import QtQuick.Controls 2.2

Window {
    id: window
    // Временный host-QML. На Sailfish это станет Silica ApplicationWindow.
    visible: true
    width: 540
    height: 960
    title: "opencode-client"

    property var log: []

    Component.onCompleted: {
        console.log("[QML] loaded, bridge.session_status = " + bridge.session_status)
        window.show()
        window.requestActivate()
    }

    // Поллинг очереди сообщений от воркера (SSE-ответы).
    Timer {
        id: pollTimer
        interval: 300
        running: true
        repeat: true
        onTriggered: {
            var msgs = bridge.drain_messages()
            if (msgs !== "") {
                // QML получает строку (возможны \n между ответами).
                var lines = msgs.split("\n")
                for (var i = 0; i < lines.length; i++) {
                    if (lines[i] !== "")
                        log = log.concat(lines[i])
                }
                chatList.positionViewAtEnd()
            }
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 8

        Text {
            width: parent.width
            text: "Статус: " + bridge.session_status
            font.pixelSize: 16
            color: "dimgray"
        }

        // Лента сообщений.
        Rectangle {
            width: parent.width
            height: parent.height - 120
            color: "whitesmoke"
            border.color: "lightgray"

            ListView {
                id: chatList
                anchors.fill: parent
                model: log
                clip: true

                delegate: Text {
                    width: parent.width - 10
                    text: modelData
                    wrapMode: Text.Wrap
                    font.pixelSize: 14
                    color: "black"
                }
            }
        }

        // Ввод + отправка.
        Row {
            width: parent.width
            spacing: 8
            TextField {
                id: input
                width: parent.width - 60
                placeholderText: "Промпт для ассистента. Enter — отправить."
                onAccepted: send()
            }
            Button {
                width: 52
                height: input.height
                text: "→"
                onClicked: send()
            }
        }
    }

    function send() {
        var t = input.text.trim()
        if (t === "") return
        log = log.concat(">>> " + t)
        bridge.send_prompt(t)
        input.text = ""
        chatList.positionViewAtEnd()
    }
}