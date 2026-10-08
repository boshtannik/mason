import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Item {
    id: chatPage

    property var appWindow

    readonly property bool busy: appWindow.statusText === "busy"

    // ms (unix) → «HH:MM» местного времени.
    function tsLabel(ms) {
        if (!ms)
            return ""
        var d = new Date(Number(ms))
        var h = d.getHours(), m = d.getMinutes()
        return (h < 10 ? "0" : "") + h + ":" + (m < 10 ? "0" : "") + m
    }

    // ── Мини-markdown → HTML (Text.RichText) ──────────────────────────────
    // Без сторонних библиотек: заголовки, списки, цитаты, жирный/курсив,
    // инлайновый код и fenced-блоки ```. Рендер базовый, но читабельный.

    function mdEscape(s) {
        return String(s)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
    }

    // Инлайновая разметка одной строки (после escape, без новострок).
    function mdInline(s) {
        // [text](url "title") → ссылка; код; жирный; курсив (*…* и _…_).
        s = s.replace(/\[([^\]]+)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g,
                      '<a href="$2" color="#82c4ff">$1</a>')
        s = s.replace(/`([^`]+)`/g, "<font face='monospace' color='#e0e0e0'>$1</font>")
        s = s.replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
        s = s.replace(/(^|[\s(])_([^_]+)_(?=\s|[)\]!.,:;]|$)/g, "$1<i>$2</i>")
        s = s.replace(/(^|[\s(])[*]([^*]+)[*](?=\s|[)\]!.,:;]|$)/g, "$1<i>$2</i>")
        return s
    }

    // Блоки по строкам: заголовки, списки, цитаты, разделители, fenced-код.
    function mdBlock(s) {
        var out = []
        var lines = s.split("\n")
        var inPre = false
        var preBuf = []
        for (var i = 0; i < lines.length; i++) {
            var l = lines[i]
            var fence = /^```([\w+-]*)\s*$/.exec(l)
            if (fence) {
                if (inPre) {
                    // Переносы внутри <pre> сохраняются дословно (Qt RichText),
                    // пробелы/отступы кода не схлопываются, шрифт моноширинный.
                    out.push("<pre>" + preBuf.join("\n") + "</pre>")
                    preBuf = []
                    inPre = false
                } else {
                    inPre = true
                }
                continue
            }
            if (inPre) { preBuf.push(l); continue }
            var h = /^(#{1,6})\s+(.*)$/.exec(l)
            if (h) {
                var sz = h[1].length === 1 ? "+3"
                       : h[1].length === 2 ? "+2"
                       : h[1].length === 3 ? "+1" : "+0"
                out.push("<font size=\"" + sz + "\"><b>" + chatPage.mdInline(h[2]) + "</b></font>")
                continue
            }
            var ul = /^[-*]\s+(.*)$/.exec(l)
            if (ul) { out.push("\u2022 " + chatPage.mdInline(ul[1])); continue }
            var ol = /^\s*(\d+)[.)]\s*(.*)$/.exec(l)
            if (ol) { out.push("<b>" + ol[1] + "</b>. " + chatPage.mdInline(ol[2])); continue }
            var qt = /^&gt;\s*(.*)$/.exec(l)
            if (qt) { out.push("&gt; <i>" + chatPage.mdInline(qt[1]) + "</i>"); continue }
            var hr = /^\s*([-*_])\1{2,}\s*$/.exec(l)
            if (hr) { out.push("<hr/>"); continue }
            out.push(chatPage.mdInline(l))
        }
        if (inPre) {
            out.push("<pre>" + preBuf.join("\n") + "</pre>")
        }
        return out.join("<br/>")
    }

    Connections {
        target: chatPage.appWindow
        onDictated: {
            if (text === "" || text === undefined)
                return
            var cur = input.text
            input.text = (cur.length > 0 ? cur + " " : "") + text
            input.focus = true
        }
    }

    StatusHeader {
        id: header
        appWindow: chatPage.appWindow
        width: parent.width
        anchors.top: parent.top
    }

    SilicaListView {
        id: chatList
        anchors {
            top: header.bottom
            left: parent.left
            right: parent.right
            bottom: inputPanel.top
        }
        model: appWindow.messages
        clip: true
        spacing: Theme.paddingSmall
        delegate: messageDelegate
        onCountChanged: positionViewAtEnd()
        VerticalScrollDecorator {}
    }

    // Dev-кнопка генерации мок-ошибок сервера: чтобы проверять отображение
    // ошибок в ленте без реального сервера (see bridge.mock_error).
    // Скрыта — в боевом интерфейсе не нужна.
    IconButton {
        id: mockErrBtn
        visible: false
        width: Theme.itemSizeMedium
        height: Theme.itemSizeMedium
        anchors {
            right: parent.right
            rightMargin: Theme.horizontalPageMargin
            bottom: inputPanel.top
            bottomMargin: Theme.paddingMedium
        }
        icon.source: "image://theme/icon-m-clear"
        highlighted: mockErrPanel.visible
        onClicked: {
            mockErrBtn.z = mockErrPanel.visible ? 0 : 10
            mockErrPanel.visible = !mockErrPanel.visible
        }
    }

    // Dev-панель: кнопки выбора мок-ошибки. Если не все умещаются —
    // прокручивается внутри (SilicaFlickable + VerticalScrollDecorator).
    Rectangle {
        id: mockErrPanel
        visible: false
        z: 20
        width: Math.min(parent.width - 2 * Theme.horizontalPageMargin,
                        Theme.itemSizeLarge * 7)
        height: Math.min(Theme.itemSizeMedium * 6 + Theme.paddingSmall * 5
                         + 2 * Theme.paddingMedium,   // полная высота списка
                         parent.height * 0.5)
        radius: Theme.paddingMedium
        color: Theme.rgba(Theme.overlayBackgroundColor, 0.92)
        border.color: Theme.primaryColor
        border.width: 1
        anchors {
            right: parent.right
            rightMargin: Theme.horizontalPageMargin
            bottom: mockErrBtn.top
            bottomMargin: Theme.paddingSmall
        }

        SilicaFlickable {
            id: mockErrFlick
            anchors {
                fill: parent
                margins: Theme.paddingMedium
            }
            contentHeight: mockErrCol.height
            VerticalScrollDecorator {}

            Column {
                id: mockErrCol
                width: parent.width
                spacing: Theme.paddingSmall

                Button {
                    width: parent.width
                    text: "ProviderAuthError"
                    onClicked: { appWindow.mockServerError("ProviderAuthError"); mockErrPanel.visible = false }
                }
                Button {
                    width: parent.width
                    text: "API Error 401"
                    onClicked: { appWindow.mockServerError("APIError401"); mockErrPanel.visible = false }
                }
                Button {
                    width: parent.width
                    text: "API Error 429 (quota)"
                    onClicked: { appWindow.mockServerError("APIError429"); mockErrPanel.visible = false }
                }
                Button {
                    width: parent.width
                    text: "MessageOutputLength"
                    onClicked: { appWindow.mockServerError("MessageOutputLengthError"); mockErrPanel.visible = false }
                }
                Button {
                    width: parent.width
                    text: "MessageAborted"
                    onClicked: { appWindow.mockServerError("MessageAbortedError"); mockErrPanel.visible = false }
                }
                Button {
                    width: parent.width
                    text: "UnknownError"
                    onClicked: { appWindow.mockServerError("UnknownError"); mockErrPanel.visible = false }
                }
                // Разделитель перед «нестандартными» событиями.
                Rectangle {
                    width: parent.width
                    height: 1
                    color: Theme.rgba(Theme.primaryColor, 0.25)
                }
                Button {
                    width: parent.width
                    text: "Permission ask (bash)"
                    onClicked: { appWindow.mockPermission(); mockErrPanel.visible = false }
                }
                Button {
                    width: parent.width
                    text: "Burst: 3 errors at once"
                    onClicked: {
                        appWindow.mockServerError("ProviderAuthError")
                        appWindow.mockServerError("APIError429")
                        appWindow.mockServerError("UnknownError")
                        mockErrPanel.visible = false
                    }
                }
            }
        }
    }

    Component {
        id: messageDelegate

        Item {
            id: line
            width: chatList.width
            height: bubble.height + Theme.paddingSmall

            // Сырая строка от воркера: `[[t:<ms>]]>>> текст` (юзер) или
            // `[[t:<ms>]]текст` (агент); живой юзер от `send()` — `[[t:<ms>]]>>> текст`.
            property string raw: "" + modelData
            property string tsMs: {
                var m = /\[\[t:(\d+)\]\]/.exec(line.raw)
                return m ? m[1] : ""
            }
            // Юзер-строки помечены префиксом «>>> » ПОСЛЕ таймстампа.
            property bool isUser: {
                var b = line.raw.replace(/^\[\[t:\d+\]\]/, "")
                return b.substring(0, 4) === ">>> "
            }
            // «Мышление» модели: строка `[[t:<ms>]][[think]]<text>` перед ответом.
            // Рендерится утопленным курсивом; озвучка/кнопка для неё недоступны.
            property bool isThink: {
                var b = line.raw.replace(/^\[\[t:\d+\]\]/, "")
                return b.substring(0, 8) === "[[think]]"
            }
            // Ошибка сервера (лимит, доступ к ИИ и т.п.) — отображается красноватым.
            property bool isError: line.raw.indexOf("[ошибка сервера]") === 0
            property string body: {
                var b = line.raw.replace(/^\[\[t:\d+\]\]/, "")
                if (b.substring(0, 8) === "[[think]]")
                    b = b.substring(8)
                if (b.substring(0, 4) === ">>> ")
                    b = b.substring(4)
                if (line.isError)
                    b = b.replace(/^\[ошибка сервера\]\s*/, "")
                return b.replace(/^\n+/, "")
            }
            readonly property real maxW: chatList.width - 2 * Theme.horizontalPageMargin
            // Ширина нижней строки: время + (для агента) кнопка озвучки + отступы.
            readonly property real footW: {
                var w = tsLabel.implicitWidth
                if (!line.isUser && !line.isThink && line.tsMs !== ""
                        && appWindow.ttsModelReady && appWindow.ttsMode !== "off")
                    w += Theme.itemSizeMedium + Theme.paddingSmall
                return w
            }

            Rectangle {
                id: bubble
                radius: Theme.paddingMedium
                color: line.isThink
                       ? Theme.rgba(Theme.primaryColor, 0.05)
                       : (line.isUser
                          ? Theme.highlightColor
                          : (line.isError
                             ? Qt.rgba(0.85, 0.15, 0.15, 0.18)
                             : Theme.rgba(Theme.primaryColor, 0.12)))
                width: Math.min(line.maxW,
                                Math.max(textLabel.implicitWidth,
                                         line.footW + 2 * Theme.paddingSmall)
                                + 2 * Theme.paddingSmall)
                height: textLabel.height
                        + (footRow.height > 0 ? footRow.height + Theme.paddingSmall : 0)
                        + 2 * Theme.paddingSmall
                anchors.right: line.isUser ? parent.right : undefined
                anchors.left: line.isUser ? undefined : parent.left
                anchors.rightMargin: line.isUser ? Theme.horizontalPageMargin : 0
                anchors.leftMargin: line.isUser ? 0 : Theme.horizontalPageMargin

                Label {
                    id: textLabel
                    x: Theme.paddingSmall
                    y: Theme.paddingSmall
                    width: Math.min(line.maxW - 2 * Theme.paddingSmall, implicitWidth)
                    text: chatPage.mdBlock(chatPage.mdEscape(line.body))
                    textFormat: Text.RichText
                    wrapMode: Text.Wrap
                    onLinkActivated: Qt.openUrlExternally(link)
                    font.pixelSize: Theme.fontSizeSmall
                    font.italic: line.isThink
                    color: line.isThink ? Theme.secondaryColor : Theme.primaryColor
                }
                // Нижняя строка: время (+ для агента кнопка озвучки).
                // Прижата к своему краю баббла, с отступами от краёв.
                Row {
                    id: footRow
                    visible: line.tsMs !== ""
                    anchors.right: line.isUser ? undefined : bubble.right
                    anchors.left: line.isUser ? bubble.left : undefined
                    anchors.rightMargin: Theme.paddingSmall + 1
                    anchors.leftMargin: Theme.paddingSmall
                    y: textLabel.y + textLabel.height + Theme.paddingSmall
                    spacing: Theme.paddingMedium

                    // 🔊 озвучить этот баббл: только ответы агента (не-user).
                    // Hit-область — полный размер иконок `iconSizeMedium`, чтобы
                    // лёгко попадать пальцем.
                    // Когда синтезируется/озвучивается ИМЕННО ЭТОТ баббл — кнопка
                    // превращается в «стоп» (иконка stop + лоадер во время синтеза),
                    // чтобы прерывать озвучку прямо на самом баббле, а не отдельной
                    // кнопкой снизу.
                    IconButton {
                        id: speakBtn
                        visible: !line.isUser && !line.isThink
                                 && appWindow.ttsModelReady
                                 && appWindow.ttsMode !== "off"
                        width: visible ? Theme.itemSizeMedium : 0
                        height: visible ? Theme.itemSizeMedium : 0
                        property bool active: appWindow.speakingBubble === line.body
                                              && (appWindow.ttsSynth || appWindow.ttsPlaying)
                        icon.source: active
                                     ? "image://theme/icon-m-stop"
                                     : "image://theme/icon-m-speaker-on"
                        highlighted: active
                        // Пока синтез речи этого баббла — лоадер вместо иконки.
                        BusyIndicator {
                            anchors.centerIn: parent
                            running: visible
                            visible: appWindow.ttsSynth
                                     && appWindow.speakingBubble === line.body
                        }
                        onClicked: {
                            if (active)
                                appWindow.stopTts()
                            else
                                appWindow.speakText(line.body)
                        }
                    }

                    Label {
                        id: tsLabel
                        text: chatPage.tsLabel(line.tsMs)
                        anchors.verticalCenter: parent.verticalCenter
                        font.pixelSize: Theme.fontSizeExtraSmall
                        color: Theme.secondaryColor
                    }
                }
            }
        }
    }

    Item {
        id: inputPanel
        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
        // Один уровень ввода: высота панели = высота одной строки кнопок.
        height: Theme.itemSizeMedium + Theme.paddingMedium

        // ── Константные раскладки панели ввода ──────────────────────────────
        // Вариантов мало — задаём каждый явно через State (анкеры), без
        // вычисления координат (расстановка «плавающими» X ломалась).
        //  - text:      [input, send]
        //  - ptt left:  [mic, input, send]
        //  - ptt center:[input, mic, send]
        //  - ptt right: [input, mic, send]  (mic у кнопки send)
        //  - voice left/center/right: [mic, send] / [mic·центр, send] / [mic, send]
        property string layoutKey: {
            if (appWindow.inputMode === "voice")
                return "voice" + appWindow.pttPosition
            if (appWindow.inputMode === "text")
                return "text"
            // В режиме «текст + диктовка» центр недопустим (только слева/справа).
            if (appWindow.pttPosition === "center")
                return "pttleft"
            return "ptt" + appWindow.pttPosition
        }
        states: [
            State {
                name: "text"
                when: inputPanel.layoutKey === "text"
                AnchorChanges {
                    target: input
                    anchors.left: inputPanel.left
                    anchors.right: send.left
                }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "pttleft"
                when: inputPanel.layoutKey === "pttleft"
                AnchorChanges {
                    target: mic
                    anchors.left: inputPanel.left
                }
                AnchorChanges {
                    target: input
                    anchors.left: mic.right
                    anchors.right: send.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "pttcenter"
                when: inputPanel.layoutKey === "pttcenter"
                AnchorChanges {
                    target: mic
                    anchors.horizontalCenter: inputPanel.horizontalCenter
                }
                AnchorChanges {
                    target: input
                    anchors.left: inputPanel.left
                    anchors.right: mic.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "pttright"
                when: inputPanel.layoutKey === "pttright"
                AnchorChanges {
                    target: mic
                    anchors.right: send.left
                }
                AnchorChanges {
                    target: input
                    anchors.left: inputPanel.left
                    anchors.right: mic.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.rightMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.leftMargin: Theme.paddingSmall }
                PropertyChanges { target: input; anchors.rightMargin: Theme.paddingSmall }
            },
            State {
                name: "voiceleft"
                when: inputPanel.layoutKey === "voiceleft"
                AnchorChanges {
                    target: mic
                    anchors.left: inputPanel.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.leftMargin: Theme.paddingSmall }
            },
            State {
                name: "voicecenter"
                when: inputPanel.layoutKey === "voicecenter"
                AnchorChanges {
                    target: mic
                    anchors.horizontalCenter: inputPanel.horizontalCenter
                }
                PropertyChanges { target: mic; visible: true }
            },
            State {
                name: "voiceright"
                when: inputPanel.layoutKey === "voiceright"
                AnchorChanges {
                    target: mic
                    anchors.right: send.left
                }
                PropertyChanges { target: mic; visible: true }
                PropertyChanges { target: mic; anchors.rightMargin: Theme.paddingSmall }
            }
        ]

        IconButton {
            id: mic
            visible: false
            width: Theme.itemSizeMedium
            height: Theme.itemSizeMedium
            anchors.verticalCenter: inputPanel.verticalCenter
            // Горизонтальную привязку задаёт только state (left/центр/right),
            // чтобы не возникало конфликта анкеров при центровке.
            icon.source: (appWindow.recording || appWindow.recognizing)
                          ? "" : "image://theme/icon-m-mic"

            // Лоадер вместо иконки, пока идёт запись или распознавание.
            BusyIndicator {
                anchors.fill: parent
                running: appWindow.recording || appWindow.recognizing
                visible: running
            }

            // PTT-холд: зажал → запись, отпустил → стоп + распознавание.
            MouseArea {
                anchors.fill: parent
                onPressed: appWindow.startPttHold()
                onReleased: appWindow.stopPttHold()
                onCanceled: appWindow.stopPttHold()
            }
        }

        IconButton {
            id: send
            visible: !chatPage.busy
            width: Theme.itemSizeMedium
            height: Theme.itemSizeMedium
            anchors {
                right: inputPanel.right
                rightMargin: Theme.paddingSmall
                verticalCenter: inputPanel.verticalCenter
            }
            icon.source: "image://theme/icon-m-enter-accept"
            enabled: input.text.length > 0
            onClicked: {
                appWindow.send(input.text)
                input.text = ""
            }
        }

        IconButton {
            id: stop
            visible: chatPage.busy
            width: Theme.itemSizeMedium
            height: Theme.itemSizeMedium
            anchors {
                right: inputPanel.right
                rightMargin: Theme.paddingSmall
                verticalCenter: inputPanel.verticalCenter
            }
            icon.source: "image://theme/icon-m-stop"
            onClicked: appWindow.stopAgent()
        }

        TextArea {
            id: input
            visible: appWindow.inputMode !== "voice"
            enabled: !chatPage.busy
            background: null
            // Плейсхолдер рисуем внутри поля, а не «этажом» сверху — иначе
            // поле выглядит двухуровневым.
            labelVisible: false
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: Text.AlignLeft
            // Одна строка той же высоты, что и кнопки: низ в один уровень.
            height: Theme.itemSizeMedium
            anchors.verticalCenter: inputPanel.verticalCenter
            // Дефолтные анкеры (ширина в стартовой раскладке [input, send]);
            // state переопределяет их под выбранную схему.
            anchors.left: inputPanel.left
            anchors.leftMargin: Theme.paddingSmall
            anchors.right: send.left
            anchors.rightMargin: Theme.paddingSmall
            placeholderText: chatPage.busy
                             ? qsTr("Agent is working…")
                             : qsTr("Agent prompt…")
            EnterKey.enabled: text.length > 0 && !chatPage.busy
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: {
                appWindow.send(input.text)
                input.text = ""
            }
        }
    }
}
