import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Item {
    id: chatPage

    property var appWindow

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
            property string body: isUser ? ("" + modelData).substring(4) : ("" + modelData)
            property real maxW: chatList.width - 2 * Theme.horizontalPageMargin

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
            visible: appWindow.inputMode !== "text"
            anchors {
                left: parent.left
                leftMargin: Theme.paddingSmall
                verticalCenter: parent.verticalCenter
            }
            icon.source: "image://theme/icon-m-mic"
            onClicked: appWindow.startPtt()
        }

        IconButton {
            id: send
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

        TextField {
            id: input
            visible: appWindow.inputMode !== "voice"
            anchors {
                left: mic.visible ? mic.right : parent.left
                right: send.left
                leftMargin: Theme.paddingSmall
                rightMargin: Theme.paddingSmall
                verticalCenter: parent.verticalCenter
            }
            placeholderText: qsTr("Промпт агенту…")
            EnterKey.enabled: text.length > 0
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: {
                appWindow.send(input.text)
                input.text = ""
            }
        }
    }
}
