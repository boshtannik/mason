import QtQuick 2.6
import Sailfish.Silica 1.0

// Управление голосовыми моделями: две сворачиваемые секции
// (распознавание / синтез), фильтр по языку, живой прогресс,
// действия-иконки и удаление с подтверждением (RemorseItem).
Page {
    id: vmPage

    property var appWindow
    property string filterLang: appWindow !== undefined ? appWindow.voiceLang : "auto"
    property bool recExpanded: true
    property bool ttsExpanded: true

    // Фингерпринт структурного состояния (БЕЗ счётчика прогресса):
    // список пересобирается только когда меняется выбор/состояние/фильтр/секция,
    // а не каждые 300 мс polling-а — скролл не сбрасывается во время скачивания.
    property string voiceFp: appWindow !== undefined ? vmPage.fingerprint() : ""

    onVoiceFpChanged: vmPage.fillModel()

    ListModel { id: voiceModel }

    function fingerprint() {
        var arr = appWindow.voiceModels || []
        var out = ""
        for (var i = 0; i < arr.length; i++)
            out += arr[i].model_id + "|" + arr[i].downloaded + "|"
                   + arr[i].state + ";"
        return out + "@" + appWindow.chosenStt + "#" + appWindow.chosenTts
               + "$" + vmPage.filterLang + "%" + vmPage.recExpanded
               + "!" + vmPage.ttsExpanded
    }

    function langName(id) {
        if (id === "multilang" || id === "multilingual")
            return qsTr("Multilingual")
        var o = appWindow.voiceLangOptions || []
        for (var i = 0; i < o.length; i++)
            if (o[i].id === id)
                return o[i].name || id
        return id
    }

    function filterIndex() {
        var o = appWindow.voiceLangOptions || []
        for (var i = 0; i < o.length; i++)
            if (o[i].id === vmPage.filterLang)
                return i
        return 0
    }

    function visible(m) {
        if (vmPage.filterLang === "" || vmPage.filterLang === "auto")
            return true
        if (m.lang_id === "multilang" || m.lang_id === "multilingual")
            return true
        return m.lang_id === vmPage.filterLang
    }

    function isSelected(m) {
        return m.model_id === appWindow.chosenStt
               || m.model_id === appWindow.chosenTts
    }

    function rowSub(m) {
        var s = vmPage.langName(m.lang_id) + " · " + appWindow.humanSize(m.size)
        if (m.downloaded)
            s += " · " + qsTr("downloaded")
        return s
    }

    function glyph(m) {
        if (m.state === "downloading")
            return "✕"
        if (vmPage.isSelected(m))
            return "✓"
        if (m.downloaded)
            return "→"
        return "↓"
    }

    function glyphColor(m) {
        if (m.state === "downloading")
            return Theme.secondaryColor
        if (vmPage.isSelected(m))
            return Theme.highlightColor
        return Theme.primaryColor
    }

    function findModel(id) {
        var arr = appWindow.voiceModels || []
        for (var i = 0; i < arr.length; i++)
            if (arr[i].model_id === id)
                return arr[i]
        return null
    }

    function livePct(id) {
        var m = vmPage.findModel(id)
        if (!m || m.state !== "downloading")
            return ""
        var size = parseInt(m.size), done = parseInt(m.done || 0)
        if (!size || !done)
            return ""
        return Math.min(99, Math.round(done / size * 100)) + "%"
    }

    function liveRatio(id) {
        var m = vmPage.findModel(id)
        if (!m || m.state !== "downloading")
            return 0
        var size = parseInt(m.size), done = parseInt(m.done || 0)
        if (!size)
            return 0
        return Math.max(0, Math.min(1, done / size))
    }

    function sectionMeta(arr, engine) {
        var total = 0, done = 0
        for (var i = 0; i < arr.length; i++) {
            if (arr[i].engine !== engine)
                continue
            total++
            if (arr[i].downloaded)
                done++
        }
        return { total: total, done: done }
    }

    function fillModel() {
        voiceModel.clear()
        var arr = appWindow.voiceModels || []

        var rec = vmPage.sectionMeta(arr, "stt_whisper")
        voiceModel.append({ isHeader: true, kind: "recognition",
                            title: qsTr("Speech recognition models"),
                            total: rec.total, done: rec.done,
                            name: "", sub: "", state: "", modelId: "",
                            engine: "", downloaded: false })
        if (vmPage.recExpanded)
            vmPage.appendModels(arr, "stt_whisper")

        var tts = vmPage.sectionMeta(arr, "tts_piper")
        voiceModel.append({ isHeader: true, kind: "synthesis",
                            title: qsTr("Speech synthesiser models"),
                            total: tts.total, done: tts.done,
                            name: "", sub: "", state: "", modelId: "",
                            engine: "", downloaded: false })
        if (vmPage.ttsExpanded)
            vmPage.appendModels(arr, "tts_piper")
    }

    function appendModels(arr, engine) {
        for (var i = 0; i < arr.length; i++) {
            var m = arr[i]
            if (m.engine !== engine)
                continue
            var chosen = vmPage.isSelected(m)
            if (!chosen && !vmPage.visible(m))
                continue
            voiceModel.append({ isHeader: false, kind: engine,
                                title: "", total: 0, done: 0,
                                name: m.name,
                                sub: vmPage.rowSub(m),
                                state: m.state,
                                modelId: m.model_id,
                                engine: engine,
                                downloaded: !!m.downloaded })
        }
    }

    function onRowTap(isHeader, kind, modelId, engine, state, downloaded) {
        if (isHeader) {
            if (kind === "recognition")
                vmPage.recExpanded = !vmPage.recExpanded
            else
                vmPage.ttsExpanded = !vmPage.ttsExpanded
            return
        }
        if (state === "downloading") {
            appWindow.voiceCmd("voice_download_cancel", modelId)
        } else if (downloaded) {
            appWindow.voiceCmd(engine === "stt_whisper"
                               ? "voice_select_stt" : "voice_select_tts", modelId)
        } else {
            appWindow.voiceCmd("voice_download", modelId)
        }
    }

    function onRowMenu(isHeader, kind, modelId, state, name) {
        if (isHeader) {
            if (kind === "recognition")
                vmPage.recExpanded = !vmPage.recExpanded
            else
                vmPage.ttsExpanded = !vmPage.ttsExpanded
            return
        }
        if (state === "downloading") {
            appWindow.voiceCmd("voice_download_cancel", modelId)
        } else {
            remorse.execute(qsTr("Removing «%1»…").arg(name),
                            function() {
                appWindow.voiceCmd("voice_delete", modelId)
            })
        }
    }

    function downloadedCount() {
        var arr = appWindow.voiceModels || []
        var c = 0
        for (var i = 0; i < arr.length; i++)
            if (arr[i].downloaded)
                c++
        return c
    }

    function downloadedBytes() {
        var arr = appWindow.voiceModels || []
        var b = 0
        for (var i = 0; i < arr.length; i++)
            if (arr[i].downloaded)
                b += parseInt(arr[i].size || 0)
        return b
    }

    RemorseItem { id: remorse }

    SilicaListView {
        id: list
        anchors.fill: parent
        model: voiceModel

        header: Item {
            width: parent.width
            height: filterCombo.height + summaryLabel.height + Theme.paddingMedium

            ComboBox {
                id: filterCombo
                width: parent.width
                label: qsTr("Language")
                currentIndex: vmPage.filterIndex()
                menu: ContextMenu {
                    Repeater {
                        model: appWindow.voiceLangOptions
                        MenuItem { text: modelData.name }
                    }
                }
                onCurrentIndexChanged: {
                    var o = appWindow.voiceLangOptions
                    if (currentIndex < 0 || currentIndex >= o.length)
                        return
                    var id = o[currentIndex].id
                    if (id === vmPage.filterLang)
                        return
                    vmPage.filterLang = id
                    if (id !== appWindow.voiceLang)
                        appWindow.voiceCmd("voice_lang", id)
                }
            }

            Label {
                id: summaryLabel
                anchors.top: filterCombo.bottom
                anchors.topMargin: Theme.paddingSmall
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("On device: %1 of %2 models, %3")
                      .arg(vmPage.downloadedCount())
                      .arg((appWindow.voiceModels || []).length)
                      .arg(appWindow.humanSize(vmPage.downloadedBytes()))
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                wrapMode: Text.Wrap
            }
        }

        delegate: ListItem {
            id: item
            width: parent.width
            contentHeight: isHeader
                           ? Theme.itemSizeMedium
                           : Math.max(Theme.itemSizeSmall,
                                      textCol.height
                                      + (state === "downloading"
                                         ? Theme.paddingSmall + 6 : 0)
                                      + Theme.paddingMedium)

            onClicked: vmPage.onRowTap(isHeader, kind, modelId,
                                       engine, state, downloaded)

            Label {
                id: headerArrow
                visible: isHeader
                width: Theme.itemSizeSmall
                x: Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                text: kind === "recognition"
                      ? (vmPage.recExpanded ? "▾" : "▸")
                      : (vmPage.ttsExpanded ? "▾" : "▸")
                color: parent.highlighted ? Theme.highlightColor : Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
            }

            Label {
                id: headerTitle
                visible: isHeader
                x: Theme.horizontalPageMargin + Theme.itemSizeSmall + Theme.paddingSmall
                width: parent.width - x - Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                text: title + " (" + done + "/" + total + ")"
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                font.pixelSize: Theme.fontSizeSmall
                truncationMode: TruncationMode.Fade
            }

            Column {
                id: textCol
                visible: !isHeader
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                         - Theme.itemSizeMedium - Theme.paddingMedium
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Label {
                    width: parent.width
                    text: name
                    wrapMode: Text.Wrap
                    color: vmPage.isSelected({ model_id: modelId, downloaded: downloaded })
                           ? Theme.highlightColor
                           : (item.highlighted ? Theme.highlightColor : Theme.primaryColor)
                    font.pixelSize: Theme.fontSizeSmall
                }
                Label {
                    width: parent.width
                    text: sub
                    wrapMode: Text.Wrap
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                }
                Label {
                    visible: state === "downloading"
                    width: parent.width
                    text: vmPage.livePct(modelId)
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                }
            }

            Label {
                id: iconLbl
                visible: !isHeader
                anchors.right: parent.right
                anchors.rightMargin: Theme.paddingSmall
                anchors.verticalCenter: parent.verticalCenter
                width: Theme.itemSizeMedium
                horizontalAlignment: Text.AlignHCenter
                text: vmPage.glyph({ state: state,
                                    model_id: modelId,
                                    downloaded: downloaded })
                color: vmPage.glyphColor({ state: state,
                                          model_id: modelId,
                                          downloaded: downloaded })
                font.pixelSize: Theme.fontSizeSmall
            }

            Rectangle {
                id: bar
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 3
                visible: !isHeader && state === "downloading"
                color: Theme.highlightBackgroundColor
                Rectangle {
                    x: 0
                    y: 0
                    width: parent.width * vmPage.liveRatio(modelId)
                    height: parent.height
                    color: Theme.highlightColor
                }
            }

            menu: ContextMenu {
                MenuItem {
                    text: isHeader
                          ? qsTr("Expand / collapse")
                          : (state === "downloading"
                             ? qsTr("Cancel download") : qsTr("Remove model"))
                    onClicked: vmPage.onRowMenu(isHeader, kind, modelId,
                                                state, name)
                }
            }
        }

        ViewPlaceholder {
            enabled: list.count === 0
            text: qsTr("Нет моделей для выбранного языка")
        }

        VerticalScrollDecorator {}
    }
}