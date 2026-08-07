# СпецМонтаж — сайт систем безопасности

Корпоративный сайт компании по продаже, проектированию, монтажу и обслуживанию
систем безопасности: СКУД, видеонаблюдение, охранные системы, оповещение,
освещение, досмотровое оборудование, электросистемы, ограждения.

**Стек:** Django 5.2 · Bootstrap 5 (self-hosted) · PostgreSQL 16 · Nginx · Docker

## Возможности

- Каталог из 8 категорий оборудования с ценами «от …»
- Кейсы «Наши работы» с фото (загрузка через админку)
- Квиз-калькулятор стоимости — заявки помечаются источником для аналитики
- Отзывы клиентов и блок брендов-партнёров
- SEO-блог со статьями
- Форма обратной связи + email-уведомления менеджеру
- Плавающие кнопки WhatsApp/Telegram
- Яндекс.Метрика с целями `contact_lead` / `quiz_lead`
- SEO: sitemap.xml, robots.txt, Open Graph, gzip

Контакты компании, мессенджеры и счётчик Метрики задаются через
env-переменные — без правки кода (см. `.env.example`).

## Запуск в Docker (рекомендуется)

```bash
cp .env.example .env   # заполните SECRET_KEY, пароли, контакты
docker compose up -d --build
```

Сайт: http://localhost • Админка: http://localhost/admin/

Миграции, демо-контент и администратор создаются автоматически при первом
старте. Продакшен-деплой с HTTPS — в [DEPLOY.md](DEPLOY.md).

## Локальная разработка без Docker

```bash
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
cd corp_site
python manage.py migrate
python manage.py loaddata categories demo_content
python manage.py createsuperuser
python manage.py runserver
```

Без переменных PostgreSQL используется SQLite — ничего настраивать не нужно.

## Тесты

```bash
cd corp_site
python manage.py test
```

## Структура

```
corp_site/
├── catalog/            # основное приложение: модели, вьюхи, формы, фикстуры
├── corp_site/          # настройки Django
├── templates/          # шаблоны (base + catalog/*)
└── static/             # стили, локальный Bootstrap (static/vendor)
deploy/                 # nginx-конфиги (HTTP и HTTPS), entrypoint
docker-compose.yml      # базовый стек: nginx + web + postgres
docker-compose.https.yml# HTTPS-оверлей с certbot
DEPLOY.md               # инструкция по продакшен-деплою
```
