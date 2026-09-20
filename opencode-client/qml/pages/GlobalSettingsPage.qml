import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: globalSettingsPage

    property var appWindow

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

            SectionHeader { text: qsTr("Models") }

            ComboBox {
                label: qsTr("Recognition language")
                currentIndex: appWindow.voiceLangOptions.indexOf(appWindow.voiceLang)
                menu: ContextMenu {
                    Repeater {
                        model: appWindow.voiceLangOptions
                        MenuItem { text: modelData }
                    }
                }
                onCurrentIndexChanged: {
                    var o = appWindow.voiceLangOptions
                    if (currentIndex >= 0 && currentIndex < o.length
                        && o[currentIndex] !== appWindow.voiceLang)
                        appWindow.voiceCmd("voice_lang", o[currentIndex])
                }
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("STT model") + ": " + appWindow.sttModelName()
                    truncationMode: TruncationMode.Fade
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: globalSettingsPage.appWindow.pageStack.push(
                               Qt.resolvedUrl("VoiceModelsPage.qml"),
                               { appWindow: globalSettingsPage.appWindow })
            }

            BackgroundItem {
                width: parent.width
                contentHeight: Theme.itemSizeSmall
                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - 2 * Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("TTS voice") + ": " + appWindow.ttsModelName()
                    truncationMode: TruncationMode.Fade
                    color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                onClicked: globalSettingsPage.appWindow.pageStack.push(
                               Qt.resolvedUrl("VoiceModelsPage.qml"),
                               { appWindow: globalSettingsPage.appWindow })
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("Tap — download / select; long press — remove / cancel.")
                wrapMode: Text.Wrap
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
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
                               Qt.resolvedUrl("VoiceModelsPage.qml"),
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
                      .arg(appWindow.sttModelName())
                      .arg(appWindow.ttsModelName())
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

    // Пикер STT-моделей (whisper): тап — скачать / выбрать.
    Component {
        id: sttPickerComponent
        Page {
            id: sttPickerPage
            property var appWindow
            property var list: sttPickerPage.appWindow !== undefined
                               ? sttPickerPage.appWindow.voiceModelsFor("stt_whisper") : []

            function pct(m) {
                var size = parseInt(m.size)
                var done = parseInt(m.done || 0)
                if (!size) return ""
                return Math.min(99, Math.round(done / size * 100)) + "%"
            }
            function sub(m) {
                var s = m.lang_id + " · " + sttPickerPage.appWindow.humanSize(m.size)
                if (m.downloaded)
                    s += " ✓"
                if (m.state === "downloading")
                    s += " · " + sttPickerPage.pct(m)
                return s
            }
            function rightText(m) {
                if (m.state === "downloading")
                    return qsTr("cancel")
                if (m.model_id === sttPickerPage.appWindow.chosenStt)
                    return qsTr("selected")
                if (m.downloaded)
                    return qsTr("select")
                return qsTr("download")
            }
            function menuText(m) {
                return m.state === "downloading" ? qsTr("Cancel download") : qsTr("Remove model")
            }

            SilicaListView {
                anchors.fill: parent
                model: sttPickerPage.list
                header: PageHeader { title: qsTr("Recognition models") }
                delegate: ListItem {
                    contentHeight: Theme.itemSizeMedium
                    onClicked: {
                        if (modelData.state === "downloading") {
                            sttPickerPage.appWindow.voiceCmd(
                                "voice_download_cancel", modelData.model_id)
                        } else if (modelData.downloaded) {
                            sttPickerPage.appWindow.voiceCmd(
                                "voice_select_stt", modelData.model_id)
                            sttPickerPage.appWindow.pageStack.pop()
                        } else {
                            sttPickerPage.appWindow.voiceCmd(
                                "voice_download", modelData.model_id)
                        }
                    }
                    menu: ContextMenu {
                        MenuItem {
                            text: sttPickerPage.menuText(modelData)
                            onClicked: {
                                if (modelData.state === "downloading")
                                    sttPickerPage.appWindow.voiceCmd(
                                        "voice_download_cancel", modelData.model_id)
                                else
                                    sttPickerPage.appWindow.voiceCmd(
                                        "voice_delete", modelData.model_id)
                            }
                        }
                    }
                    Column {
                        x: Theme.horizontalPageMargin
                        width: parent.width - 2 * Theme.horizontalPageMargin
                                 - Theme.itemSizeSmall
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2
                        Label {
                            text: modelData.name
                            width: parent.width
                            truncationMode: TruncationMode.Fade
                            color: parent.parent.highlighted
                                   ? Theme.highlightColor : Theme.primaryColor
                        }
                        Label {
                            text: sttPickerPage.sub(modelData)
                            width: parent.width
                            truncationMode: TruncationMode.Fade
                            color: Theme.secondaryColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                        }
                    }
                    Label {
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        text: sttPickerPage.rightText(modelData)
                        color: Theme.highlightColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                }
                VerticalScrollDecorator {}
            }
        }
    }

    // Пикер TTS-голосов (piper): тап — скачать / выбрать.
    Component {
        id: ttsPickerComponent
        Page {
            id: ttsPickerPage
            property var appWindow
            property var list: ttsPickerPage.appWindow !== undefined
                               ? ttsPickerPage.appWindow.voiceModelsFor("tts_piper") : []

            function pct(m) {
                var size = parseInt(m.size)
                var done = parseInt(m.done || 0)
                if (!size) return ""
                return Math.min(99, Math.round(done / size * 100)) + "%"
            }
            function sub(m) {
                var s = m.lang_id + " · " + ttsPickerPage.appWindow.humanSize(m.size)
                if (m.downloaded)
                    s += " ✓"
                if (m.state === "downloading")
                    s += " · " + ttsPickerPage.pct(m)
                return s
            }
            function rightText(m) {
                if (m.state === "downloading")
                    return qsTr("cancel")
                if (m.model_id === ttsPickerPage.appWindow.chosenTts)
                    return qsTr("selected")
                if (m.downloaded)
                    return qsTr("select")
                return qsTr("download")
            }
            function menuText(m) {
                return m.state === "downloading" ? qsTr("Cancel download") : qsTr("Remove model")
            }

            SilicaListView {
                anchors.fill: parent
                model: ttsPickerPage.list
                header: PageHeader { title: qsTr("Voices (TTS)") }
                delegate: ListItem {
                    contentHeight: Theme.itemSizeMedium
                    onClicked: {
                        if (modelData.state === "downloading") {
                            ttsPickerPage.appWindow.voiceCmd(
                                "voice_download_cancel", modelData.model_id)
                        } else if (modelData.downloaded) {
                            ttsPickerPage.appWindow.voiceCmd(
                                "voice_select_tts", modelData.model_id)
                            ttsPickerPage.appWindow.pageStack.pop()
                        } else {
                            ttsPickerPage.appWindow.voiceCmd(
                                "voice_download", modelData.model_id)
                        }
                    }
                    menu: ContextMenu {
                        MenuItem {
                            text: ttsPickerPage.menuText(modelData)
                            onClicked: {
                                if (modelData.state === "downloading")
                                    ttsPickerPage.appWindow.voiceCmd(
                                        "voice_download_cancel", modelData.model_id)
                                else
                                    ttsPickerPage.appWindow.voiceCmd(
                                        "voice_delete", modelData.model_id)
                            }
                        }
                    }
                    Column {
                        x: Theme.horizontalPageMargin
                        width: parent.width - 2 * Theme.horizontalPageMargin
                                 - Theme.itemSizeSmall
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2
                        Label {
                            text: modelData.name
                            width: parent.width
                            truncationMode: TruncationMode.Fade
                            color: parent.parent.highlighted
                                   ? Theme.highlightColor : Theme.primaryColor
                        }
                        Label {
                            text: ttsPickerPage.sub(modelData)
                            width: parent.width
                            truncationMode: TruncationMode.Fade
                            color: Theme.secondaryColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                        }
                    }
                    Label {
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        text: ttsPickerPage.rightText(modelData)
                        color: Theme.highlightColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                    }
                }
                VerticalScrollDecorator {}
            }
        }
    }
}
