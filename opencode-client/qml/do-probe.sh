#!/bin/sh
# Чистый старт harbour-opencode на телефоне (по рабочему шаблону из заметок).
# Выполнять на устройстве от defaultuser.
# Пароль devel-su берётся из $SFOS_PASS.
PW="${SFOS_PASS:?задайте SFOS_PASS (пароль devel-su)}"
export XDG_RUNTIME_DIR=/run/user/100000
export WAYLAND_DISPLAY=../../display/wayland-0
export QT_QPA_PLATFORM=wayland
export EGL_PLATFORM=wayland
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket

echo "=== 1. убиваем старое + booster'ы и qmlcache:"
killall harbour-opencode 2>/dev/null
echo "$PW" | devel-su killall booster-silica-qt5 booster-qt5 2>/dev/null
sleep 3
rm -rf ~/.cache/harbour-opencode/qmlcache
echo "==== 2. стартуем invoker:"
nohup invoker --type=silica-qt5 -s /usr/bin/harbour-opencode >/dev/null 2>&1 &
sleep 15
echo "==== 3. PID (ожидаем число):"
pidof harbour-opencode
echo "==== 4. маркер корня в app.log (ожидаем: QML send_prompt -> \"\"):"
grep -F 'QML send_prompt' ~/.cache/harbour-opencode/app.log | tail -3
echo "==== 5. хвост app.log:"
tail -8 ~/.cache/harbour-opencode/app.log
