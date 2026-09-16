#!/bin/sh
# Сборка opencode-client (кросс-сборка под aarch64 для Sailfish).
#
# Зачем: не держать команды сборки в голове — одна команда на всё.
#
# Использование:
#   scripts/build.sh          # debug, host (проверка)
#   scripts/build.sh -r       # release, host
#   scripts/build.sh -a       # debug, aarch64 (телефон)
#   scripts/build.sh -ar      # release, aarch64
set -e

CLIENT_DIR="$(dirname "$0")/../opencode-client"
cd "$CLIENT_DIR"

TARGET=""
RELEASE=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -a|--aarch64) TARGET="--target aarch64-unknown-linux-gnu" ;;
        -r|--release) RELEASE="--release" ;;
        *) echo "неизвестный флаг: $1"; exit 1 ;;
    esac
    shift
done

if [ -n "$TARGET" ]; then
    echo "==> cargo build $RELEASE $TARGET"
    cargo build $RELEASE $TARGET
    BIN="target/aarch64-unknown-linux-gnu/${RELEASE:-debug}/opencode-client"
else
    echo "==> cargo build $RELEASE"
    cargo build $RELEASE
    BIN="target/${RELEASE:-debug}/opencode-client"
fi

echo
echo "OK: $BIN"
ls -lh "$BIN"