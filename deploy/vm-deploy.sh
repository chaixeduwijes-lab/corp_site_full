#!/usr/bin/env bash
# Обновление сайта из GitHub на самой ВМ. Вызывается:
#   - таймером vest-smr-deploy.timer каждые 10 минут (без аргументов);
#   - из vm-setup.sh с --force (собрать и выложить безусловно).
# Устанавливается в /usr/local/bin/vest-smr-deploy скриптом vm-setup.sh.
# Без --force собирает только если origin/main ушёл вперёд.
set -euo pipefail

APP_DIR="/opt/corp_site_full"
WEB_ROOT="/var/www/vest-smr"

# защита от наложения запусков: таймер при занятом локе просто уходит
# (следующий тик через 10 минут), а --force ждёт завершения фонового деплоя —
# иначе принудительная пересборка из vm-setup.sh молча превращалась бы в no-op
exec 9>/run/vest-smr-deploy.lock
if [ "${1:-}" = "--force" ]; then
  flock -w 900 9
else
  flock -n 9 || exit 0
fi

cd "$APP_DIR"
git fetch origin main
if [ "${1:-}" != "--force" ] \
  && [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] \
  && [ -f "$WEB_ROOT/index.html" ]; then
  exit 0
fi
git reset --hard origin/main

cd salon-dashboard
npm ci --no-audit --no-fund
npm run build
test -f dist/index.html

mkdir -p "$WEB_ROOT/data"
# --exclude=/data/: там ежедневный news.json, его нет в dist/,
# и без исключения --delete стирал бы дайджест при каждом деплое
rsync -a --delete --exclude=/data/ dist/ "$WEB_ROOT/"
echo "Выложена ревизия $(git -C "$APP_DIR" rev-parse --short HEAD)"
