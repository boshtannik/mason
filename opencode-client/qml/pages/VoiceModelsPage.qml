import QtQuick 2.6
import Sailfish.Silica 1.0

// Управление голосовыми моделями: две сворачиваемые секции
// (распознавание / синтез), фильтр по языку, живой прогресс,
// действия-иконки и удаление с подтверждением (RemorseItem).
Page {
    id: vmPage

    property var appWindow
    property string filterLang: "auto"
    property bool dlExpanded: true
    property bool recExpanded: true
    property bool ttsExpanded: true

    // Фингерпринт структурного состояния (БЕЗ счётчика прогресса):
    // список пересобирается только когда меняется выбор/состояние/фильтр/секция,
    // а не каждые 300 мс polling-а — скролл не сбрасывается во время скачивания.
    property string voiceFp: appWindow !== undefined ? vmPage.fingerprint() : ""

    // Идентификаторы секций (локальные для страницы).
    readonly property string secOndevice: "ondevice"
    readonly property string secRecognition: "recognition"
    readonly property string secSynthesis: "synthesis"

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
               + "!" + vmPage.ttsExpanded + "^" + vmPage.dlExpanded
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

    // Предикат строк секций Recognition/Synthesis (единый для списка и счётчика).
    // В режиме «Auto» секция предлагает только ещё не скачанные модели;
    // при выборе конкретного языка — ВСЕ модели этого языка (скачанные тоже),
    // чтобы язык не «исчезал» из раздела и модели можно было выбрать/удалить.
    function storeRows(m, engine) {
        if (m.engine !== engine || !vmPage.visible(m))
            return false
        if (vmPage.filterLang === "" || vmPage.filterLang === "auto")
            return !m.downloaded
        return true
    }

    function isSelected(m) {
        return m.model_id === appWindow.chosenStt
               || m.model_id === appWindow.chosenTts
    }

    function rowSub(m) {
        var s = vmPage.langName(m.lang_id) + " · " + appWindow.humanSize(m.size)
        if (m.state === appWindow.stateError)
            s += " · " + qsTr("download failed")
        else if (m.downloaded)
            s += " · " + qsTr("downloaded")
        return s
    }

    function glyph(m) {
        if (m.state === appWindow.stateDownloading)
            return "✕"
        if (m.state === appWindow.stateError)
            return "!"
        if (vmPage.isSelected(m))
            return "✓"
        if (m.downloaded)
            return "→"
        return "↓"
    }

    function glyphColor(m) {
        if (m.state === appWindow.stateDownloading || m.state === appWindow.stateError)
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

    function liveInfo(id) {
        var m = vmPage.findModel(id)
        if (!m || m.state !== appWindow.stateDownloading)
            return ""
        var size = parseInt(m.size), done = parseInt(m.done || 0)
        if (!size)
            return ""
        var pct = Math.min(99, Math.round(done / size * 100))
        return qsTr("Downloading… %1% (%2 / %3)")
               .arg(pct)
               .arg(appWindow.humanSize(done))
               .arg(appWindow.humanSize(size))
    }

    function liveRatio(id) {
        var m = vmPage.findModel(id)
        if (!m || m.state !== appWindow.stateDownloading)
            return 0
        var size = parseInt(m.size), done = parseInt(m.done || 0)
        if (!size)
            return 0
        return Math.max(0, Math.min(1, done / size))
    }

    function fillModel() {
        voiceModel.clear()
        var arr = appWindow.voiceModels || []

        // 1. Скачанные модели (обе движка) — всегда без языкового фильтра.
        var ondev = vmPage.downloadedArr(arr)
        voiceModel.append({ isHeader: true, kind: vmPage.secOndevice,
                            title: qsTr("Downloaded models"),
                            total: ondev.length, done: ondev.length,
                            name: "", sub: "", dlstate: "", modelId: "",
                            engine: "", downloaded: false })
        if (vmPage.dlExpanded)
            for (var i = 0; i < ondev.length; i++)
                vmPage.appendRow(ondev[i])

        // 2. Распознавание, 3. Синтез — только нескачанные, с фильтром по языку.
        vmPage.appendStore(arr, appWindow.engineStt, qsTr("Recognition"),
                           vmPage.secRecognition, vmPage.recExpanded)
        vmPage.appendStore(arr, appWindow.engineTts, qsTr("Synthesis"),
                           vmPage.secSynthesis, vmPage.ttsExpanded)
    }

    function downloadedArr(arr) {
        var out = []
        for (var i = 0; i < arr.length; i++) {
            if (!arr[i].downloaded)
                continue
            var d = {}
            for (var k in arr[i])
                d[k] = arr[i][k]
            out.push(d)
        }
        return out
    }

    function appendStore(arr, engine, title, kind, expanded) {
        var n = 0
        for (var i = 0; i < arr.length; i++)
            if (vmPage.storeRows(arr[i], engine))
                n++
        voiceModel.append({ isHeader: true, kind: kind,
                            title: title, total: n, done: n,
                            name: "", sub: "", dlstate: "", modelId: "",
                            engine: "", downloaded: false })
        if (expanded)
            for (var j = 0; j < arr.length; j++) {
                var mm = arr[j]
                if (vmPage.storeRows(mm, engine))
                    vmPage.appendRow(mm)
            }
    }

    function appendRow(m) {
        voiceModel.append({ isHeader: false, kind: m.engine,
                            title: "", total: 0, done: 0,
                            name: m.name, sub: vmPage.rowSub(m),
                            dlstate: m.state, modelId: m.model_id,
                            engine: m.engine, downloaded: !!m.downloaded })
    }

    function toggleSection(kind) {
        if (kind === vmPage.secRecognition)
            vmPage.recExpanded = !vmPage.recExpanded
        else if (kind === vmPage.secSynthesis)
            vmPage.ttsExpanded = !vmPage.ttsExpanded
        else
            vmPage.dlExpanded = !vmPage.dlExpanded
    }

    function onRowTap(isHeader, kind, modelId, engine, dlstate, downloaded) {
        if (isHeader) {
            vmPage.toggleSection(kind)
            return
        }
        if (dlstate === appWindow.stateDownloading) {
            appWindow.voiceCmd(appWindow.cmdDownloadCancel, modelId)
        } else if (downloaded) {
            appWindow.voiceCmd(engine === appWindow.engineStt
                               ? appWindow.cmdSelectStt : appWindow.cmdSelectTts,
                               modelId)
        } else {
            appWindow.voiceCmd(appWindow.cmdDownload, modelId)
        }
    }

    function onRowMenu(isHeader, kind, modelId, dlstate, downloaded, name, wdg) {
        if (isHeader) {
            vmPage.toggleSection(kind)
            return
        }
        if (dlstate === appWindow.stateDownloading) {
            appWindow.voiceCmd(appWindow.cmdDownloadCancel, modelId)
        } else if (downloaded) {
            // RemorseItem.execute(item, text, callback) — первый аргумент
            // владелец, «Removing…» показывается поверх его строки.
            remorse.execute(wdg, qsTr("Removing «%1»…").arg(name),
                            function() {
                appWindow.voiceCmd(appWindow.cmdDelete, modelId)
            })
        } else {
            appWindow.voiceCmd(appWindow.cmdDownload, modelId)
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
                    // Фильтр страницы — чисто UI: глобальный язык агента не меняем.
                    var o = appWindow.voiceLangOptions
                    if (currentIndex < 0 || currentIndex >= o.length)
                        return
                    vmPage.filterLang = o[currentIndex].id
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
                                       + (dlstate === appWindow.stateDownloading
                                          ? Theme.paddingSmall + 6 : 0)
                                       + Theme.paddingMedium)

            onClicked: vmPage.onRowTap(isHeader, kind, modelId,
                                       engine, dlstate, downloaded)

            Label {
                id: headerArrow
                visible: isHeader
                width: Theme.itemSizeSmall
                x: Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                text: kind === vmPage.secRecognition
                      ? (vmPage.recExpanded ? "▾" : "▸")
                      : kind === vmPage.secSynthesis
                        ? (vmPage.ttsExpanded ? "▾" : "▸")
                        : (vmPage.dlExpanded ? "▾" : "▸")
                color: parent.highlighted ? Theme.highlightColor : Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
            }

            Label {
                id: headerTitle
                visible: isHeader
                x: Theme.horizontalPageMargin + Theme.itemSizeSmall + Theme.paddingSmall
                width: parent.width - x - Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                text: (total === done)
                      ? title + " (" + done + ")"
                      : title + " (" + done + "/" + total + ")"
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
                Row {
                    visible: dlstate === appWindow.stateDownloading
                    spacing: Theme.paddingMedium
                    width: parent.width
                    BusyIndicator {
                        running: parent.visible
                        size: BusyIndicatorSize.Small
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - Theme.paddingMedium - Theme.iconSizeSmall
                        text: vmPage.liveInfo(modelId)
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        truncationMode: TruncationMode.Fade
                    }
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
                text: vmPage.glyph({ state: dlstate,
                                    model_id: modelId,
                                    downloaded: downloaded })
                color: vmPage.glyphColor({ state: dlstate,
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
                visible: !isHeader && dlstate === appWindow.stateDownloading
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
                          : (dlstate === appWindow.stateDownloading
                             ? qsTr("Cancel download")
                             : (downloaded ? qsTr("Remove model") : qsTr("Download")))
                    onClicked: vmPage.onRowMenu(isHeader, kind, modelId,
                                                dlstate, downloaded, name, item)
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