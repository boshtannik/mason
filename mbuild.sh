#!/bin/sh
# Сборка harbour-opencode через mb2 в Docker-образе sailo-rs (Rust + SFOS SDK).
#
# Использование:
#   sg docker -c "./mbuild.sh"            # debug — быстро, без оптимизаций (для разработки)
#   sg docker -c "./mbuild.sh --release"  # release — с оптимизациями (для дистрибуции)
#
# Кэш cargo-target живёт на хосте (CACHE) и переживает запуски контейнера:
# пересборка идёт только по изменённым крейтам, а не с нуля.
set -e

PROJ=/home/jack/w/sailfish-opencode
OUT=/home/jack/.cache/opencode-sailfish/out
CACHE=/home/jack/.cache/opencode-sailfish/cache
IMAGE=registry.gitlab.com/whisperfish/sailo-rs/rust-aarch64-5.1.0.11:sfos-5-1

MODE=debug
for arg in "$@"; do
    case "$arg" in
        --release) MODE=release ;;
        -h|--help)
            echo "Сборка harbour-opencode (RPM)."
            echo "  ./mbuild.sh            — debug (по умолчанию, быстро)."
            echo "  ./mbuild.sh --release  — со всеми оптимизациями."
            exit 0 ;;
        *) echo "неизвестный флаг: $arg (допустим только --release)"; exit 1 ;;
    esac
done

mkdir -p "$OUT"
mkdir -p "$CACHE/cargo"
chmod 777 "$CACHE" "$CACHE/cargo"
chmod 777 "$OUT"
rm -f "$OUT"/*.rpm

docker run --rm \
  -e MBUILD_MODE="$MODE" \
  -v "$PROJ:/src:ro" \
  -v "$OUT:/out" \
  -v "$CACHE:/home/mersdk/cargo-cache" \
  "$IMAGE" bash -lc '
  set -e
  echo "MBUILD_MODE=$MBUILD_MODE"
  rm -rf ~/opencode-build
  mkdir -p ~/opencode-build
  cd /src
  tar -cf - \
    --exclude=./opencode-client/target \
    --exclude=./.git \
    --exclude=./spike \
    --exclude=./sfos-sysroot \
    --exclude=./docker \
    --exclude=./third_party \
    . | tar -xf - -C ~/opencode-build
  cd ~/opencode-build
  # Зависимости для голоса: whisper.cpp (STT) в $HOME/cargo-cache —
  # sb2 пробрасывает наружу только $HOME, поэтому и cargo-target, и whisper
  # держим там (persistent volume), чтобы пересобирать лишь изменённое.
  if [ ! -d "$HOME/cargo-cache/whisper.cpp" ]; then
    curl -fsSL -o /tmp/whisper.cpp.tar.gz \
      "https://github.com/ggml-org/whisper.cpp/archive/refs/tags/v1.5.4.tar.gz"
    tar -xzf /tmp/whisper.cpp.tar.gz -C "$HOME/cargo-cache"
    mv "$HOME/cargo-cache"/whisper.cpp-1.5.4 "$HOME/cargo-cache/whisper.cpp"
  fi
  mkdir -p third_party
  ln -sfn "$HOME/cargo-cache/whisper.cpp" third_party/whisper.cpp
  # Патч к whisper-cli: добавляет CLI-опцию --audio-ctx (FUTO-подход к скорости).
  if ! grep -qF -- '--audio-ctx' third_party/whisper.cpp/examples/main/main.cpp; then
    patch -p1 -d third_party/whisper.cpp < /src/packaging/whisper-cli-audio-ctx.patch
    echo "whisper-cli: патч --audio-ctx применён"
  else
    echo "whisper-cli: патч --audio-ctx уже применён"
  fi
  ls -d third_party/whisper.cpp
  # Piper (TTS): готовая статическая сборка под aarch64 (эталон) — распаковываем
  # в кэш, дальше spec ставит её в /usr/libexec/harbour-opencode/piper/.
  if [ ! -e "$HOME/cargo-cache/piper-aarch64/piper/piper" ]; then
    curl -fsSL -o /tmp/piper-aarch64.tar.gz \
      "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_aarch64.tar.gz"
    rm -rf "$HOME/cargo-cache/piper-aarch64"
    mkdir -p "$HOME/cargo-cache/piper-aarch64"
    tar -xzf /tmp/piper-aarch64.tar.gz -C "$HOME/cargo-cache/piper-aarch64"
  fi
  find "$HOME/cargo-cache/piper-aarch64" -maxdepth 2 -name piper -type f
  ls -l "$HOME/cargo-cache/piper-aarch64/piper"
  mb2 -n -t SailfishOS-5.1.0.11-aarch64 --no-snapshot=force build
  echo "=== RPM ($MBUILD_MODE) ==="
  ls -l RPMS/*.rpm
  cp -v RPMS/*.rpm /out/
'

cp -v "$OUT"/*.rpm "$PROJ"