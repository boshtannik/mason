#!/bin/sh
# Деплой на телефон Sailfish OS.
#
# Переменные окружения (или дефолты):
#   SFOS_HOST      — хост телефона (по умолчанию 172.28.172.1)
#   SFOS_USER      — пользователь (defaultuser)
#   SFOS_PASS      — пароль SSH
#
# Использование:
#   scripts/deploy.sh                           # бинарь debug aarch64
#   scripts/deploy.sh -r                        # бинарь release aarch64
#   SFOS_PASS=091772 scripts/deploy.sh -r       # с паролем
set -e

HOST="${SFOS_HOST:-172.28.172.1}"
USER="${SFOS_USER:-defaultuser}"
PASS="${SFOS_PASS:-}"
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null"

RELEASE=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -r|--release) RELEASE="--release" ;;
        *) echo "неизвестный флаг: $1"; exit 1 ;;
    esac
    shift
done

CLIENT_DIR="$(dirname "$0")/../opencode-client"
cd "$CLIENT_DIR"

cargo build $RELEASE --target aarch64-unknown-linux-gnu

BIN="target/aarch64-unknown-linux-gnu/${RELEASE:-debug}/opencode-client"
if [ ! -f "$BIN" ]; then
    echo "бинарь не найден: $BIN"; exit 1
fi

echo "==> копируем $BIN на $USER@$HOST"
if [ -n "$PASS" ]; then
    sshpass -p "$PASS" scp $SSH_OPTS "$BIN" "$USER@$HOST:~"
else
    scp $SSH_OPTS "$BIN" "$USER@$HOST:~"
fi

echo "==> проверяем запуск (ver)"
if [ -n "$PASS" ]; then
    sshpass -p "$PASS" ssh $SSH_OPTS "$USER@$HOST" "./opencode-client --version 2>&1 || true"
else
    ssh $SSH_OPTS "$USER@$HOST" "./opencode-client --version 2>&1 || true"
fi

echo
echo "OK: бинарь доставлен в ~/$USER"