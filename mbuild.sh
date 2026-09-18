#!/bin/sh
# Сборка harbour-opencode через mb2 в Docker-образе sailo-rs (Rust 1.89 + SFOS SDK).
# Использование: sg docker -c "./mbuild.sh"
set -e

PROJ=/home/jack/w/sailfish-opencode
OUT=/tmp/opencode/sailfish-out
IMAGE=registry.gitlab.com/whisperfish/sailo-rs/rust-aarch64-5.1.0.11:sfos-5-1

mkdir -p "$OUT"
chmod 777 "$OUT"
rm -f "$OUT"/*.rpm

docker run --rm -v "$PROJ:/src:ro" -v "$OUT:/out" "$IMAGE" bash -lc '
  set -e
  rm -rf ~/opencode-build
  mkdir -p ~/opencode-build
  cd /src
  tar -cf - \
    --exclude=./opencode-client/target \
    --exclude=./.git \
    --exclude=./spike \
    --exclude=./sfos-sysroot \
    --exclude=./docker \
    . | tar -xf - -C ~/opencode-build
  cd ~/opencode-build
  # Зависимости для голоса: whisper.cpp (STT). Cкачиваем вне mb2 — у целевого
  # gitHub/HF доступа обычно нет.
  mkdir -p third_party
  curl -fsSL -o /tmp/whisper.cpp.tar.gz \
    "https://github.com/ggml-org/whisper.cpp/archive/refs/tags/v1.5.4.tar.gz"
  tar -xzf /tmp/whisper.cpp.tar.gz -C third_party
  mv third_party/whisper.cpp-1.5.4 third_party/whisper.cpp
  ls -d third_party/whisper.cpp
  mb2 -n -t SailfishOS-5.1.0.11-aarch64 --no-snapshot=force build
  echo "=== RPM ==="
  ls -l RPMS/*.rpm
  cp -v RPMS/*.rpm /out/
'

cp -v "$OUT"/*.rpm "$PROJ"
