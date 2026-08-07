from django.urls import path

from . import views

urlpatterns = [
    path('', views.index, name='index'),
    path('equipment/', views.equipment_list, name='equipment_list'),
    path('equipment/<slug:slug>/', views.equipment_detail, name='equipment_detail'),
    path('services/', views.services, name='services'),
    path('projects/', views.projects_list, name='projects'),
    path('calculator/', views.calculator, name='calculator'),
    path('blog/', views.blog_list, name='blog'),
    path('blog/<slug:slug>/', views.blog_detail, name='blog_detail'),
    path('contacts/', views.contacts, name='contacts'),
    path('contacts/success/', views.contact_success, name='contact_success'),
    path('robots.txt', views.robots_txt, name='robots_txt'),
    path('sitemap.xml', views.sitemap_xml, name='sitemap'),
]
