#!/bin/sh
# Перезапуск harbour-opencode на телефоне (dev-хелпер).
PW="${SFOS_PASS:?задайте SFOS_PASS (пароль devel-su)}"

echo "=== 1. убиваем старые booster'ы (держат отравленный QML-кэш):"
echo "$PW" | devel-su killall booster-silica-qt5 booster-qt5 2>/dev/null
killall harbour-opencode 2>/dev/null
sleep 4

echo "=== 2. чистим qmlcache устройства:"
rm -rf ~/.cache/harbour-opencode/qmlcache

echo "=== 3. поднимаем через invoker (НЕ booster: по заметкам booster держит MediaPlayer-кэш):"
export XDG_RUNTIME_DIR=/run/user/100000
export WAYLAND_DISPLAY=display/wayland-0
export QT_QPA_PLATFORM=wayland
export EGL_PLATFORM=wayland
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket
nohup invoker --type=silica-qt5 -s /usr/bin/harbour-opencode >/dev/null 2>&1 &
echo "запущено, жду 18 сек..."
sleep 18

echo "=== 4. PID (ожидаем число):"
pidof harbour-opencode

echo "=== 5. МАРКЕР КОРНЯ в app.log (ожидаем: QML send_prompt -> \"\"):"
grep -F 'QML send_prompt' ~/.cache/harbour-opencode/app.log | tail -4

echo "=== 6. хвост app.log:"
tail -8 ~/.cache/harbour-opencode/app.log
