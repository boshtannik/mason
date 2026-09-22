import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: sessionSettingsPage

    property var appWindow

    function currentSession() {
        var id = appWindow.currentSessionId
        var list = appWindow.sessions || []
        for (var i = 0; i < list.length; i++) {
            if (list[i].id === id)
                return list[i]
        }
        return null
    }

    property var session: currentSession()
    property string sessionId: session ? session.id : appWindow.currentSessionId
    property string shareUrl: (session && session.shareUrl) ? session.shareUrl : ""
    property string modelLabel: (session && session.modelID)
                                ? (session.providerID + " / " + session.modelID)
                                : qsTr("no data")

    // Защита от двойного тапа (действие выполняется ~секунду).
    property bool actionBusy: false
    Timer {
        id: actionGuard
        interval: 1200
        onTriggered: sessionSettingsPage.actionBusy = false
    }

    function runAction(fn) {
        if (actionBusy)
            return
        actionBusy = true
        actionGuard.restart()
        fn()
    }

    function forkSession() {
        if (actionBusy)
            return
        actionBusy = true
        appWindow.forkSession(sessionId)
        actionGuard.restart()
    }

    function todoMark(status) {
        if (status === "completed")
            return "✓"
        if (status === "in_progress")
            return "→"
        if (status === "cancelled")
            return "✗"
        return "☐"
    }

    function todoColor(status) {
        if (status === "completed")
            return Theme.secondaryColor
        if (status === "in_progress")
            return Theme.highlightColor
        if (status === "cancelled")
            return Theme.secondaryColor
        return Theme.primaryColor
    }

    function tokensLabel() {
        if (!session || !session.tokens)
            return "—"
        var t = session.tokens
        return "in " + (t.input || 0) + " / out " + (t.output || 0)
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: pageHeader.height + column.height + Theme.paddingLarge

        PageHeader {
            id: pageHeader
            title: (session && session.title && session.title.length > 0)
                   ? session.title
                   : qsTr("Session")
        }

        Column {
            id: column
            anchors {
                top: pageHeader.bottom
                left: parent.left
                right: parent.right
            }

            SectionHeader { text: qsTr("Session") }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Fork (branch)")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: sessionSettingsPage.forkSession()
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: sessionSettingsPage.shareUrl.length > 0
                          ? qsTr("Refresh link")
                          : qsTr("Share")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: sessionSettingsPage.runAction(function() {
                    appWindow.shareSession(sessionSettingsPage.sessionId)
                })
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                visible: sessionSettingsPage.shareUrl.length > 0
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Open link")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: Qt.openUrlExternally(sessionSettingsPage.shareUrl)
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                visible: sessionSettingsPage.shareUrl.length > 0
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Copy link")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: Clipboard.text = sessionSettingsPage.shareUrl
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                visible: sessionSettingsPage.shareUrl.length > 0
                text: sessionSettingsPage.shareUrl
                wrapMode: Text.Wrap
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Session summary")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: appWindow.pageStack.push(summaryDialog, {
                    title: sessionSettingsPage.session && sessionSettingsPage.session.title
                           ? sessionSettingsPage.session.title : "",
                    sid: sessionSettingsPage.sessionId,
                    model: sessionSettingsPage.modelLabel,
                    share: sessionSettingsPage.shareUrl,
                    cost: sessionSettingsPage.session ? (sessionSettingsPage.session.cost || 0) : 0,
                    tokens: sessionSettingsPage.tokensLabel()
                })
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Switch model")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: appWindow.pageStack.push(modelPicker, {
                    sessionId: sessionSettingsPage.sessionId,
                    appWindow: sessionSettingsPage.appWindow,
                    currentProvider: sessionSettingsPage.session
                                     ? sessionSettingsPage.session.providerID : "",
                    currentModel: sessionSettingsPage.session
                                  ? sessionSettingsPage.session.modelID : ""
                })
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Compact (compress history)")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: {
                    var sid = sessionSettingsPage.sessionId
                    remorse.execute(qsTr("Compress history?"), function() {
                        sessionSettingsPage.runAction(function() {
                            appWindow.summarizeSession(sid)
                        })
                    })
                }
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                visible: appWindow.statusText === "busy"
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Abort")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: appWindow.abortSession(sessionSettingsPage.sessionId)
            }

            SectionHeader { text: qsTr("TODO tasks") }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                visible: appWindow.todos.length === 0
                text: qsTr("No tasks")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
            }

            Repeater {
                model: appWindow.todos

                Item {
                    width: column.width
                    height: Math.max(mark.height, taskLabel.height) + Theme.paddingMedium

                    Label {
                        id: mark
                        x: Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        text: sessionSettingsPage.todoMark(modelData.status)
                        color: sessionSettingsPage.todoColor(modelData.status)
                        font.pixelSize: Theme.fontSizeMedium
                    }

                    Label {
                        id: taskLabel
                        x: mark.x + Theme.itemSizeExtraSmall
                        width: parent.width - x - Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.content
                        wrapMode: Text.Wrap
                        color: sessionSettingsPage.todoColor(modelData.status)
                        font.pixelSize: Theme.fontSizeSmall
                    }
                }
            }

            SectionHeader { text: qsTr("Misc") }

            TextSwitch {
                text: qsTr("Show tools strip")
                checked: appWindow.showTools
                onCheckedChanged: appWindow.showTools = checked
            }
        }

        VerticalScrollDecorator {}
    }

    RemorsePopup {
        id: remorse
    }

    Component {
        id: summaryDialog

        Dialog {
            id: dlg
            property string title
            property string sid
            property string model
            property string share
            property real cost
            property string tokens

            Column {
                width: parent.width

                DialogHeader {
                    acceptText: qsTr("OK")
                    cancelText: ""
                }

                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    wrapMode: Text.Wrap
                    text: dlg.title.length > 0 ? dlg.title : qsTr("(untitled)")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeMedium
                }
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    wrapMode: Text.Wrap
                    text: dlg.sid
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                }
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    wrapMode: Text.Wrap
                    text: qsTr("Model: ") + dlg.model
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeSmall
                }
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    wrapMode: Text.Wrap
                    text: qsTr("Cost: $") + dlg.cost.toFixed(4)
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeSmall
                }
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    wrapMode: Text.Wrap
                    text: qsTr("Tokens: ") + dlg.tokens
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeSmall
                }
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    visible: dlg.share.length > 0
                    wrapMode: Text.Wrap
                    text: qsTr("Link: ") + dlg.share
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                }
            }
        }
    }

    Component {
        id: modelPicker

        Page {
            id: pickerPage
            property string sessionId
            property var appWindow
            property string currentProvider
            property string currentModel

            SilicaListView {
                id: modelList
                anchors.fill: parent
                model: pickerPage.appWindow.models

                header: PageHeader {
                    title: qsTr("Model")
                }

                delegate: ListItem {
                    id: modelItem
                    contentHeight: Theme.itemSizeSmall

                    Label {
                        x: Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        text: (modelData.providerID === pickerPage.currentProvider
                               && modelData.modelID === pickerPage.currentModel)
                              ? "✓" : ""
                        color: Theme.highlightColor
                    }

                    Column {
                        x: Theme.itemSizeSmall
                        width: parent.width - x - Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter

                        Label {
                            width: parent.width
                            truncationMode: TruncationMode.Fade
                            text: modelData.modelName
                            color: modelItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                        }
                        Label {
                            width: parent.width
                            truncationMode: TruncationMode.Fade
                            text: modelData.providerName
                            color: Theme.secondaryColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                        }
                    }

                    onClicked: {
                        pickerPage.appWindow.setModel(pickerPage.sessionId,
                                                      modelData.providerID,
                                                      modelData.modelID)
                        pageStack.pop()
                    }
                }

                ViewPlaceholder {
                    enabled: modelList.count === 0
                    text: qsTr("No available models")
                }

                VerticalScrollDecorator {}
            }
        }
    }
}
