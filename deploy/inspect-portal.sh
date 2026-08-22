#!/usr/bin/env bash
# Инвентаризация ВМ vest-smr.ru: что занимает порт 443 («ВЕСТ Портал»),
# какие сервисы запущены и где лежит код. Только чтение и печать —
# скрипт ничего не меняет, не останавливает и не удаляет.
#
# Запуск (браузерная SSH-консоль Яндекс Клауда → ВМ → «Подключиться»):
#   curl -fsSL https://raw.githubusercontent.com/ivchenkoIL/corp_site_full/main/deploy/inspect-portal.sh | sudo bash
#
# Вывод пришлите в чат Claude — по нему можно будет работать с порталом.
set -euo pipefail

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }

main() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "Запустите с sudo" >&2
    exit 1
  fi

  say "=== nginx: конфиги и владелец порта 443 ==="
  for f in /etc/nginx/sites-enabled/* /etc/nginx/conf.d/*.conf; do
    [ -e "$f" ] || continue
    echo "--- $f"
    grep -E 'server_name|listen|proxy_pass|root ' "$f" | sed 's/^[[:space:]]*/  /'
  done

  say "=== кто слушает 80/443 и соседние порты ==="
  ss -tlnp 2>/dev/null | awk 'NR==1 || /:(80|443|3000|5000|8000|8080|8443|9000)[[:space:]]/' || true

  say "=== запущенные нестандартные сервисы ==="
  systemctl list-units --type=service --state=running --no-pager --no-legend 2>/dev/null \
    | grep -viE 'systemd|dbus|ssh|cron|getty|network|resolv|login|journal|udev|apparmor|polkit|unattended|snapd|multipath|packagekit|rsyslog|qemu|cloud-|chrony|acpid' \
    || true

  say "=== docker (если есть) ==="
  docker ps --format '{{.Names}}  {{.Image}}  {{.Ports}}' 2>/dev/null || echo "  docker не используется"

  say "=== каталоги с кодом (верхний уровень) ==="
  for d in /opt /srv /root /home/*; do
    [ -d "$d" ] || continue
    echo "--- $d:"
    ls -1 "$d" 2>/dev/null | head -12 | sed 's/^/  /'
  done

  say "=== git-репозитории на ВМ ==="
  find /opt /srv /root /home -maxdepth 3 -name .git -type d 2>/dev/null \
    | sed 's|/\.git$||; s/^/  /' || true

  say "Готово. Скопируйте вывод выше и пришлите в чат Claude."
  echo "Важно: у кода портала, живущего только на ВМ, нет резервной копии —"
  echo "по инвентаризации Claude подскажет безопасные команды для бэкапа в Git."
}

# main в самом конце: при обрыве скачивания bash не выполнит обрезанный скрипт
main "$@"
