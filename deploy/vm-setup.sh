#!/usr/bin/env bash
# Полная настройка ВМ для vest-smr.ru одной командой (Ubuntu/Debian):
#
#   curl -fsSL https://raw.githubusercontent.com/ivchenkoIL/corp_site_full/main/deploy/vm-setup.sh | sudo bash
#
# Что делает: nginx + Node.js, клонирует репозиторий, собирает дашборд,
# включает автообновление сайта из GitHub (каждые 10 минут) и ежедневную
# генерацию новостей YandexGPT (06:00 МСК) прямо на ВМ — GitHub-секреты
# не нужны. Скрипт идемпотентный: повторный запуск безопасен, подхватывает
# новые версии deploy-скриптов и юнитов и не трогает конфиг nginx,
# если его уже переписал certbot.
set -euo pipefail

REPO_URL="https://github.com/ivchenkoIL/corp_site_full.git"
APP_DIR="/opt/corp_site_full"
WEB_ROOT="/var/www/vest-smr"
ENV_FILE="/etc/vest-smr/news.env"
LOCK_FILE="/run/vest-smr-deploy.lock"
# на свежей ВМ apt-лок первые минуты держат cloud-init/unattended-upgrades —
# ждём до 5 минут вместо мгновенного падения
APT="apt-get -o DPkg::Lock::Timeout=300"

main() {
  if [ "$(id -u)" -ne 0 ]; then
    echo "Нужны права root: перезапустите команду с sudo" >&2
    exit 1
  fi

  echo "==> Пакеты"
  export DEBIAN_FRONTEND=noninteractive
  $APT update -y
  $APT install -y nginx git rsync curl ca-certificates certbot python3-certbot-nginx

  echo "==> Node.js"
  local need_node=1 major
  if command -v node >/dev/null 2>&1; then
    major="$(node -v | sed 's/^v//; s/\..*//')"
    [ "$major" -ge 20 ] && need_node=0
  fi
  if [ "$need_node" -eq 1 ]; then
    # старый node из репозиториев Ubuntu конфликтует с пакетом NodeSource
    $APT remove -y --purge nodejs npm libnode-dev 2>/dev/null || true
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
    $APT install -y nodejs
  fi
  echo "node $(node -v), npm $(npm -v)"

  echo "==> Код из GitHub"
  if [ -d "$APP_DIR/.git" ]; then
    # под тем же локом, что и деплой-таймер: reset посреди чужой сборки
    # дал бы вебрут из смеси двух ревизий
    flock -w 600 "$LOCK_FILE" -c \
      "git -C '$APP_DIR' fetch origin main && git -C '$APP_DIR' reset --hard origin/main"
  else
    git clone --branch main "$REPO_URL" "$APP_DIR"
  fi

  echo "==> Deploy-скрипт и systemd-юниты"
  install -m 755 "$APP_DIR/deploy/vm-deploy.sh" /usr/local/bin/vest-smr-deploy
  install -m 644 "$APP_DIR"/deploy/systemd/vest-smr-*.service \
                 "$APP_DIR"/deploy/systemd/vest-smr-*.timer /etc/systemd/system/
  systemctl daemon-reload

  echo "==> Сборка и выкладка сайта"
  mkdir -p "$WEB_ROOT/data"
  /usr/local/bin/vest-smr-deploy --force

  echo "==> nginx"
  local conf_target
  if [ -d /etc/nginx/sites-available ]; then
    conf_target=/etc/nginx/sites-available/vest-smr.conf
  else
    conf_target=/etc/nginx/conf.d/vest-smr.conf
  fi
  if [ -f "$conf_target" ] && grep -q "managed by Certbot" "$conf_target"; then
    # certbot уже вписал сюда HTTPS — перезапись вернула бы сайт на голый
    # HTTP и молча погасила бы 443
    echo "конфиг уже настроен certbot — не перезаписываю ($conf_target)"
  else
    install -m 644 "$APP_DIR/deploy/nginx-vest-smr.conf" "$conf_target"
  fi
  if [ -d /etc/nginx/sites-enabled ]; then
    ln -sf "$conf_target" /etc/nginx/sites-enabled/vest-smr.conf
    rm -f /etc/nginx/sites-enabled/default
  fi
  nginx -t
  systemctl enable --now nginx
  systemctl reload nginx

  echo "==> Файл окружения для новостей"
  mkdir -p /etc/vest-smr
  touch "$ENV_FILE"
  chmod 600 "$ENV_FILE"

  echo "==> Таймеры: автообновление сайта + ежедневные новости"
  systemctl enable --now vest-smr-deploy.timer vest-smr-news.timer

  if grep -qs 'YC_' "$ENV_FILE"; then
    echo "==> Токен уже вписан — генерирую первый дайджест"
    systemctl start vest-smr-news.service \
      || echo "!! Генерация не удалась, смотрите: journalctl -u vest-smr-news -n 50"
  fi

  local IP
  IP="$(curl -fsS --max-time 5 https://ifconfig.me 2>/dev/null || hostname -I | awk '{print $1}')"
  cat <<EOF

============================================================
Готово! Сайт уже отвечает: http://$IP/

Осталось:
1) Токен для ежедневных новостей (один раз):
     sudo tee $ENV_FILE >/dev/null <<'ENV'
YC_OAUTH_TOKEN=вставьте_сюда_токен_y0__...
YC_FOLDER_ID=вставьте_сюда_id_каталога
ENV
     sudo systemctl start vest-smr-news.service   # первый дайджест сразу
2) DNS: A-записи vest-smr.ru и www.vest-smr.ru -> $IP
3) HTTPS после смены DNS:
     sudo certbot --nginx -d vest-smr.ru -d www.vest-smr.ru

Дальше всё само: сайт обновляется из GitHub каждые 10 минут,
новости генерируются ежедневно в 06:00 МСК.
Диагностика: journalctl -u vest-smr-deploy -n 50
             journalctl -u vest-smr-news -n 50
============================================================
EOF
}

# main в самом конце: при обрыве curl на середине скачивания bash не выполнит
# случайный префикс скрипта под root
main "$@"
