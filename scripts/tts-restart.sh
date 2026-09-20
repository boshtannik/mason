#!/bin/sh
# Быстрый перезапуск harbour-opencode + проверка маркера инстанцирования корня.
set -x
echo 091772 | devel-su killall booster-silica-qt5 booster-qt5 booster-boostertest 2>/dev/null
killall harbour-opencode 2>/dev/null
sleep 2
rm -rf ~/.cache/harbour-opencode/qmlcache
export XDG_RUNTIME_DIR=/run/user/100000 WAYLAND_DISPLAY=../display/wayland-0 \
       QT_QPA_PLATFORM=wayland EGL_PLATFORM=wayland \
       DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket
nohup invoker --type=silica-qt5 -s /usr/bin/harbour-opencode >/dev/null 2>&1 &
sleep 12
echo PID=$(pidof harbour-opencode)
echo ---- app.log: маркер корня ----
grep -n "send_prompt" ~/.cache/harbour-opencode/app.log | tail -3
echo ---- app.log: последние ----
tail -6 ~/.cache/harbour-opencode/app.log
echo ---- есть ли MediaPlayer в боевом корне (ожидаем 0) ----
grep -c MediaPlayer /usr/share/harbour-opencode/qml/harbour-opencode.qml
