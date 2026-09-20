#!/usr/bin/env bash
set -uo pipefail
REPO=/home/jack/w/sailfish-opencode
LOG=/tmp/sf-spike-build.log
: > "$LOG"
exec >> "$LOG" 2>&1
echo "=== started $(date) ==="
sg docker -c "docker run --rm \
  -v /home/jack/w/sailfish-opencode:/home/mersdk/share \
  -v /home/jack/.whisperfish-docker/target-chown-skip:/dev/null \
  sailfish-rust:5.1.0.11-aarch64 \
  bash -c '
    set -o pipefail
    T=SailfishOS-5.1.0.11-aarch64
    for f in /srv/mer/toolings/SailfishOS-5.1.0.11/usr/bin/{rustc,cargo} \
             /srv/mer/targets/SailfishOS-5.1.0.11-aarch64/usr/bin/{rustc,cargo}; do
      echo \"tool: \$f\"
    done
    sb2 -t \$T rustc --version
    sb2 -t \$T cargo --version
    echo \">>> cargo build (accel, may take a while) @ \$(date)\"
    cd /home/mersdk/share/spike/qml-qt56
    cargo build --release
    echo \"=== build rc=\$? @ \$(date) ===\"
    file target/aarch64-unknown-linux-gnu/release/spike-qt56
  '" 2>&1
echo "=== done $(date) rc=$? ==="
sg docker -c "sudo kill \$(pgrep -f 'bin/opencode serve' | head -1) 2>/dev/null" 2>/dev/null
