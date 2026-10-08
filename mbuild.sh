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
  # opencode CLI: бандлим arm64-бинарь в RPM (/usr/libexec/harbour-opencode/opencode),
  # чтобы клиент не зависел от отдельно установленного opencode и не конфликтовал
  # с ним по PATH. Версия 1.18.30 — единственная, реально держащая `opencode serve`
  # на arm64 (проверена на телефоне). 1.18.31/1.18.32 arm64 — Bun-сборки: `--version`
  # без аргументов выводит 1.3.14 (Bun), а `serve` падает с "Script not found serve".
  # НЕ «улучшать» до новее, пока в релизах не вернут нативный arm64 бинарь.
  OC_VERSION="1.18.30"
  # SHA-256 эталонного бинаря 1.18.30 (linux-arm64, проверен на телефоне).
  # Сверяем не только версию, но и сам файл: защита от подмены кэша
  # (например, Bun-бинарём с уже подходящим opencode.version), из-за которой
  # в RPM мог уехать не тот открытод.
  OC_SHA256="01edb5839aa10d5b09133fedcb335a062ecad6e82552933bb14f71756f2b296b"
  OC_CLI="$HOME/cargo-cache/opencode-cli"
  # set -e: sha256sum отсутствующего файла умирает с 1 — это не ошибка,
  # а признак «надо скачать» (проверяется в if ниже).
  OC_SHA_HAVE=$(sha256sum "$OC_CLI/opencode" 2>/dev/null || true)
  OC_SHA_HAVE=${OC_SHA_HAVE%% *}
  if [ ! -e "$OC_CLI/opencode" ] \
     || [ "$(cat "$OC_CLI/opencode.version" 2>/dev/null)" != "$OC_VERSION" ] \
     || [ "$OC_SHA_HAVE" != "$OC_SHA256" ]; then
    curl -fsSL -o /tmp/opencode-linux-arm64.tar.gz \
      "https://github.com/anomalyco/opencode/releases/download/v${OC_VERSION}/opencode-linux-arm64.tar.gz"
    rm -rf "$OC_CLI"
    mkdir -p "$OC_CLI"
    tar -xzf /tmp/opencode-linux-arm64.tar.gz -C "$OC_CLI"
    printf '%s\n' "$OC_VERSION" > "$OC_CLI/opencode.version"
  fi
  echo "opencode CLI version: $(cat "$OC_CLI/opencode.version")"
  OC_SHA_NOW=$(sha256sum "$OC_CLI/opencode")
  echo "opencode CLI sha256: ${OC_SHA_NOW%% *}"
  find "$OC_CLI" -maxdepth 1 -name opencode -type f
  ls -l "$OC_CLI/opencode"
  mb2 -n -t SailfishOS-5.1.0.11-aarch64 --no-snapshot=force build
  echo "=== RPM ($MBUILD_MODE) ==="
  ls -l RPMS/*.rpm
  # Верификация: внутрь RPM обязан попасть ТОЧНО пинированный opencode.
  # В прошлом brp-strip ужинал Bun-бинарь и в RPM уезжал бинарь без "serve"
  # (1.3.14 вместо 1.18.30), хотя в buildroot клали правильный файл.
  OC_RPM="$(pwd)/$(ls RPMS/*.rpm | head -1)"
  echo "проверяю opencode внутри RPM: $OC_RPM"
  rm -rf /tmp/ocverify && mkdir -p /tmp/ocverify && cd /tmp/ocverify
  rpm2cpio "$OC_RPM" | cpio -id --quiet "./usr/libexec/harbour-opencode/opencode" >/dev/null 2>&1 || true
  if [ ! -f usr/libexec/harbour-opencode/opencode ]; then
    echo "ОШИБКА: не удалось извлечь opencode из RPM (rpm2cpio/cpio)"
    exit 1
  fi
  OC_VERIFY="$(sha256sum usr/libexec/harbour-opencode/opencode | awk "{print \$1}")"
  echo "opencode в RPM sha256: $OC_VERIFY"
  if [ "$OC_VERIFY" != "$OC_SHA256" ]; then
    echo "ОШИБКА: в RPM уехал НЕ пинированный opencode."
    echo "ожидался: $OC_SHA256"
    echo "реально:  $OC_VERIFY"
    echo "похоже, rpmbuild снова урезал бинарь — проверь %global __strip / debug_package в spec."
    exit 1
  fi
  echo "opencode в RPM: совпадает с пином — OK"
  cd - >/dev/null
  cp -v RPMS/*.rpm /out/
'

cp -v "$OUT"/*.rpm "$PROJ"