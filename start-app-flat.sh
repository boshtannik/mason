#!/bin/sh
# Запуск harbour-opencode в UI-сессии телефона (для dev-проверок).
# Пароль devel-su: 091772
export XDG_RUNTIME_DIR=/run/user/100000
export WAYLAND_DISPLAY=display/wayland-0
export QT_QPA_PLATFORM=wayland
export EGL_PLATFORM=wayland
export EGLFS_PLATFORM=wayland
export QT_QUICK_CONTROLS_STYLE=Silica
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket
echo "091772" | /bin/devel-su sh -c 'XDG_RUNTIME_DIR=/run/user/100000 WAYLAND_DISPLAY=display/wayland-0 QT_QPA_PLATFORM=wayland EGL_PLATFORM=wayland EGLFS_PLATFORM=wayland DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/100000/dbus/user_bus_socket TF_YA_T..=1; nohup /usr/bin/invoker --type=silica-qt5 -s /usr/bin/harbour-opencode >/dev/null 2>&1 &'
