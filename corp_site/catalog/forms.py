from django import forms

from .models import ContactRequest


class ContactForm(forms.ModelForm):
    class Meta:
        model = ContactRequest
        fields = ['name', 'email', 'phone', 'message']
        widgets = {
            'name': forms.TextInput(attrs={
                'class': 'form-control',
                'placeholder': 'Ваше имя',
            }),
            'email': forms.EmailInput(attrs={
                'class': 'form-control',
                'placeholder': 'email@example.com',
            }),
            'phone': forms.TextInput(attrs={
                'class': 'form-control',
                'placeholder': '+7 (___) ___-__-__',
            }),
            'message': forms.Textarea(attrs={
                'class': 'form-control',
                'rows': 5,
                'placeholder': 'Опишите ваш запрос...',
            }),
        }

    def clean(self):
        cleaned = super().clean()
        if not cleaned.get('email') and not cleaned.get('phone'):
            raise forms.ValidationError(
                'Укажите телефон или электронную почту, '
                'чтобы мы могли с вами связаться.'
            )
        return cleaned


class QuizForm(forms.Form):
    OBJECT_CHOICES = [
        ('Квартира', 'Квартира'),
        ('Частный дом / дача', 'Частный дом / дача'),
        ('Офис', 'Офис'),
        ('Магазин / торговая точка', 'Магазин / торговая точка'),
        ('Склад / производство', 'Склад / производство'),
        ('Другое', 'Другое'),
    ]
    SYSTEM_CHOICES = [
        ('Видеонаблюдение', 'Видеонаблюдение'),
        ('СКУД (контроль доступа)', 'СКУД (контроль доступа)'),
        ('Охранная сигнализация', 'Охранная сигнализация'),
        ('Система оповещения', 'Система оповещения'),
        ('Освещение территории', 'Освещение территории'),
        ('Электромонтаж', 'Электромонтаж'),
        ('Забор / ворота / КПП', 'Забор / ворота / КПП'),
    ]
    AREA_CHOICES = [
        ('до 100 м²', 'до 100 м²'),
        ('100–500 м²', '100–500 м²'),
        ('500–2000 м²', '500–2000 м²'),
        ('более 2000 м²', 'более 2000 м²'),
    ]

    object_type = forms.ChoiceField(
        label='Тип объекта', choices=OBJECT_CHOICES,
        widget=forms.RadioSelect,
    )
    systems = forms.MultipleChoiceField(
        label='Какие системы нужны', choices=SYSTEM_CHOICES,
        widget=forms.CheckboxSelectMultiple,
    )
    area = forms.ChoiceField(
        label='Площадь объекта', choices=AREA_CHOICES,
        widget=forms.RadioSelect,
    )
    name = forms.CharField(
        label='Имя', max_length=150,
        widget=forms.TextInput(attrs={
            'class': 'form-control', 'placeholder': 'Ваше имя',
        }),
    )
    phone = forms.CharField(
        label='Телефон', max_length=30,
        widget=forms.TextInput(attrs={
            'class': 'form-control', 'placeholder': '+7 (___) ___-__-__',
        }),
    )

    def build_message(self):
        data = self.cleaned_data
        systems = ', '.join(data['systems'])
        return (
            f'Заявка из калькулятора.\n'
            f'Тип объекта: {data["object_type"]}\n'
            f'Системы: {systems}\n'
            f'Площадь: {data["area"]}'
        )
