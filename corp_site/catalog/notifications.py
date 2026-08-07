import logging

from django.conf import settings
from django.core.mail import send_mail

logger = logging.getLogger(__name__)


def notify_manager(contact_request):
    """Письмо менеджеру о новой заявке. Ошибка отправки не должна ломать
    оформление заявки — логируем и продолжаем."""
    recipient = settings.MANAGER_EMAIL
    if not recipient:
        return
    subject = f'Новая заявка с сайта: {contact_request.name}'
    body = (
        f'Источник: {contact_request.get_source_display()}\n'
        f'Имя: {contact_request.name}\n'
        f'Телефон: {contact_request.phone or "—"}\n'
        f'Email: {contact_request.email or "—"}\n\n'
        f'{contact_request.message}'
    )
    try:
        send_mail(subject, body, settings.DEFAULT_FROM_EMAIL, [recipient])
    except Exception:
        logger.exception('Не удалось отправить уведомление о заявке #%s',
                         contact_request.pk)
