#!/usr/bin/env bash
# Ручная выкладка дашборда на ВМ (то же, что делает deploy-vest-smr.yml).
# Запускать из корня репозитория, нужен ssh-доступ на ВМ.
#
#   deploy/deploy.sh yc-user@1.2.3.4
#   deploy/deploy.sh yc-user@vest-smr.ru /var/www/vest-smr
#
# Первый аргумент — user@host, второй (необязательно) — путь на ВМ.
set -euo pipefail

TARGET="${1:-${DEPLOY_TARGET:-}}"
DEPLOY_PATH="${2:-${DEPLOY_PATH:-/var/www/vest-smr}}"

if [ -z "$TARGET" ]; then
  echo "Укажите цель: deploy/deploy.sh user@host [/var/www/vest-smr]" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root/salon-dashboard"

echo "==> Сборка"
npm ci
npm run build

echo "==> Проверка сборки"
test -f dist/index.html || { echo "dist/index.html не найден" >&2; exit 1; }

echo "==> Выкладка в $TARGET:$DEPLOY_PATH"
ssh "$TARGET" "sudo mkdir -p '$DEPLOY_PATH/data'"
# --exclude=/data/: дайджест новостей на ВМ обновляется отдельно и не входит
# в dist/, иначе --delete стирал бы его при каждом деплое.
# sudo rsync: на ВМ после vm-setup.sh вебрут принадлежит root
rsync -az --delete --exclude=/data/ --rsync-path="sudo rsync" dist/ "$TARGET:$DEPLOY_PATH/"

echo "==> Готово. Проверьте: curl -sI http://${TARGET#*@}/ | head -1"
