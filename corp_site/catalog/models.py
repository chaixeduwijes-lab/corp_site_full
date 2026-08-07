from django.db import models


class Category(models.Model):
    name = models.CharField('Название', max_length=255)
    slug = models.SlugField('URL-идентификатор', unique=True, max_length=100)
    short_name = models.CharField('Аббревиатура', max_length=20, blank=True)
    description = models.TextField('Краткое описание')
    full_description = models.TextField('Полное описание', blank=True)
    icon = models.CharField(
        'CSS-класс иконки (Bootstrap Icons)', max_length=100, blank=True
    )
    price_from = models.PositiveIntegerField(
        'Цена «от», ₽', null=True, blank=True,
        help_text='Ориентир стоимости монтажа; пусто — не показывать',
    )
    order = models.PositiveIntegerField('Порядок отображения', default=0)

    class Meta:
        verbose_name = 'Категория оборудования'
        verbose_name_plural = 'Категории оборудования'
        ordering = ['order', 'name']

    def __str__(self):
        return self.name


class Project(models.Model):
    title = models.CharField('Название объекта', max_length=255)
    slug = models.SlugField('URL-идентификатор', unique=True, max_length=100)
    object_type = models.CharField(
        'Тип объекта', max_length=100, blank=True,
        help_text='Например: складской комплекс, ЖК, офис',
    )
    category = models.ForeignKey(
        Category, verbose_name='Основная категория',
        on_delete=models.SET_NULL, null=True, blank=True,
        related_name='projects',
    )
    description = models.TextField('Задача')
    result = models.TextField(
        'Что сделано', blank=True,
        help_text='Каждый пункт с новой строки — выводится списком',
    )
    image = models.ImageField(
        'Фото объекта', upload_to='projects/', blank=True,
    )
    completed_at = models.DateField('Дата завершения', null=True, blank=True)
    is_published = models.BooleanField('Опубликован', default=True)
    order = models.PositiveIntegerField('Порядок отображения', default=0)

    class Meta:
        verbose_name = 'Выполненный объект'
        verbose_name_plural = 'Выполненные объекты'
        ordering = ['order', '-completed_at']

    def __str__(self):
        return self.title

    def result_items(self):
        return [line.strip() for line in self.result.splitlines() if line.strip()]


class Partner(models.Model):
    name = models.CharField('Название', max_length=100)
    logo = models.ImageField('Логотип', upload_to='partners/', blank=True)
    url = models.URLField('Сайт', blank=True)
    order = models.PositiveIntegerField('Порядок отображения', default=0)

    class Meta:
        verbose_name = 'Бренд-партнёр'
        verbose_name_plural = 'Бренды-партнёры'
        ordering = ['order', 'name']

    def __str__(self):
        return self.name


class ContactRequest(models.Model):
    class Source(models.TextChoices):
        FORM = 'form', 'Форма обратной связи'
        QUIZ = 'quiz', 'Калькулятор'

    name = models.CharField('Имя', max_length=150)
    email = models.EmailField('Электронная почта', blank=True)
    phone = models.CharField('Телефон', max_length=30, blank=True)
    message = models.TextField('Сообщение')
    source = models.CharField(
        'Источник', max_length=10,
        choices=Source.choices, default=Source.FORM,
    )
    created_at = models.DateTimeField('Дата создания', auto_now_add=True)

    class Meta:
        verbose_name = 'Заявка'
        verbose_name_plural = 'Заявки'
        ordering = ['-created_at']

    def __str__(self):
        return f'{self.name} — {self.created_at:%d.%m.%Y %H:%M}'
