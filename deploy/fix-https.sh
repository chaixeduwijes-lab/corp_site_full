#!/usr/bin/env bash
# УСТАРЕЛО (20.08.2026): порт 443 на vest-smr.ru теперь штатно занимает
# «ВЕСТ Портал» — отдельное приложение, развёрнутое на той же ВМ. Этот
# скрипт отключил бы его nginx-конфиг и отдал 443 дашборду, СЛОМАВ портал.
# Оставлен только на случай сознательного отката: запуск требует FORCE=1.
#
#   FORCE=1 bash fix-https.sh   # осознанный запуск, отберёт 443 у портала
#
set -euo pipefail

if [ "${FORCE:-0}" != "1" ]; then
  echo "ОСТАНОВЛЕНО: на 443 сейчас работает «ВЕСТ Портал», этот скрипт его сломает." >&2
  echo "Если вы точно хотите вернуть 443 дашборду: FORCE=1 bash fix-https.sh" >&2
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "Запустите с sudo" >&2
  exit 1
fi

echo "==> Что сейчас включено в nginx:"
ls -l /etc/nginx/sites-enabled/ 2>/dev/null || true
ls -l /etc/nginx/conf.d/ 2>/dev/null || true

echo "==> Отключаю все сайты, кроме vest-smr.conf"
shopt -s nullglob
for f in /etc/nginx/sites-enabled/*; do
  b="$(basename "$f")"
  if [ "$b" != "vest-smr.conf" ]; then
    echo "  - отключаю $b (файл остаётся в sites-available)"
    rm -f "$f"
  fi
done
for f in /etc/nginx/conf.d/*.conf; do
  b="$(basename "$f")"
  if [ "$b" != "vest-smr.conf" ] && grep -qsE "listen[[:space:]]+443|vest-smr" "$f"; then
    echo "  - отключаю conf.d/$b -> $b.disabled"
    mv "$f" "$f.disabled"
  fi
done

echo "==> Вешаю сертификат на конфиг нового сайта (+редирект http -> https)"
certbot install --nginx --cert-name vest-smr.ru-0001 --redirect -n \
  || certbot --nginx -n --agree-tos --reinstall --redirect -d vest-smr.ru -d www.vest-smr.ru \
  || certbot --nginx -n --agree-tos --redirect -d vest-smr.ru -d www.vest-smr.ru

nginx -t
systemctl reload nginx

echo "==> Проверка (оба кода должны быть 200):"
curl -sk -o /dev/null -w '  https://vest-smr.ru/               -> HTTP %{http_code}\n' https://vest-smr.ru/ || true
curl -sk -o /dev/null -w '  https://vest-smr.ru/data/news.json -> HTTP %{http_code}\n' https://vest-smr.ru/data/news.json || true
