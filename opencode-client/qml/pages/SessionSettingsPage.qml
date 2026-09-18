import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: sessionSettingsPage

    property var appWindow

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        PageHeader {
            id: pageHeader
            title: qsTr("Session")
        }

        Column {
            id: column
            anchors {
                top: pageHeader.bottom
                left: parent.left
                right: parent.right
            }

            SectionHeader { text: qsTr("Actions") }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Clear history")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: appWindow.clearHistory()
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                enabled: false
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Compact / Summary / Switch model")
                    color: Theme.secondaryColor
                }
            }

            SectionHeader { text: qsTr("Agent") }

            TextSwitch {
                text: qsTr("Show tools feed")
                checked: appWindow.showTools
                onCheckedChanged: appWindow.showTools = checked
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("TODO: агент, стриминг и permission — позже.")
                wrapMode: Text.Wrap
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }
        }

        VerticalScrollDecorator {}
    }
}
