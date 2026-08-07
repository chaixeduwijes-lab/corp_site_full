from django.contrib import admin

from .models import Category, ContactRequest, Partner, Project


@admin.register(Category)
class CategoryAdmin(admin.ModelAdmin):
    list_display = ['name', 'short_name', 'slug', 'price_from', 'order']
    list_editable = ['price_from', 'order']
    prepopulated_fields = {'slug': ('name',)}
    search_fields = ['name', 'short_name']


@admin.register(Project)
class ProjectAdmin(admin.ModelAdmin):
    list_display = ['title', 'object_type', 'category', 'completed_at',
                    'is_published', 'order']
    list_editable = ['is_published', 'order']
    list_filter = ['is_published', 'category']
    prepopulated_fields = {'slug': ('title',)}
    search_fields = ['title', 'object_type']


@admin.register(Partner)
class PartnerAdmin(admin.ModelAdmin):
    list_display = ['name', 'url', 'order']
    list_editable = ['order']


@admin.register(ContactRequest)
class ContactRequestAdmin(admin.ModelAdmin):
    list_display = ['name', 'phone', 'email', 'source', 'created_at']
    list_filter = ['source', 'created_at']
    search_fields = ['name', 'email', 'phone']
    readonly_fields = ['created_at']
