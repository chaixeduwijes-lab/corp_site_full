FROM python:3.11-slim

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

WORKDIR /app

# Поддержка корпоративных / прокси CA-сертификатов: положите .crt в deploy/certs/
COPY deploy/certs/ /usr/local/share/extra-ca/
RUN for f in /usr/local/share/extra-ca/*.crt; do \
        [ -f "$f" ] || continue; \
        cat "$f" >> /etc/ssl/certs/ca-certificates.crt; \
        printf '\n' >> /etc/ssl/certs/ca-certificates.crt; \
    done
ENV PIP_CERT=/etc/ssl/certs/ca-certificates.crt
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY corp_site/ /app/

COPY deploy/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

RUN python manage.py collectstatic --no-input

EXPOSE 8000

ENTRYPOINT ["/entrypoint.sh"]
CMD ["gunicorn", "corp_site.wsgi:application", "--bind", "0.0.0.0:8000", "--workers", "3"]
