# Деплой СпецМонтаж

Сайт разворачивается через Docker Compose: Nginx + Django (Gunicorn) + PostgreSQL 16.

## Быстрый старт (HTTP, локально или для теста)

```bash
cp .env.example .env
# отредактируйте .env: SECRET_KEY, POSTGRES_PASSWORD, пароль администратора, контакты
docker compose up -d --build
```

Сайт: http://localhost • Админка: http://localhost/admin/

При первом старте автоматически выполняются миграции, загрузка демо-контента
(категории, кейсы, отзывы, статьи, бренды) и создание администратора из
`DJANGO_SUPERUSER_*`-переменных.

## Продакшен на VM (Yandex Cloud и любой другой хостинг)

### 1. Сервер

- Ubuntu 22.04+, от 2 vCPU / 2 ГБ RAM
- Открытые порты: 80, 443
- Установите Docker:
  ```bash
  sudo apt update && sudo apt install -y docker.io docker-compose-v2
  sudo usermod -aG docker $USER && newgrp docker
  ```

### 2. Домен

Направьте A-записи домена (`example.ru` и `www.example.ru`) на IP сервера
и дождитесь обновления DNS (`dig +short example.ru`).

### 3. Код и настройки

```bash
git clone <repo-url> && cd corp_site_full
cp .env.example .env
nano .env
```

Обязательно заполните:

| Переменная | Что указать |
|---|---|
| `SECRET_KEY` | длинная случайная строка |
| `DEBUG` | `False` |
| `ALLOWED_HOSTS` | `example.ru,www.example.ru` |
| `CSRF_TRUSTED_ORIGINS` | `https://example.ru,https://www.example.ru` |
| `POSTGRES_PASSWORD` | надёжный пароль |
| `DJANGO_SUPERUSER_*` | логин/пароль администратора |
| `SITE_PHONE`, `SITE_EMAIL`, `SITE_WHATSAPP`, `SITE_TELEGRAM` | реальные контакты |
| `MANAGER_EMAIL` + `EMAIL_*` | почта для уведомлений о заявках |
| `YANDEX_METRIKA_ID` | номер счётчика Метрики (цели: `contact_lead`, `quiz_lead`) |

### 4. Первый запуск (HTTP)

```bash
docker compose up -d --build
```

Проверьте, что сайт открывается по `http://example.ru`.

### 5. Выпуск HTTPS-сертификата

```bash
# подставьте свой домен и email
docker compose -f docker-compose.yml -f docker-compose.https.yml run --rm \
  --entrypoint certbot certbot certonly --webroot -w /var/www/certbot \
  -d example.ru -d www.example.ru --email admin@example.ru \
  --agree-tos --no-eff-email
```

> Если получите ошибку «nginx не отдаёт challenge» — на шаге 5 стек ещё работает
> с HTTP-конфигом, в котором нет `/.well-known/`. Временный обходной путь:
> остановите nginx (`docker compose stop nginx`) и выпустите сертификат в
> standalone-режиме: `... certonly --standalone -d example.ru ...`

### 6. Включение HTTPS

Замените `example.ru` на ваш домен в `deploy/nginx-https.conf` (3 места), затем:

```bash
docker compose -f docker-compose.yml -f docker-compose.https.yml up -d
```

Оверлей включает: редирект 80→443, TLS 1.2/1.3, HSTS в Django
(`ENABLE_HTTPS=true`), контейнер certbot с автопродлением сертификата каждые
12 часов.

## Обновление сайта

```bash
git pull
docker compose up -d --build          # или с https-оверлеем
```

Статика пересобирается автоматически при старте контейнера, миграции
применяются сами. Загруженные через админку фото хранятся в volume
`media_files` и переживают пересборку.

## Резервное копирование

```bash
# База данных
docker compose exec db pg_dump -U corp_site corp_site > backup_$(date +%F).sql
# Медиа-файлы
docker run --rm -v corp_site_full_media_files:/media -v $(pwd):/backup \
  alpine tar czf /backup/media_$(date +%F).tar.gz -C /media .
```

## Полезное

- Логи: `docker compose logs -f web`
- Django-shell: `docker compose exec web python manage.py shell`
- Заявки с сайта: админка → Заявки (фильтр по источнику: форма / калькулятор)
