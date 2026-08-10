# Размещение дашборда на ВМ в Яндекс Клауде (vest-smr.ru)

Дашборд — статический сайт (папка `salon-dashboard/dist` после сборки).
Для его работы на ВМ достаточно nginx.

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

```bash
cd salon-dashboard
npm ci && npm run build
rsync -az --delete dist/ user@<IP-ВМ>:/var/www/vest-smr/
```

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

Нужные секреты в репозитории:

| Секрет | Как получить |
|---|---|
| `YC_FOLDER_ID` | консоль Яндекс Клауда → ваш каталог → ID каталога |
| `YC_API_KEY` | создать сервисный аккаунт с ролью `ai.languageModels.user`, затем «Создать API-ключ» |

Плюс уже описанные `DEPLOY_HOST` / `DEPLOY_USER` / `DEPLOY_SSH_KEY`.
Запустить вручную: Actions → «Daily news update via YandexGPT» → Run workflow.

Альтернатива без GitHub: тот же скрипт кроном прямо на ВМ:

```cron
0 6 * * * cd /opt/corp_site_full && YC_API_KEY=... YC_FOLDER_ID=... \
  node scripts/update-news.mjs /var/www/vest-smr/data/news.json
```
