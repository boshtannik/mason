import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: sessionsPage

    property var appWindow

    SilicaListView {
        id: list
        anchors.fill: parent
        model: appWindow.sessions

        header: PageHeader {
            title: qsTr("Sessions")
        }

        delegate: ListItem {
            id: item
            contentHeight: Theme.itemSizeSmall

            Label {
                x: Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                text: modelData
                color: item.highlighted ? Theme.highlightColor : Theme.primaryColor
            }

            onClicked: appWindow.openSession(modelData)
        }

        PullDownMenu {
            MenuItem {
                text: qsTr("New session")
                onClicked: appWindow.newSession()
            }
        }

        ViewPlaceholder {
            enabled: list.count === 0
            text: qsTr("No sessions yet")
            hintText: qsTr("Pull down to create a new session")
        }

        VerticalScrollDecorator {}
    }
}
