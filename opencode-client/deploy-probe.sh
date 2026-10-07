#!/bin/sh
PW="${SFOS_PASS:?задайте SFOS_PASS (пароль SSH от телефона)}"
H="${SFOS_HOST:-defaultuser@192.168.76.232}"
sshpass -p $PW ssh -o ConnectTimeout=25 $H "
  md5sum /usr/share/harbour-opencode/qml/harbour-opencode.qml
  echo --- пробник в боевом (ожидаем: строка с send_prompt):;
  grep -nF 'send_prompt' /usr/share/harbour-opencode/qml/harbour-opencode.qml | head -3
  echo --- MediaPlayer-ОБЪЕКТ (ожидаем 0, комментарии не считаем):;
  grep -nE 'MediaPlayer[[:space:]]*\{' /usr/share/harbour-opencode/qml/harbour-opencode.qml | wc -l
  echo --- чистим qmlcache и перезапускаем:;
  killall harbour-opencode 2>/dev/null
  echo "$PW" | devel-su killall booster-silica-qt5 booster-qt5 2>/dev/null
  sleep 3
  rm -rf ~/.cache/harbour-opencode/qmlcache
  export XDG_RUNTIME_DIR=/run/user/100000 WAYLAND_DISPLAY=../../display/wayland-0 QT_QPA_PLATFORM=wayland EGL_PLATFORM=wayland DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket
  nohup invoker --type=silica-qt5 -s /usr/bin/harbour-opencode >/dev/null 2>&1 &
  sleep 14
  echo PID=\$(pidof harbour-opencode)
  echo --- маркер корня в app.log (ожидаем 'QML send_prompt -> \"\"' или хотя бы padpик):;
  grep -F 'send_prompt ->' ~/.cache/harbour-opencode/app.log | tail -3
" 2>&1