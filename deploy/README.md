# Размещение дашборда на ВМ в Яндекс Клауде (vest-smr.ru)

Дашборд — статический сайт (папка `salon-dashboard/dist` после сборки).
Для его работы на ВМ достаточно nginx.

## Самый быстрый путь: одна команда на ВМ

Откройте браузерную SSH-консоль ВМ (консоль Яндекс Клауда → ВМ →
«Подключиться») и выполните:

```bash
curl -fsSL https://raw.githubusercontent.com/ivchenkoIL/corp_site_full/main/deploy/vm-setup.sh | sudo bash
```

Скрипт сам ставит nginx и Node.js, клонирует репозиторий, собирает сайт и
включает два systemd-таймера:

- `vest-smr-deploy.timer` — каждые 10 минут проверяет GitHub и пересобирает
  сайт, если в `main` появились изменения (деплой без секретов и без SSH);
- `vest-smr-news.timer` — ежедневно в 06:00 МСК генерирует дайджест
  YandexGPT в `/var/www/vest-smr/data/news.json`.

После него останется (скрипт напечатает это же с вашим IP):

1. вписать токен и каталог в `/etc/vest-smr/news.env` и запустить
   `sudo systemctl start vest-smr-news.service` — первый дайджест;
2. направить A-записи `vest-smr.ru` и `www` на IP ВМ;
3. `sudo certbot --nginx -d vest-smr.ru -d www.vest-smr.ru` — HTTPS.

Повторный запуск скрипта безопасен: он же обновляет deploy-скрипты и юниты.
Разделы ниже — тот же результат вручную или через GitHub Actions, если
вариант с таймерами на ВМ не подходит.

## 1. Разовая настройка ВМ (выполняется один раз)

Подключитесь к ВМ по SSH и выполните:

```bash
sudo apt update && sudo apt install -y nginx certbot python3-certbot-nginx
sudo mkdir -p /var/www/vest-smr
sudo chown "$USER" /var/www/vest-smr

# конфиг nginx из этого репозитория
sudo cp deploy/nginx-vest-smr.conf /etc/nginx/sites-available/vest-smr.conf
sudo ln -s /etc/nginx/sites-available/vest-smr.conf /etc/nginx/sites-enabled/
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t && sudo systemctl reload nginx
```

DNS: у регистратора домена vest-smr.ru создайте A-запись на публичный IP ВМ
(и A-запись для `www`, если нужна). В Яндекс Клауде убедитесь, что у ВМ
публичный IP статический, а в группе безопасности открыты порты 80 и 443.

HTTPS (после того как DNS начал указывать на ВМ):

```bash
sudo certbot --nginx -d vest-smr.ru -d www.vest-smr.ru
```

## 2. Автодеплой из GitHub (рекомендуется)

Workflow `.github/workflows/deploy-vest-smr.yml` при каждом пуше в `main`
собирает дашборд и выкладывает его на ВМ по SSH.

Создайте на ВМ ключ для деплоя и добавьте секреты в репозиторий
(Settings → Secrets and variables → Actions):

```bash
# на своём компьютере или на ВМ
ssh-keygen -t ed25519 -f deploy_key -N "" -C "github-deploy"
# публичную часть — на ВМ:
cat deploy_key.pub >> ~/.ssh/authorized_keys
```

| Секрет | Значение |
|---|---|
| `DEPLOY_HOST` | публичный IP ВМ (или vest-smr.ru после настройки DNS) |
| `DEPLOY_USER` | пользователь на ВМ (например, `ubuntu` или `yc-user`) |
| `DEPLOY_SSH_KEY` | содержимое файла `deploy_key` (приватный ключ целиком) |
| `DEPLOY_PATH` | (необязательно) путь на ВМ, по умолчанию `/var/www/vest-smr` |

После добавления секретов запустите workflow вручную
(Actions → «Deploy dashboard to vest-smr.ru» → Run workflow) или сделайте
любой пуш в `main`.

## 3. Ручной деплой (без GitHub Actions)

Скрипт делает то же, что workflow — собирает и заливает по rsync:

