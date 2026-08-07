from django.http import HttpResponse
from django.shortcuts import get_object_or_404, redirect, render
from django.urls import reverse

from .forms import ContactForm, QuizForm
from .models import Category, ContactRequest, Partner, Project
from .notifications import notify_manager


def index(request):
    return render(request, 'catalog/index.html', {
        'categories': Category.objects.all()[:6],
        'partners': Partner.objects.all(),
        'projects': Project.objects.filter(is_published=True)[:3],
    })


def equipment_list(request):
    categories = Category.objects.all()
    return render(request, 'catalog/equipment_list.html', {
        'categories': categories,
    })


def equipment_detail(request, slug):
    category = get_object_or_404(Category, slug=slug)
    return render(request, 'catalog/equipment_detail.html', {
        'category': category,
        'projects': category.projects.filter(is_published=True)[:3],
    })


def services(request):
    return render(request, 'catalog/services.html')


def projects_list(request):
    return render(request, 'catalog/projects.html', {
        'projects': Project.objects.filter(is_published=True),
    })


def calculator(request):
    if request.method == 'POST':
        form = QuizForm(request.POST)
        if form.is_valid():
            contact = ContactRequest.objects.create(
                name=form.cleaned_data['name'],
                phone=form.cleaned_data['phone'],
                message=form.build_message(),
                source=ContactRequest.Source.QUIZ,
            )
            notify_manager(contact)
            return redirect('contact_success')
    else:
        form = QuizForm()
    return render(request, 'catalog/calculator.html', {'form': form})


def contacts(request):
    if request.method == 'POST':
        form = ContactForm(request.POST)
        if form.is_valid():
            contact = form.save()
            notify_manager(contact)
            return redirect('contact_success')
    else:
        form = ContactForm()
    return render(request, 'catalog/contacts.html', {'form': form})


def contact_success(request):
    return render(request, 'catalog/contact_success.html')


def robots_txt(request):
    sitemap_url = request.build_absolute_uri(reverse('sitemap'))
    content = f'User-agent: *\nAllow: /\nDisallow: /admin/\n\nSitemap: {sitemap_url}\n'
    return HttpResponse(content, content_type='text/plain')


def sitemap_xml(request):
    static_names = [
        'index', 'equipment_list', 'services', 'projects', 'calculator',
        'contacts',
    ]
    urls = [request.build_absolute_uri(reverse(name)) for name in static_names]
    urls += [
        request.build_absolute_uri(
            reverse('equipment_detail', args=[cat.slug])
        )
        for cat in Category.objects.all()
    ]
    items = '\n'.join(f'  <url><loc>{u}</loc></url>' for u in urls)
    xml = (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n'
        f'{items}\n'
        '</urlset>\n'
    )
    return HttpResponse(xml, content_type='application/xml')
