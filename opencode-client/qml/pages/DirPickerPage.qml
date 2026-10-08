import QtQuick 2.6
import Sailfish.Silica 1.0

// Проводник для выбора рабочей директории (мост bridge.list_dir).
// Показываем каталоги: тап по каталогу — войти, длинное нажатие — выбрать
// его. «Use this folder» — выбрать текущий путь. Есть фильтр по имени,
// быстрые кнопки (Home, Root) и ручной ввод пути.
Page {
    id: dirPickerPage

    property var appWindow
    property string startPath: ""
    property string currentPath: "/"
    // TextField на странице настроек, чтобы показать выбранный путь.
    property var targetField
    // Полный список каталогов текущей директории (для фильтра).
    property var allEntries: []
    property string filter: ""
    // Умеем ли мы отличать «пусто» от «не открылось».
    property bool lastOk: true

    ListModel { id: entriesModel }

    function parentOf(p) {
        var norm = String(p)
        while (norm.length > 1 && norm.charAt(norm.length - 1) === "/")
            norm = norm.slice(0, -1)
        if (norm.length <= 1)
            return "/"
        var idx = norm.lastIndexOf("/")
        if (idx <= 0)
            return "/"
        return norm.slice(0, idx)
    }

    function reload() {
        allEntries = []
        if (appWindow === undefined || appWindow.bridge === undefined)
            return
        var data = {}
        lastOk = false
        try {
            data = JSON.parse(appWindow.bridge.list_dir(currentPath))
        } catch (e) { }
        if (data.path !== undefined && data.path.length > 0)
            currentPath = data.path
        lastOk = data.ok === true
        if (currentPath !== "/")
            allEntries.push({ name: "..", path: parentOf(currentPath), isDir: true })
        var items = data.entries || []
        for (var i = 0; i < items.length; i++)
            allEntries.push(items[i])
        pathLabel.text = currentPath
        applyFilter()
    }

    function applyFilter() {
        entriesModel.clear()
        var f = filter.trim().toLowerCase()
        for (var i = 0; i < allEntries.length; i++) {
            var e = allEntries[i]
            if (e.name === ".." || f.length === 0 || e.name.toLowerCase().indexOf(f) >= 0)
                entriesModel.append(e)
        }
        if (!lastOk) {
            statusLabel.text = qsTr("Not accessible")
        } else if (entriesModel.count === 0) {
            statusLabel.text = qsTr("Empty directory")
        } else {
            statusLabel.text = ""
        }
    }

    function goTo(p) {
        currentPath = String(p)
        filter = ""
        if (searchField.text.length > 0)
            searchField.text = ""
        reload()
        pathField.text = ""
    }

    function choose(p) {
        if (appWindow !== undefined && appWindow.bridge !== undefined) {
            appWindow.workdir = p
            appWindow.bridge.set_workdir(p)
        }
        if (targetField !== undefined)
            targetField.text = p
        pageStack.pop()
    }

    SilicaListView {
        id: list
        anchors.fill: parent
        model: entriesModel

        header: Column {
            width: parent.width
            PageHeader { title: qsTr("Working directory") }

            Label {
                id: pathLabel
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: "/"
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
                elide: Text.ElideMiddle
            }

            Row {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                spacing: Theme.paddingMedium
                Button {
                    text: qsTr("Home")
                    onClicked: goTo(appWindow !== undefined && appWindow.bridge !== undefined
                                   ? appWindow.bridge.app_home() : "/")
                }
                Button {
                    text: qsTr("Root")
                    onClicked: goTo("/")
                }
                Button {
                    text: qsTr("Type path…")
                    onClicked: { pathField.visible = !pathField.visible; pathField.focus = pathField.visible }
                }
            }

            TextField {
                id: pathField
                visible: false
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                placeholderText: qsTr("e.g. /home/defaultuser/mason")
                EnterKey.iconSource: "image://theme/icon-m-enter-accept"
                EnterKey.onClicked: {
                    if (text.length > 0)
                        goTo(text)
                }
            }

            SearchField {
                id: searchField
                width: parent.width
                placeholderText: qsTr("Filter…")
                onTextChanged: {
                    dirPickerPage.filter = text
                    dirPickerPage.applyFilter()
                }
            }

            Button {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("Use this folder")
                highlighted: true
                onClicked: choose(currentPath)
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - 2 * Theme.horizontalPageMargin
                text: qsTr("Tap a folder to open it, long press to select it")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                wrapMode: Text.Wrap
            }
        }

        delegate: ListItem {
            contentHeight: Theme.itemSizeMedium
            onClicked: {
                if (model.name === ".." || typeof model.path !== "undefined")
                    goTo(model.path)
            }
            onPressAndHold: {
                if (model.name !== "..")
                    choose(model.path)
            }
            Image {
                id: folderIcon
                x: Theme.horizontalPageMargin
                width: Theme.iconSizeMedium
                height: Theme.iconSizeMedium
                anchors.verticalCenter: parent.verticalCenter
                source: "image://theme/"
                        + (model.name === ".." ? "icon-m-level-up" : "icon-m-folder")
                sourceSize.width: width
                sourceSize.height: height
            }
            Label {
                x: folderIcon.x + folderIcon.width + Theme.paddingMedium
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - folderIcon.width - Theme.paddingMedium
                         - 2 * Theme.horizontalPageMargin
                text: model.name === ".." ? qsTr("Up one level") : model.name
                truncationMode: TruncationMode.Fade
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }

        footer: Label {
            id: statusLabel
            visible: text.length > 0
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: ""
            color: Theme.secondaryColor
            font.pixelSize: Theme.fontSizeSmall
            wrapMode: Text.Wrap
        }

        VerticalScrollDecorator {}
    }

    onStatusChanged: {
        if (status === PageStatus.Active) {
            if (startPath.length > 0) {
                currentPath = startPath
                startPath = ""
            } else if (currentPath === "/"
                && appWindow !== undefined && appWindow.bridge !== undefined) {
                currentPath = appWindow.bridge.app_home()
            } else {
                return
            }
            reload()
        }
    }
}