import QtQuick 2.6

Item {
    property int waited: 0

    Component.onCompleted: {
        console.log("[TEST] старт, посылаю промпт")
        bridge.send_prompt("Ответь одним числом: 2+2=?")
    }

    Timer {
        id: t
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            waited++
            var msgs = "" + bridge.drain_messages()
            if (msgs !== "") {
                console.log("[TEST] ОТВЕТ ПОЛУЧЕН: \"" + msgs + "\"")
                Qt.quit()
            } else if (waited >= 45) {
                console.log("[TEST] ТАЙМАУТ: ответ не пришёл за " + waited + "s")
                Qt.quit()
            }
        }
    }
}