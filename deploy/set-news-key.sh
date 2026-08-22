#!/usr/bin/env bash
# Ввод или замена ключа для ежедневных новостей (спросит ключ и сразу
# сгенерирует дайджест):
#
#   curl -fsSL https://raw.githubusercontent.com/ivchenkoIL/corp_site_full/main/deploy/set-news-key.sh | sudo bash
#
set -euo pipefail

ENV_FILE=/etc/vest-smr/news.env
FOLDER="${YC_FOLDER_DEFAULT:-b1gl1jbtedm0f70h0fgb}"

if [ "$(id -u)" -ne 0 ]; then
  echo "Запустите с sudo" >&2
  exit 1
fi

echo "Вставьте API-ключ сервисного аккаунта (начинается с AQVN...)."
printf "Ключ: "
IFS= read -r key < /dev/tty
if [ -z "$key" ]; then
  echo "Пусто — ничего не меняю"
  exit 1
fi

mkdir -p /etc/vest-smr
case "$key" in
  # OAuth-токены новее июня 2026 облако больше не обменивает на IAM,
  # но старые могут работать — различаем по префиксу
  y0*) printf 'YC_OAUTH_TOKEN=%s\nYC_FOLDER_ID=%s\n' "$key" "$FOLDER" > "$ENV_FILE" ;;
  *)   printf 'YC_API_KEY=%s\nYC_FOLDER_ID=%s\n' "$key" "$FOLDER" > "$ENV_FILE" ;;
esac
chmod 600 "$ENV_FILE"

echo "Записано в $ENV_FILE. Генерирую дайджест..."
systemctl start vest-smr-news.service || true
journalctl -u vest-smr-news -n 8 --no-pager
curl -s -o /dev/null -w 'Проверка news.json: HTTP %{http_code} (200 = новости на сайте)\n' \
  http://127.0.0.1/data/news.json || true
