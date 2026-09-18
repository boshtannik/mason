import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: statusHeader
    height: Theme.itemSizeSmall

    property var appWindow

    Row {
        anchors {
            left: parent.left
            leftMargin: Theme.horizontalPageMargin
            verticalCenter: parent.verticalCenter
        }
        spacing: Theme.paddingSmall

        // Индикатор: цвет = статус сессии.
        Label {
            text: "●"
            color: appWindow.statusColor(appWindow.statusText)
            font.pixelSize: Theme.fontSizeSmall
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            text: "opencode"
            color: Theme.primaryColor
            font.pixelSize: Theme.fontSizeMedium
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            text: appWindow.statusText
            color: appWindow.statusColor(appWindow.statusText)
            font.pixelSize: Theme.fontSizeExtraSmall
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
