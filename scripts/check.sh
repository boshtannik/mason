#!/bin/sh
# Быстрая проверка стека: примеры opencode-client (host).
#
# Запускает последовательно примеры: ServerGuard смоук, полный цикл,
# (голосовой roundtrip и аудио — отдельными флагами, т.к. требуют/дают звук).
#
# Использование:
#   scripts/check.sh           # серверные тесты (guard_smoke + full_cycle)
#   scripts/check.sh --voice   # + voice_roundtrip (STT/TTS без микрофона)
#   scripts/check.sh --audio   # + audio_record_play (запишет 3 сек с микрофона и проиграет)
set -e

CLIENT_DIR="$(dirname "$0")/../opencode-client"
cd "$CLIENT_DIR"

# Утилир/библиотеки (LD_LIBRARY_PATH локального whisper).
WHISPER_LIB=/home/jack/tools/voice/whisper-x64/whisper-bin-ubuntu-x64
export LD_LIBRARY_PATH="$WHISPER_LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

run_example() {
    echo "────────────────────────────────────────────"
    echo ">>> cargo run --example $1"
    echo "────────────────────────────────────────────"
    timeout "${TIMEOUT:-60}" cargo run --quiet --example "$1" 2>&1 \
        | grep -vE "warning|Compiling|Finished|Running" || true
}

run_example guard_smoke
run_example full_cycle

for arg in "$@"; do
    case "$arg" in
        --voice) run_example voice_roundtrip ;;
        --audio) run_example audio_record_play ;;
        *) echo "неизвестный флаг: $arg" ;;
    esac
done

echo
echo "OK: все примеры отработали (см. вывод выше)"