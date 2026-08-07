import os
import re


def site_contacts(request):
    """Контакты компании доступны во всех шаблонах и меняются через env
    без правки кода — при переезде на домен или смене номера достаточно
    обновить переменные окружения."""
    phone = os.environ.get('SITE_PHONE', '+7 (XXX) XXX-XX-XX')
    return {
        'SITE_PHONE': phone,
        'SITE_PHONE_LINK': re.sub(r'[^0-9+]', '', phone),
        'SITE_EMAIL': os.environ.get('SITE_EMAIL', 'info@specmontazh.ru'),
        'SITE_WHATSAPP': os.environ.get('SITE_WHATSAPP', ''),
        'SITE_TELEGRAM': os.environ.get('SITE_TELEGRAM', ''),
        'SITE_WORK_HOURS': os.environ.get('SITE_WORK_HOURS', 'Пн—Пт: 9:00—18:00'),
        'YANDEX_METRIKA_ID': os.environ.get('YANDEX_METRIKA_ID', ''),
    }
