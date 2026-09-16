import QtQuick 2.6
import QtQuick.Window 2.2

Window {
    id: window
    visible: true
    width: 540
    height: 960
    title: "opencode-client"

    Component.onCompleted: {
        console.log("[QML] loaded, bridge.session_status = " + bridge.session_status)
        window.show()
        window.requestActivate()
    }

    Rectangle {
        anchors.fill: parent
        color: "lightgray"
        Text {
            anchors.centerIn: parent
            text: "bridge.session_status: " + bridge.session_status
        }
        Text {
            id: status
            anchors.top: parent.top
            anchors.topMargin: 24
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width - 40
            wrapMode: Text.Wrap
            font.pixelSize: 14
        }
    }
}