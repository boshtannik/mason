import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: statusHeader
    // One line normally; while the server reports a retry the message wraps
    // over as many lines as it needs, then the header shrinks back.
    height: statusHeader.retry
            ? retryColumn.height + 2 * Theme.paddingSmall
            : Theme.itemSizeSmall

    property var appWindow

    // Server-reported retry status: `retry|<next_ms>|<attempt>|<message>`
    // (set by the worker on the `session.status` SSE event, see main.rs).
    // Mirrors the opencode TUI line: "message — retrying in N s - attempt #A".
    readonly property var retry: {
        var s = appWindow && appWindow.statusText ? appWindow.statusText : ""
        if (s.indexOf("retry|") !== 0)
            return null
        var p = s.substring(6).split("|")
        return {
            next: parseInt(p[0], 10),
            attempt: parseInt(p[1], 10),
            message: p.slice(2).join("|")
        }
    }
    // Ticks once per second so the countdown below is live.
    property int tick: 0
    Timer {
        interval: 1000
        repeat: true
        running: statusHeader.retry !== null
        onTriggered: statusHeader.tick++
    }
    readonly property int retrySeconds: {
        if (!statusHeader.retry) return 0
        statusHeader.tick // dependency: re-evaluate every second
        return Math.max(0, Math.ceil((statusHeader.retry.next - Date.now()) / 1000))
    }
    readonly property string retryText: {
        if (!statusHeader.retry) return ""
        var line = qsTr("retrying")
        if (statusHeader.retrySeconds > 0)
            line += " " + qsTr("in %1 s").arg(statusHeader.retrySeconds)
        var tail = qsTr("attempt #%1").arg(statusHeader.retry.attempt)
        return statusHeader.retry.message + " — " + line + " - " + tail
    }

    // Compact header: one line (idle / busy / connecting / error).
    Row {
        visible: !statusHeader.retry
        anchors {
            left: parent.left
            leftMargin: Theme.horizontalPageMargin
            verticalCenter: parent.verticalCenter
        }
        spacing: Theme.paddingSmall

        Label {
            text: "●"
            color: appWindow.statusColor(appWindow.statusText)
            font.pixelSize: Theme.fontSizeSmall
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            text: "opencode"
            color: Theme.primaryColor
            font.pixelSize: Theme.fontSizeMedium
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            text: appWindow.statusText
            width: Math.min(implicitWidth, statusHeader.width * 0.65)
            elide: Text.ElideRight
            color: appWindow.statusColor(appWindow.statusText)
            font.pixelSize: Theme.fontSizeExtraSmall
            anchors.verticalCenter: parent.verticalCenter
        }
    }

    // Retry header: title line + wrapped message, growing as needed.
    Column {
        id: retryColumn
        visible: !!statusHeader.retry
        x: Theme.horizontalPageMargin
        y: Theme.paddingSmall
        width: statusHeader.width - 2 * Theme.horizontalPageMargin
        spacing: Theme.paddingSmall

        Row {
            spacing: Theme.paddingSmall
            Label {
                text: "●"
                color: appWindow.statusColor(appWindow.statusText)
                font.pixelSize: Theme.fontSizeSmall
                anchors.verticalCenter: parent.verticalCenter
            }
            Label {
                text: "opencode"
                color: Theme.primaryColor
                font.pixelSize: Theme.fontSizeMedium
                anchors.verticalCenter: parent.verticalCenter
            }
        }
        Label {
            width: parent.width
            text: statusHeader.retryText
            wrapMode: Text.WordWrap
            color: appWindow.statusColor(appWindow.statusText)
            font.pixelSize: Theme.fontSizeExtraSmall
        }
    }
}
