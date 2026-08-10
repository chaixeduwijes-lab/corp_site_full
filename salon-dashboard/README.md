# salon-dashboard

Фронтенд vest-smr.ru: дашборд мониторинга ИИ-индустрии (корень сайта) и аналитика салона (раздел `/salon`).
React 19 + TypeScript + Vite, стили — Tailwind, графики — Recharts, роутинг — wouter.

## Команды

```bash
npm ci          # установка зависимостей
npm run dev     # dev-сервер с HMR
npm run build   # tsc -b + vite build → dist/
npm run lint    # oxlint
npm run preview # локальный просмотр собранного dist/
```

## Структура

```
src/
  App.tsx              маршруты (все страницы — lazy)
  components/Layout.tsx шапка, навигация, футер
  components/ui/       card, badge, button
  data/report.ts       данные отчёта по ИИ: метрики, модели, цены, рекомендации
  data/salon.json      выгрузка salonbackup (1127 визитов), ~388 КБ
  hooks/useNews.ts     подгрузка /data/news.json с фолбэком на вшитый дайджест
  lib/format.ts        формат чисел и дат, без зависимостей от данных
  lib/data.ts          датасет салона: entries, expenses, masters, прайс
  pages/               страницы разделов ИИ и салона
```

## Разбиение бандла

Страницы подключены через `lazy()`, поэтому Recharts и `salon.json` не попадают
в стартовую загрузку главной страницы: `/` тянет ~228 КБ (~75 КБ gzip), выгрузка
салона (~227 КБ) грузится только при переходе в `/salon`.

Чтобы это не сломалось:

- **не импортируйте `lib/data` из `Layout` или других модулей, которые грузятся всегда** —
  через него в основной чанк попадёт `salon.json`. Для формата чисел и дат
  используйте `lib/format` (`lib/data` реэкспортит его для страниц салона);
- новые страницы добавляйте в `App.tsx` тоже через `lazy(() => import(...))`.

После сборки проверяйте, что в выводе `npm run build` нет предупреждения
о чанках больше 500 КБ, кроме вендорного чанка Recharts.

## Данные новостей

`useNews` в рантайме запрашивает `/data/news.json`; если файла нет — показывает
дайджест, вшитый в `data/report.ts`. Файл публикуется на сервер отдельно
воркфлоу `.github/workflows/update-news.yml` (см. `deploy/README.md`), поэтому
обновление новостей не требует пересборки фронтенда. В nginx `/data/news.json`
и `index.html` отдаются с `Cache-Control: no-cache`, а `/assets/` — с
долгим кешем, так как имена файлов содержат хеш.

## Деплой

Сборка выкладывается в `/var/www/vest-smr` на VM воркфлоу
`.github/workflows/deploy-vest-smr.yml`, конфиг nginx — `deploy/nginx-vest-smr.conf`.
Вариант в контейнере: `Dockerfile` + `nginx.container.conf` (порт 8080).
