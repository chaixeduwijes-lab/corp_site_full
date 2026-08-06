#!/bin/sh
set -e

echo "Applying database migrations..."
python manage.py migrate --no-input

echo "Loading initial data..."
python manage.py loaddata categories || echo "WARNING: fixture load failed, continuing"

if [ -n "$DJANGO_SUPERUSER_USERNAME" ] && [ -n "$DJANGO_SUPERUSER_PASSWORD" ]; then
    echo "Ensuring superuser exists..."
    python manage.py createsuperuser --no-input 2>/dev/null || echo "Superuser already exists"
fi

echo "Starting server..."
exec "$@"
