#!/bin/sh
echo "=== ктo сейчас запущен:"
pidof harbour-opencode
echo "=== boostuid/booster'ы:"
pgrep -af booster | head
echo "=== корректный корень: MediaPlayer-объект отсутствует? (ожидаем 0)"
grep -cE "MediaPlayer *\{" /usr/share/harbour-opencode/qml/harbour-opencode.qml
echo "=== есть send_prompt (ожидаем >0):"
grep -c "send_prompt" /usr/share/harbour-opencode/qml/harbour-opencode.qml
echo "=== чистим qmlcache:"
rm -rf ~/.cache/harbour-opencode/qmlcache

PW="${SFOS_PASS:?задайте SFOS_PASS (пароль devel-su)}"
echo "$PW" | devel-su sh -c 'killall booster-silica-qt5 booster-qt5 2>/dev/null; sleep 2'
sleep 3
echo "$PW" | devel-su sh -c 'export XDG_RUNTIME_DIR=/run/user/100000 WAYLAND_DISPLAY=display/wayland-0 QT_QPA_PLATFORM=wayland EGL_PLATFORM=wayland DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket TF_YA_TEE=1; nohup invoker --type=silica-qt5 -s /usr/bin/harbour-opencode >/dev/null 2>&1 &'
sleep 14
echo "=== pidof после старта:"
pidof harbour-opencode
echo "=== пробник в app.log (ожидаем 'QML send_prompt'):"
grep -n "send_prompt" ~/.cache/harbour-opencode/app.log | tail -5
echo "=== хвост app.log:"
tail -8 ~/.cache/harbour-opencode/app.log
