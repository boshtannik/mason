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
                width: parent.width - 2 * Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                truncationMode: TruncationMode.Fade
                text: (modelData && modelData.title && modelData.title.length > 0)
                      ? modelData.title
                      : (modelData ? modelData.id : "")
                color: item.highlighted ? Theme.highlightColor : Theme.primaryColor
            }

            menu: ContextMenu {
                MenuItem {
                    text: qsTr("Session settings")
                    onClicked: appWindow.openSessionSettings(modelData.id)
                }
                MenuItem {
                    text: qsTr("Rename")
                    onClicked: appWindow.pageStack.push(renameDialog, {
                        sessionId: modelData.id,
                        currentTitle: modelData.title
                    })
                }
                MenuItem {
                    text: qsTr("Delete")
                    onClicked: {
                        var sid = modelData.id
                        // Полоса отсчёта — на самой строке, чтобы сразу можно было
                        // удалить несколько сессий подряд (RemorsePopup блокировал экран).
                        item.showRemorse(qsTr("Delete session"), function() {
                            appWindow.deleteSession(sid)
                        })
                    }
                }
            }

            onClicked: appWindow.openSession(modelData.id)
        }

        PullDownMenu {
            MenuItem {
                text: qsTr("Delete all sessions")
                enabled: list.count > 0
                onClicked: remorse.execute(qsTr("Delete all sessions?"), function() {
                    appWindow.deleteAllSessions()
                })
            }
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

    RemorsePopup {
        id: remorse
    }

    Component {
        id: renameDialog

        Dialog {
            id: dlg
            property string sessionId
            property string currentTitle

            canAccept: titleField.text.trim().length > 0
            onAccepted: appWindow.renameSession(sessionId, titleField.text.trim())
            onStatusChanged: {
                if (status === PageStatus.Active) {
                    titleField.forceActiveFocus()
                }
            }

            Column {
                width: parent.width

                DialogHeader {
                    acceptText: qsTr("Rename")
                }
                TextField {
                    id: titleField
                    width: parent.width
                    label: qsTr("Session name")
                    text: dlg.currentTitle
                    inputMethodHints: Qt.ImhNoPredictiveText
                    EnterKey.enabled: text.trim().length > 0
                    EnterKey.iconSource: "image://theme/icon-m-enter-accept"
                    EnterKey.onClicked: dlg.accept()
                }
            }
        }
    }
}
