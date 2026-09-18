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
                text: qsTr("Finish sound")
                checked: appWindow.soundOnFinish
                onCheckedChanged: appWindow.soundOnFinish = checked
            }

            TextSwitch {
                text: qsTr("Permission sound")
                checked: appWindow.soundOnPermission
                onCheckedChanged: appWindow.soundOnPermission = checked
            }

            TextSwitch {
                text: qsTr("Send immediately")
                checked: appWindow.sendImmediately
                onCheckedChanged: appWindow.sendImmediately = checked
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
}