```bash
deploy/deploy.sh yc-user@<IP-ВМ>              # путь по умолчанию /var/www/vest-smr
deploy/deploy.sh yc-user@vest-smr.ru /var/www/vest-smr
```

То же вручную:

```bash
cd salon-dashboard
npm ci && npm run build
rsync -az --delete --exclude=/data/ dist/ user@<IP-ВМ>:/var/www/vest-smr/
```

`--exclude=/data/` обязателен: в `dist/` нет каталога `data/`, а на ВМ там лежит
ежедневный `news.json`, и без исключения `--delete` стирал бы его при каждом деплое.

## Обновление данных

Данные зашиты в сборку из `salon-dashboard/src/data/salon.json`.
Чтобы обновить цифры, замените этот файл новой выгрузкой salonbackup
и запушьте в `main` — автодеплой пересоберёт и выложит сайт.

## Вариант: контейнером (аналог Cloud Run из инструкции)

В `salon-dashboard/` есть `Dockerfile` (сборка + nginx, порт 8080).

**На ВМ с Docker:**

```bash
cd salon-dashboard
docker build -t salon-dashboard .
docker run -d --restart unless-stopped -p 80:8080 salon-dashboard
```

**Или Yandex Serverless Containers (без ВМ, аналог Cloud Run):**

```bash
yc container registry create --name salon
docker build -t cr.yandex/<registry-id>/salon-dashboard:latest salon-dashboard
docker push cr.yandex/<registry-id>/salon-dashboard:latest
yc serverless container create --name salon-dashboard
yc serverless container revision deploy \
  --container-name salon-dashboard \
  --image cr.yandex/<registry-id>/salon-dashboard:latest \
  --execution-timeout 30s --memory 256MB --cores 1
yc serverless container allow-unauthenticated-invoke --name salon-dashboard
```

Домен vest-smr.ru к Serverless Containers подключается через API Gateway
или остаётся на ВМ с nginx — тогда используйте первый вариант.

## Ежедневные новости через YandexGPT

Workflow `.github/workflows/update-news.yml` каждый день в 06:00 МСК:
1) забирает свежие заголовки из RSS-лент об ИИ;
2) просит YandexGPT собрать русскоязычный дайджест (`scripts/update-news.mjs`),
   ссылки-источники проверяются по реальным лентам — выдуманные отбрасываются;
3) выкладывает `data/news.json` на ВМ. Сайт подхватывает файл без пересборки
   (страница «Новости» читает `/data/news.json`, при его отсутствии показывает
   встроенный выпуск от 8 августа).

Если новости генерируются на самой ВМ (см. «одна команда» выше) — этот
workflow не нужен. Для варианта через GitHub нужны секреты:

| Секрет | Как получить |
|---|---|
| `YC_FOLDER_ID` | консоль Яндекс Клауда → ваш каталог → ID каталога |
| `YC_API_KEY` | создать сервисный аккаунт с ролью `ai.languageModels.user`, затем «Создать API-ключ» |
| `YC_OAUTH_TOKEN` | альтернатива Api-Key: OAuth-токен Яндекс ID (`y0_...`), скрипт сам обменяет его на IAM-токен |

Плюс уже описанные `DEPLOY_HOST` / `DEPLOY_USER` / `DEPLOY_SSH_KEY`.
Запустить вручную: Actions → «Daily news update via YandexGPT» → Run workflow.

Альтернатива без GitHub: `vm-setup.sh` уже ставит systemd-таймер
`vest-smr-news.timer` (06:00 МСК с учётом часового пояса). Если нужен именно
cron, помните, что облачные ВМ живут в UTC:

```cron
# 03:00 UTC = 06:00 МСК
0 3 * * * cd /opt/corp_site_full && YC_API_KEY=... YC_FOLDER_ID=... \
  node scripts/update-news.mjs /var/www/vest-smr/data/news.json
```

Примечание к GitHub-вариантам деплоя и публикации новостей: они выполняют
на ВМ `sudo rsync`/`sudo tee`, поэтому у `DEPLOY_USER` должен быть sudo без
пароля (для стандартных пользователей облачных образов это так).
