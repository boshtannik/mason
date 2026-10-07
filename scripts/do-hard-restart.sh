#!/bin/sh
# Жёсткий рестарт harbour-opencode: booster-ритуал из мем-заметок (PID=32537/send_prompt OK).
echo "=== 1. убиваю harbour + booster'ы:"
killall harbour-opencode 2>/dev/null
echo "${SFOS_PASS:?задайте SFOS_PASS}" | devel-su killall booster-silica-qt5 booster-qt5 2>/dev/null
sleep 3

echo "=== 2. чищу qmlcache (на случай старых скомпилированных):"
rm -rf ~/.cache/harbour-opencode/qmlcache

echo "=== 3. старт через invoker (ровно как в ритуале):"
export XDG_RUNTIME_DIR=/run/user/100000
export WAYLAND_DISPLAY=../../display/wayland-0
export QT_QPA_PLATFORM=wayland
export EGL_PLATFORM=wayland
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket
nohup invoker --type=silica-qt5 -s /usr/bin/harbour-opencode >/dev/null 2>&1 &
sleep 14

echo "=== 4. PID (ожидаем число):"
pidof harbour-opencode

echo "=== 5. маркер корня (ожидаем: QML send_prompt -> \"\"):"
grep -F "QML send_prompt" ~/.cache/harbour-opencode/app.log | tail -4

echo "=== 6. хвост app.log:"
tail -6 ~/.cache/harbour-opencode/app.log
