import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Item {
    id: chatPage

    property var appWindow

    readonly property bool busy: appWindow.statusBase(appWindow.statusText) === "busy"

    // ms (unix) → "HH:MM" local time.
    function tsLabel(ms) {
        if (!ms)
            return ""
        var d = new Date(Number(ms))
        var h = d.getHours(), m = d.getMinutes()
        return (h < 10 ? "0" : "") + h + ":" + (m < 10 ? "0" : "") + m
    }

    // ── Mini-markdown → HTML (Text.RichText) ──────────────────────────────
    // No third-party libraries: headings, lists, quotes, bold/italic,
    // inline code and ``` fenced blocks. Rendering is basic but readable.

    function mdEscape(s) {
        return String(s)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
    }

    // Inline markup of a single line (after escaping, no newlines).
    function mdInline(s) {
        // [text](url "title") → link; code; bold; italic (*…* and _…_).
        s = s.replace(/\[([^\]]+)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g,
                      '<a href="$2" color="#82c4ff">$1</a>')
        s = s.replace(/`([^`]+)`/g, "<font face='monospace' color='#e0e0e0'>$1</font>")
        s = s.replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
        s = s.replace(/(^|[\s(])_([^_]+)_(?=\s|[)\]!.,:;]|$)/g, "$1<i>$2</i>")
        s = s.replace(/(^|[\s(])[*]([^*]+)[*](?=\s|[)\]!.,:;]|$)/g, "$1<i>$2</i>")
        return s
    }

    // Block-level per line: headings, lists, quotes, rules, fenced code.
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
                    // Line breaks inside <pre> are preserved verbatim (Qt RichText),
                    // code spaces/indentation are not collapsed, the font is monospace.
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

    // Dedicated HTML for a code block: monospace, verbatim whitespace/newlines.
    function mdCode(s) {
        return "<pre>" + chatPage.mdEscape(s) + "</pre>"
    }

    // Split a bubble body into display segments so that fenced code blocks and
    // whole-line links become their own blocks (each with a copy button):
    //   {code:false, link:false, text:<prose>, url:"", lang:""}
    //   {code:true,  link:false, text:<code>, url:"", lang:"<fence-lang>"}
    //   {code:false, link:true,  text:<title>, url:<url>, lang:""}
    function splitBody(body) {
        var segs = []
        var prose = []
        function flush() {
            if (prose.length > 0) {
                segs.push({ code: false, link: false, text: prose.join("\n"),
                            url: "", lang: "" })
                prose = []
            }
        }
        // A whole line that is exactly one link: `- [title](url)` / `[title](url)`
        // / `- url` / `url`.
        var linkRe = /^\s*(?:[-*]\s+)?(?:\[([^\]]+)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)|(https?:\/\/\S+))\s*$/
        var fenceRe = /^```([\w+-]*)\s*$/
        var inPre = false
        var preBuf = []
        var lines = String(body).split("\n")
        for (var i = 0; i < lines.length; i++) {
            var l = lines[i]
            var fence = fenceRe.exec(l)
            if (fence) {
                if (inPre) {
                    flush()
                    segs.push({ code: true, link: false,
                                text: preBuf.join("\n").replace(/\s+$/, ""),
                                url: "", lang: fence[1] })
                    preBuf = []
                    inPre = false
                } else {
                    flush()
                    inPre = true
                }
                continue
            }
            if (inPre) { preBuf.push(l); continue }
            var lm = linkRe.exec(l)
            if (lm) {
                flush()
                var url = lm[2] !== undefined ? lm[2] : lm[3]
                var title = lm[1] !== undefined ? lm[1] : url
                segs.push({ code: false, link: true, text: title, url: url, lang: "" })
                continue
            }
            prose.push(l)
        }
        if (inPre) {
            flush()
            segs.push({ code: true, link: false,
                        text: preBuf.join("\n").replace(/\s+$/, ""),
                        url: "", lang: "" })
        }
        flush()
        return segs
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
        // Cover action "pen" (text-only mode): focus the prompt input and
        // open the keyboard right away.
        onQuickComposeRequested: {
            input.forceActiveFocus()
            Qt.inputMethod.show()
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

    // Dev button that generates mock server errors: to test how errors
    // are displayed in the feed without a real server (see bridge.mock_error).
    // Hidden — not needed in the production UI.
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

    // Dev panel: buttons to pick a mock error. If not all fit —
    // it scrolls inside (SilicaFlickable + VerticalScrollDecorator).
    Rectangle {
        id: mockErrPanel
        visible: false
        z: 20
        width: Math.min(parent.width - 2 * Theme.horizontalPageMargin,
                        Theme.itemSizeLarge * 7)
        height: Math.min(Theme.itemSizeMedium * 6 + Theme.paddingSmall * 5
                         + 2 * Theme.paddingMedium,   // full height of the list
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
                // Separator before the "non-standard" events.
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
            // Model "thinking": a binding to the "show thinking" switch
            // hides/shows already rendered [[think]] lines at once, without
            // reopening the session (new ones are not accumulated when the setting is off —
            // poll() in the root QML filters them out).
            visible: appWindow.showReasoning || !line.isThink
            height: visible ? bubble.height + Theme.paddingSmall : 0

            // Raw line from the worker: `[[t:<ms>]]>>> text` (user) or
            // `[[t:<ms>]]text` (agent); a live user line from `send()` — `[[t:<ms>]]>>> text`.
            property string raw: "" + modelData
            property string tsMs: {
                var m = appWindow.tsRe.exec(line.raw)
                return m ? m[1] : ""
            }
            // User lines are marked with the ">>> " prefix AFTER the timestamp.
            property bool isUser: appWindow.lineIs(appWindow.stripTs(line.raw),
                                                  appWindow.kUserPrefix)
            // Model "thinking": the line `[[t:<ms>]][[think]]<text>` before the answer.
            // Rendered as recessed italic; TTS/the button are unavailable for it.
            property bool isThink: appWindow.lineIs(appWindow.stripTs(line.raw),
                                                   appWindow.kThinkTag)
            // Server/service error (limits, AI access, etc.) — red-ish bubble.
            // Worker sends `[[e:KEY…]]`; QML stores the line as `[[e]]<text>`.
            property bool isError: appWindow.lineIs(appWindow.stripTs(line.raw),
                                                    appWindow.kErrTag)
            // Service notice (voice/STT/TTS/catalog progress) — a SYSTEM bubble:
            // muted styling, no speak button, no auto-TTS (never an answer).
            property bool isSvc: appWindow.lineIs(appWindow.stripTs(line.raw),
                                                 appWindow.kSvcTag)
            property bool isSystem: line.isError || line.isSvc
            property string body: {
                var b = appWindow.stripTs(line.raw)
                if (appWindow.lineIs(b, appWindow.kThinkTag))
                    b = b.substring(appWindow.kThinkTag.length)
                if (appWindow.lineIs(b, appWindow.kUserPrefix))
                    b = b.substring(appWindow.kUserPrefix.length)
                if (line.isError)
                    b = b.substring(appWindow.kErrTag.length)
                if (line.isSvc)
                    b = b.substring(appWindow.kSvcTag.length)
                return b.replace(/^\n+/, "")
            }
            // The body split into display segments: prose, fenced-code blocks
            // and whole-line links — so copy buttons can sit next to each block.
            // `{code, link, text, url, lang}` (see chatPage.splitBody).
            property var segments: chatPage.splitBody(line.body)
            readonly property real maxW: chatList.width - 2 * Theme.horizontalPageMargin
            // Width of the bottom line: time + (copy and, for the agent, TTS) buttons + padding.
            readonly property real footW: {
                var w = tsLabel.implicitWidth
                if (!line.isUser && !line.isSystem && line.tsMs !== "") {
                    // copy (always on agent/thinking lines)
                    w += Theme.itemSizeMedium + Theme.paddingSmall
                    // speak (only when TTS is available)
                    if (appWindow.ttsModelReady && appWindow.ttsMode !== "off")
                        w += Theme.itemSizeMedium + Theme.paddingSmall
                }
                return w
            }

            // Hidden measuring label: keeps the bubble width in line with the
            // old single-label layout (natural width of the whole body).
            Label {
                id: measLabel
                visible: false
                width: line.maxW - 2 * Theme.paddingSmall
                text: line.body
                font.pixelSize: Theme.fontSizeSmall
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
                             : (line.isSvc
                                ? Theme.rgba(Theme.secondaryColor, 0.10)
                                : Theme.rgba(Theme.primaryColor, 0.12))))
                width: Math.min(line.maxW,
                                Math.max(measLabel.implicitWidth,
                                         line.footW + 2 * Theme.paddingSmall)
                                + 2 * Theme.paddingSmall)
                height: bodyCol.height
                        + (footRow.height > 0 ? footRow.height + Theme.paddingSmall : 0)
                        + 2 * Theme.paddingSmall
                anchors.right: line.isUser ? parent.right : undefined
                anchors.left: line.isUser ? undefined : parent.left
                anchors.rightMargin: line.isUser ? Theme.horizontalPageMargin : 0
                anchors.leftMargin: line.isUser ? 0 : Theme.horizontalPageMargin

                Column {
                    id: bodyCol
                    x: Theme.paddingSmall
                    y: Theme.paddingSmall
                    width: bubble.width - 2 * Theme.paddingSmall
                    spacing: Theme.paddingSmall

                    Repeater {
                        model: line.segments
                        delegate: Item {
                            id: segItem
                            width: bodyCol.width
                            property var seg: modelData
                            height: segItem.seg.code ? segBox.height
                                    : segItem.seg.link ? linkRow.height
                                                       : proseLbl.height

                            // Free-form prose (markdown → rich text).
                            Label {
                                id: proseLbl
                                visible: !segItem.seg.code && !segItem.seg.link
                                width: parent.width
                                text: chatPage.mdBlock(chatPage.mdEscape(segItem.seg.text))
                                textFormat: Text.RichText
                                wrapMode: Text.Wrap
                                onLinkActivated: Qt.openUrlExternally(link)
                                font.pixelSize: Theme.fontSizeSmall
                                font.italic: line.isThink
                                color: (line.isThink || line.isSvc)
                                       ? Theme.secondaryColor : Theme.primaryColor
                            }

                            // A whole-line link: the URL itself + a copy button
                            // right beside it.
                            Row {
                                id: linkRow
                                visible: segItem.seg.link
                                width: parent.width
                                height: Math.max(urlLbl.height, copyLinkBtn.height)
                                spacing: Theme.paddingSmall
                                Label {
                                    id: urlLbl
                                    width: parent.width - copyLinkBtn.width - parent.spacing
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: segItem.seg.url
                                    wrapMode: Text.Wrap
                                    font.pixelSize: Theme.fontSizeSmall
                                    font.family: "monospace"
                                    color: Theme.highlightColor
                                }
                                // Tap the URL itself to open it (the copy button
                                // sits to the right).
                                MouseArea {
                                    anchors.fill: urlLbl
                                    onClicked: Qt.openUrlExternally(segItem.seg.url)
                                }
                                IconButton {
                                    id: copyLinkBtn
                                    width: Theme.itemSizeSmall
                                    height: Theme.itemSizeSmall
                                    anchors.verticalCenter: parent.verticalCenter
                                    icon.source: "image://theme/icon-m-clipboard"
                                    onClicked: appWindow.copyText(segItem.seg.url)
                                }
                            }

                            // Fenced code block: own card with a light header bar
                            // and a copy button in its top-right corner.
                            Rectangle {
                                id: segBox
                                visible: segItem.seg.code
                                width: parent.width
                                radius: Theme.paddingSmall
                                color: Theme.rgba(Theme.highlightColor, 0.08)
                                height: codeHeader.height
                                        + codeLbl.height + 2 * Theme.paddingSmall
                                Rectangle {
                                    id: codeHeader
                                    width: parent.width
                                    height: Theme.itemSizeSmall
                                    radius: Theme.paddingSmall
                                    color: Theme.rgba(Theme.highlightColor, 0.15)
                                    Label {
                                        anchors.left: parent.left
                                        anchors.leftMargin: Theme.paddingSmall
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: segItem.seg.lang
                                        visible: segItem.seg.lang.length > 0
                                        color: Theme.secondaryColor
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                    }
                                    IconButton {
                                        id: copyCodeBtn
                                        anchors.right: parent.right
                                        anchors.rightMargin: Theme.paddingSmall
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: Theme.itemSizeSmall
                                        height: Theme.itemSizeSmall
                                        icon.source: "image://theme/icon-m-clipboard"
                                        onClicked: appWindow.copyText(segItem.seg.text)
                                    }
                                }
                                Label {
                                    id: codeLbl
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.leftMargin: Theme.paddingSmall
                                    anchors.rightMargin: Theme.paddingSmall
                                    anchors.top: codeHeader.bottom
                                    text: chatPage.mdCode(segItem.seg.text)
                                    textFormat: Text.RichText
                                    wrapMode: Text.Wrap
                                    font.pixelSize: Theme.fontSizeSmall
                                    color: Theme.primaryColor
                                }
                            }
                        }
                    }
                }

                // Bottom line: time (+ copy and, for the agent, the TTS button).
                // Pinned to its own bubble edge, with padding from the edges.
                Row {
                    id: footRow
                    visible: line.tsMs !== ""
                    anchors.right: line.isUser ? undefined : bubble.right
                    anchors.left: line.isUser ? bubble.left : undefined
                    anchors.rightMargin: Theme.paddingSmall + 1
                    anchors.leftMargin: Theme.paddingSmall
                    y: bodyCol.y + bodyCol.height + Theme.paddingSmall
                    spacing: Theme.paddingMedium

                    // 📋 copy the whole answer (or thinking) to the clipboard.
                    IconButton {
                        id: copyBtn
                        visible: !line.isUser && !line.isSystem
                        width: visible ? Theme.itemSizeMedium : 0
                        height: visible ? Theme.itemSizeMedium : 0
                        icon.source: "image://theme/icon-m-clipboard"
                        onClicked: appWindow.copyText(line.body)
                    }

                    // 🔊 speak this bubble: agent answers only (not user).
                    // Hit area — the full `iconSizeMedium` icon size, so
                    // it is easy to hit with a finger.
                    // When THIS VERY bubble is being synthesised/spoken, the button
                    // turns into "stop" (stop icon + loader during synthesis),
                    // so TTS can be interrupted right on the bubble itself, not via a separate
                    // button below.
                    IconButton {
                        id: speakBtn
                        visible: !line.isUser && !line.isSystem
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
                        // While this bubble is being synthesised — a loader instead of the icon.
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
        // Single input level: the panel height = the height of one row of buttons.
        height: Theme.itemSizeMedium + Theme.paddingMedium

        // ── Fixed input panel layouts ──────────────────────────────────────
        // There are few variants — we define each explicitly via State (anchors), without
        // computing coordinates (laying out with "floating" X used to break).
        //  - text:      [input, send]
        //  - ptt left:  [mic, input, send]
        //  - ptt center:[input, mic, send]
        //  - ptt right: [input, mic, send]  (mic next to the send button)
        //  - voice left/center/right: [mic, send] / [mic·center, send] / [mic, send]
        property string layoutKey: {
            if (appWindow.inputMode === "voice")
                return "voice" + appWindow.pttPosition
            if (appWindow.inputMode === "text")
                return "text"
            // In "text + dictation" mode the center is not allowed (only left/right).
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
            // The horizontal anchor is set only by state (left/center/right),
            // so that no anchor conflict arises when centering.
            icon.source: (appWindow.recording || appWindow.recognizing)
                          ? "" : "image://theme/icon-m-mic"

            // A loader instead of the icon while recording or recognising.
            BusyIndicator {
                anchors.fill: parent
                running: appWindow.recording || appWindow.recognizing
                visible: running
            }

            // PTT hold: pressed → recording, released → stop + recognition.
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
            // We draw the placeholder inside the field, not as a layer on top — otherwise
            // the field looks two-level.
            labelVisible: false
            verticalAlignment: Text.AlignVCenter
            horizontalAlignment: Text.AlignLeft
            // One line of the same height as the buttons: the bottom aligns to one level.
            height: Theme.itemSizeMedium
            anchors.verticalCenter: inputPanel.verticalCenter
            // Default anchors (width in the initial layout [input, send]);
            // the state overrides them for the chosen scheme.
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
