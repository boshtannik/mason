#!/bin/sh
# Deploy + (опционально) перезапуск нового harbour-opencode probe.
set -e
PW=091772
echo "=== шаг 1: ставим rpm (деплой уже сделан, если rpm тот же) ==="
echo "$PW" | devel-su sh -c 'rpm -Uvh --force /tmp/h.o.rpm' 2>&1 | tail -1
echo "=== шаг 2: чистим qmlcache ==="
rm -rf /home/defaultuser/.cache/harbour-opencode/qmlcache
echo "=== шаг 3: смотрим запущен ли app ==="
if pgrep -x harbour-opencode >/dev/null; then
  echo "app уже запущен, НЕ перезапускаю"
else
  echo "app не запущен — перезапускаю через restart-probe"
  if [ -x /tmp/restart-probe.sh ]; then
    /tmp/restart-probe.sh
  else
    echo "нет /tmp/restart-probe.sh — не запускаю (только деплой)"
  fi
fi
