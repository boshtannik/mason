import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: globalSettingsPage

    property var appWindow

    // Синхронизация дропдауна позиции PTT. Биндинг currentIndex ломается
    // при ручном выборе пункта меню, поэтому индекс ставим явно.
    readonly property var pttPositions: ["left", "center", "right"]
    function syncPttCombo() {
        var v = pttPosCombo.available
        var idx = v.indexOf(appWindow.pttPosition)
        if (idx < 0)
            idx = 0
        pttPosCombo.currentIndex = idx
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: pageHeader.height + column.height + Theme.paddingLarge

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
                    if (currentIndex >= 0 && currentIndex < v.length) {
                        appWindow.inputMode = v[currentIndex]
                        // Для text_ptt кнопка диктовки может быть только слева
                        // или справа от инпута — центр недопустим.
                        if (v[currentIndex] === "text_ptt"
                            && appWindow.pttPosition === "center")
                            appWindow.pttPosition = "left"
                        // Дропдаун позиции PTT пересчитывает доступные пункты.
                        globalSettingsPage.syncPttCombo()
                    }
                }
            }

            ComboBox {
                id: pttPosCombo
                label: qsTr("PTT button position")
                // Центр доступен только в режиме «Только диктовка».
                property var available: appWindow.inputMode === "voice"
                                        ? globalSettingsPage.pttPositions
                                        : ["left", "right"]
                currentIndex: 0
                menu: ContextMenu {
                    MenuItem { text: qsTr("Left") }
                    MenuItem {
                        text: qsTr("Center")
                        // В text_ptt центр недопустим — прячем пункт.
                        visible: appWindow.inputMode === "voice"
                    }
                    MenuItem { text: qsTr("Right") }
                }
                onCurrentIndexChanged: {
                    var v = pttPosCombo.available
                    if (currentIndex >= 0 && currentIndex < v.length)
                        appWindow.pttPosition = v[currentIndex]
                }
                Component.onCompleted: globalSettingsPage.syncPttCombo()
            }

            SectionHeader { text: qsTr("Voice") }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeMedium
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Voice models")
                    truncationMode: TruncationMode.Fade
                    color: parent.highlighted
                           ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: globalSettingsPage.appWindow.pageStack.push(
                               voiceModelsComponent,
                               { appWindow: globalSettingsPage.appWindow })
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("Recognition:\n%1\nSynthesiser:\n%2")
                      .arg(appWindow.sttModelName())
                      .arg(appWindow.ttsModelName())
                wrapMode: Text.Wrap
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("↓ download\n→ start using\n✓ selected\n✕ cancel\nlong press — remove")
                wrapMode: Text.Wrap
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }

            // — Диктовка: отдельная секция про отправку распознанного текста.
            SectionHeader { text: qsTr("Dictation") }

            TextSwitch {
                text: qsTr("Send immediately")
                checked: appWindow.sendImmediately
                onCheckedChanged: appWindow.sendImmediately = checked
            }

            // — Автоозвучка ответов - отдельная секция. Доступна только когда
            // скачана и выбрана модель синтеза речи; иначе — подсказка.
            SectionHeader { text: qsTr("Voice answers") }

            Label {
                visible: !appWindow.ttsModelReady
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("Download and select a speech synthesis model to enable\nvoice answers (Settings → Voice models).")
                wrapMode: Text.Wrap
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }

            ComboBox {
                label: qsTr("Free hands / autoplay")
                enabled: appWindow.ttsModelReady
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

            SectionHeader { text: qsTr("General") }

            TextField {
                id: workdirField
                width: parent.width
                text: (typeof appWindow.workdir !== "undefined") ? appWindow.workdir : ""
                placeholderText: qsTr("e.g. /home/defaultuser/mason")
                label: qsTr("Working directory (restart required)")
                onActiveFocusChanged: {
                    if (!activeFocus && typeof appWindow.bridge !== "undefined" && typeof appWindow.workdir !== "undefined") {
                        appWindow.workdir = text
                        appWindow.bridge.set_workdir(text)
                    }
                }
                EnterKey.enabled: text.length > 0
                EnterKey.iconSource: "image://theme/icon-m-enter-accept"
                EnterKey.onClicked: {
                    if (typeof appWindow.bridge !== "undefined" && typeof appWindow.workdir !== "undefined") {
                        appWindow.workdir = text
                        appWindow.bridge.set_workdir(text)
                    }
                    focus = false
                }
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("The agent creates and edits files here and runs commands from this directory.")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                wrapMode: Text.Wrap
            }

            Button {
                text: qsTr("Browse…")
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                onClicked: {
                    var start = workdirField.text.length > 0
                                ? workdirField.text
                                : (typeof appWindow.workdir !== "undefined" ? appWindow.workdir : "")
                    globalSettingsPage.appWindow.pageStack.push(
                        dirPickerComponent,
                        { appWindow: globalSettingsPage.appWindow,
                          startPath: start,
                          targetField: workdirField })
                }
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("Changes take effect after restarting the app")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                wrapMode: Text.Wrap
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
                                                         qsTr("Test notification"))
            }

            // Язык интерфейса — системный; переводы подгружаются нативно (.qm).
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
                header: PageHeader { title: qsTr("Notification sound") }
                delegate: ListItem {
                    contentHeight: Theme.itemSizeSmall
                    onClicked: {
                        soundPickerPage.appWindow.previewDing(modelData.path)
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

    Component {
        id: voiceModelsComponent
        VoiceModelsPage { }
    }

    Component {
        id: dirPickerComponent
        DirPickerPage { }
    }
}
