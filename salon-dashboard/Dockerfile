# Контейнер с дашбордом — аналог рецепта из инструкции (Cloud Run),
# адаптирован для Яндекс Клауда: собираем статику и отдаём её nginx,
# а не dev-сервером `npm run preview`.
FROM node:22-alpine AS build
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=build /app/dist /usr/share/nginx/html
COPY nginx.container.conf /etc/nginx/conf.d/default.conf
# 8080 — порт по умолчанию для Yandex Serverless Containers
EXPOSE 8080
