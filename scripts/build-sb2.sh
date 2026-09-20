#!/usr/bin/env bash
#
# Build a Rust project with the official Sailfish OS SDK toolchain
# (sb2 + aarch64-meego-linux-gnu-*), using the sailfish-rust image
# (platform SDK + Rust from nas.rubdos.be), modelled on whisperfish/sailo-rs.
#
# Env:
#   PROJECT       path relative to repo root (default: spike/qml-qt56)
#   SFOS_VERSION  default: 5.1.0.11
#   SFOS_ARCH     default: aarch64
#   IMAGE         default: sailfish-rust:$SFOS_VERSION-$SFOS_ARCH
#
# Requires docker access; if the current shell lacks the docker group, use:
#   sg docker -c "./scripts/build-sb2.sh"
#
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT="${PROJECT:-spike/qml-qt56}"
SFOS_VERSION="${SFOS_VERSION:-5.1.0.11}"
SFOS_ARCH="${SFOS_ARCH:-aarch64}"
IMAGE="${IMAGE:-sailfish-rust:${SFOS_VERSION}-${SFOS_ARCH}}"
TARGET="SailfishOS-${SFOS_VERSION}-${SFOS_ARCH}"
RUST_TRIPLE="aarch64-unknown-linux-gnu"

PROJ_DIR="$REPO/$PROJECT"
CFG="$PROJ_DIR/.cargo/config.toml"
SAILO_CFG="$PROJ_DIR/.cargo/config.sailo.toml"
BAK="$CFG.host.bak"

if [ ! -f "$SAILO_CFG" ]; then
  echo "missing $SAILO_CFG" >&2
  exit 1
fi

restore() { if [ -f "$BAK" ]; then mv "$BAK" "$CFG"; fi; }
trap restore EXIT

# Swap in the sb2/meego cargo config for the duration of the build.
if [ -f "$CFG" ]; then mv "$CFG" "$BAK"; fi
cp "$SAILO_CFG" "$CFG"

echo ">> image   : $IMAGE"
echo ">> target  : $TARGET"
echo ">> project : $PROJECT"

docker run --rm -v "$REPO:/home/mersdk/share" "$IMAGE" \
  sb2 -t "$TARGET" bash -c "
    set -e
    export SB2_RUST_TARGET_TRIPLE=$RUST_TRIPLE
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-meego-linux-gnu-gcc
    export CC_aarch64_unknown_linux_gnu=aarch64-meego-linux-gnu-gcc
    export CXX_aarch64_unknown_linux_gnu=aarch64-meego-linux-gnu-g++
    export AR_aarch64_unknown_linux_gnu=aarch64-meego-linux-gnu-ar
    export QMAKE=/usr/bin/qmake
    export PKG_CONFIG_ALLOW_CROSS_aarch64_unknown_linux_gnu=1
    cd /home/mersdk/share/$PROJECT
    cargo build --release
  "
