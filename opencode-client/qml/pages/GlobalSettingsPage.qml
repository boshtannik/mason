import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: globalSettingsPage

    property var appWindow

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        PageHeader {
            id: pageHeader
            title: qsTr("Settings")
        }

        Column {
            id: column
            anchors {
                top: pageHeader.bottom
                left: parent.left
                right: parent.right
            }

            SectionHeader { text: qsTr("Input") }

            ComboBox {
                label: qsTr("Mode")
                currentIndex: ["text", "text_ptt", "voice"].indexOf(appWindow.inputMode)
                menu: ContextMenu {
                    MenuItem { text: qsTr("Text only") }
                    MenuItem { text: qsTr("Text + PTT") }
                    MenuItem { text: qsTr("Voice") }
                }
                onCurrentIndexChanged: {
                    var v = ["text", "text_ptt", "voice"]
                    if (currentIndex >= 0 && currentIndex < v.length)
                        appWindow.inputMode = v[currentIndex]
                }
            }

            ComboBox {
                label: qsTr("PTT button position")
                currentIndex: ["left", "right"].indexOf(appWindow.pttPosition)
                menu: ContextMenu {
                    MenuItem { text: qsTr("Left") }
                    MenuItem { text: qsTr("Right") }
                }
                onCurrentIndexChanged: {
                    var v = ["left", "right"]
                    if (currentIndex >= 0 && currentIndex < v.length)
                        appWindow.pttPosition = v[currentIndex]
                }
            }

            SectionHeader { text: qsTr("Speakers") }

            ComboBox {
                label: qsTr("Free hands / autoplay")
                currentIndex: ["auto", "button", "off"].indexOf(appWindow.ttsMode)
                menu: ContextMenu {
                    MenuItem { text: qsTr("Auto") }
                    MenuItem { text: qsTr("Button") }
                    MenuItem { text: qsTr("Off") }
                }
                onCurrentIndexChanged: {
                    var v = ["auto", "button", "off"]
                    if (currentIndex >= 0 && currentIndex < v.length)
                        appWindow.ttsMode = v[currentIndex]
                }
            }

            TextSwitch {
                text: qsTr("Send immediately")
                checked: appWindow.sendImmediately
                onCheckedChanged: appWindow.sendImmediately = checked
            }

            SectionHeader { text: qsTr("Notifications") }

            TextSwitch {
                text: qsTr("Sound when agent finishes (ding)")
                checked: appWindow.soundOnFinish
                onCheckedChanged: appWindow.soundOnFinish = checked
            }

            TextSwitch {
                text: qsTr("Sound on permission request")
                checked: appWindow.soundOnPermission
                onCheckedChanged: appWindow.soundOnPermission = checked
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Sound") + ": " + globalSettingsPage.appWindow.dingSoundName()
                    truncationMode: TruncationMode.Fade
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: globalSettingsPage.appWindow.pageStack.push(
                               soundPickerComponent,
                               { appWindow: globalSettingsPage.appWindow })
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Test notification")
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: appWindow.publishNotification(qsTr("opencode"),
                                                         qsTr("Тестовое уведомление"))
            }

            SectionHeader { text: qsTr("Interface") }

            ComboBox {
                label: qsTr("Language")
                currentIndex: ["en", "ru"].indexOf(appWindow.uiLanguage)
                menu: ContextMenu {
                    MenuItem { text: qsTr("English") }
                    MenuItem { text: qsTr("Русский") }
                }
                onCurrentIndexChanged: {
                    var v = ["en", "ru"]
                    if (currentIndex >= 0 && currentIndex < v.length)
                        appWindow.uiLanguage = v[currentIndex]
                }
            }

            SectionHeader { text: qsTr("Models") }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("STT: %1\nTTS: %2")
                      .arg(appWindow.sttModelReady ? qsTr("ready") : qsTr("not loaded"))
                      .arg(appWindow.ttsModelReady ? qsTr("ready") : qsTr("not loaded"))
                wrapMode: Text.Wrap
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }
        }

        VerticalScrollDecorator {}
    }

    Component {
        id: soundPickerComponent
        Page {
            id: soundPickerPage
            property var appWindow

            SilicaListView {
                anchors.fill: parent
                model: soundPickerPage.appWindow !== undefined
                       ? soundPickerPage.appWindow.sounds
                       : []
                header: PageHeader { title: qsTr("Звук уведомления") }
                delegate: ListItem {
                    contentHeight: Theme.itemSizeSmall
                    onClicked: {
                        soundPickerPage.appWindow.setDingSound(modelData.path)
                        soundPickerPage.appWindow.playDing()
                    }
                    Label {
                        x: Theme.horizontalPageMargin
                        width: parent.width - 2 * Theme.horizontalPageMargin
                                 - Theme.itemSizeSmall
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.name
                        truncationMode: TruncationMode.Fade
                        color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                    }
                    Label {
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.path === soundPickerPage.appWindow.dingSound ? "✓" : ""
                        color: Theme.highlightColor
                    }
                }
                VerticalScrollDecorator {}
            }
        }
    }
}
