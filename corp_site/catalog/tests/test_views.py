from django.core import mail
from django.test import TestCase, override_settings
from django.urls import reverse

from catalog.models import Category, ContactRequest, Partner, Project


class IndexViewTest(TestCase):
    def test_index_page_loads(self):
        response = self.client.get(reverse('index'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'СпецМонтаж')


class EquipmentListViewTest(TestCase):
    @classmethod
    def setUpTestData(cls):
        cls.cat = Category.objects.create(
            name='Видеонаблюдение',
            slug='svn',
            short_name='СВН',
            description='Описание СВН',
            icon='bi-camera-video',
            order=1,
        )

    def test_equipment_list_loads(self):
        response = self.client.get(reverse('equipment_list'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Видеонаблюдение')

    def test_equipment_list_empty(self):
        Category.objects.all().delete()
        response = self.client.get(reverse('equipment_list'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Каталог оборудования пока пуст')


class EquipmentDetailViewTest(TestCase):
    @classmethod
    def setUpTestData(cls):
        cls.cat = Category.objects.create(
            name='СКУД',
            slug='skud',
            short_name='СКУД',
            description='Описание',
            icon='bi-fingerprint',
            order=1,
        )

    def test_detail_loads(self):
        response = self.client.get(
            reverse('equipment_detail', args=[self.cat.slug])
        )
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'СКУД')

    def test_detail_404(self):
        response = self.client.get(
            reverse('equipment_detail', args=['nonexistent'])
        )
        self.assertEqual(response.status_code, 404)


class ServicesViewTest(TestCase):
    def test_services_page_loads(self):
        response = self.client.get(reverse('services'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Проектирование')
        self.assertContains(response, 'Монтаж')
        self.assertContains(response, 'Обслуживание')


class ContactsViewTest(TestCase):
    def test_contacts_page_loads(self):
        response = self.client.get(reverse('contacts'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Форма обратной связи')

    def test_submit_valid_form(self):
        response = self.client.post(reverse('contacts'), {
            'name': 'Иван Иванов',
            'email': 'ivan@example.com',
            'phone': '+7 999 123 4567',
            'message': 'Нужна консультация по СКУД',
        })
        self.assertRedirects(response, reverse('contact_success'))
        self.assertEqual(ContactRequest.objects.count(), 1)
        req = ContactRequest.objects.first()
        self.assertEqual(req.name, 'Иван Иванов')

    def test_submit_invalid_form(self):
        response = self.client.post(reverse('contacts'), {
            'name': '',
            'email': 'invalid',
            'message': '',
        })
        self.assertEqual(response.status_code, 200)
        self.assertEqual(ContactRequest.objects.count(), 0)


class ContactSuccessViewTest(TestCase):
    def test_success_page_loads(self):
        response = self.client.get(reverse('contact_success'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Заявка успешно отправлена')


class ProjectsViewTest(TestCase):
    @classmethod
    def setUpTestData(cls):
        cls.project = Project.objects.create(
            title='Складской комплекс',
            slug='sklad',
            object_type='Склад',
            description='Задача',
            result='32 камеры\nСервер',
            is_published=True,
        )
        Project.objects.create(
            title='Черновик', slug='draft', description='-', is_published=False,
        )

    def test_projects_page_loads(self):
        response = self.client.get(reverse('projects'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Складской комплекс')
        self.assertContains(response, '32 камеры')

    def test_unpublished_hidden(self):
        response = self.client.get(reverse('projects'))
        self.assertNotContains(response, 'Черновик')


class CalculatorViewTest(TestCase):
    def test_calculator_page_loads(self):
        response = self.client.get(reverse('calculator'))
        self.assertEqual(response.status_code, 200)
        self.assertContains(response, 'Рассчитайте стоимость')

    def test_submit_valid_quiz(self):
        response = self.client.post(reverse('calculator'), {
            'object_type': 'Офис',
            'systems': ['Видеонаблюдение', 'СКУД (контроль доступа)'],
            'area': '100–500 м²',
            'name': 'Пётр',
            'phone': '+7 900 000 00 00',
        })
        self.assertRedirects(response, reverse('contact_success'))
        req = ContactRequest.objects.get()
        self.assertEqual(req.source, ContactRequest.Source.QUIZ)
        self.assertIn('Видеонаблюдение', req.message)
        self.assertIn('Офис', req.message)

    def test_submit_incomplete_quiz(self):
        response = self.client.post(reverse('calculator'), {
            'object_type': 'Офис',
            'name': 'Пётр',
        })
        self.assertEqual(response.status_code, 200)
        self.assertEqual(ContactRequest.objects.count(), 0)


class PriceDisplayTest(TestCase):
    def test_price_from_shown(self):
        Category.objects.create(
            name='СВН', slug='svn', description='-', price_from=35000, order=1,
        )
        response = self.client.get(reverse('equipment_list'))
        self.assertContains(response, 'от 35000')


class PartnersDisplayTest(TestCase):
    def test_partners_on_index(self):
        Partner.objects.create(name='Hikvision', order=1)
        response = self.client.get(reverse('index'))
        self.assertContains(response, 'Hikvision')


@override_settings(MANAGER_EMAIL='manager@example.com')
class EmailNotificationTest(TestCase):
    def test_email_sent_on_contact(self):
        self.client.post(reverse('contacts'), {
            'name': 'Иван',
            'email': 'ivan@example.com',
            'phone': '',
            'message': 'Вопрос по СКУД',
        })
        self.assertEqual(len(mail.outbox), 1)
        self.assertIn('Иван', mail.outbox[0].subject)
        self.assertIn('Вопрос по СКУД', mail.outbox[0].body)


class ContactFormValidationTest(TestCase):
    def test_requires_phone_or_email(self):
        response = self.client.post(reverse('contacts'), {
            'name': 'Иван',
            'email': '',
            'phone': '',
            'message': 'Сообщение без контактов',
        })
        self.assertEqual(response.status_code, 200)
        self.assertEqual(ContactRequest.objects.count(), 0)

    def test_phone_only_is_enough(self):
        response = self.client.post(reverse('contacts'), {
            'name': 'Иван',
            'email': '',
            'phone': '+7 900 000 00 00',
            'message': 'Только телефон',
        })
        self.assertRedirects(response, reverse('contact_success'))
        self.assertEqual(ContactRequest.objects.count(), 1)


class SeoViewsTest(TestCase):
    @classmethod
    def setUpTestData(cls):
        Category.objects.create(
            name='СКУД', slug='skud', description='Описание', order=1,
        )

    def test_robots_txt(self):
        response = self.client.get('/robots.txt')
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response['Content-Type'], 'text/plain')
        self.assertContains(response, 'Sitemap:')
        self.assertContains(response, 'Disallow: /admin/')

    def test_sitemap_xml(self):
        response = self.client.get('/sitemap.xml')
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response['Content-Type'], 'application/xml')
        self.assertContains(response, '/equipment/skud/')
