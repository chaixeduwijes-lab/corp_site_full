from django.contrib import admin

from .models import (
    Article, Category, ContactRequest, Partner, Project, Review,
)


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


@admin.register(Review)
class ReviewAdmin(admin.ModelAdmin):
    list_display = ['author_name', 'object_name', 'rating', 'is_published',
                    'order', 'created_at']
    list_editable = ['is_published', 'order']
    list_filter = ['is_published', 'rating']
    search_fields = ['author_name', 'object_name', 'text']


@admin.register(Article)
class ArticleAdmin(admin.ModelAdmin):
    list_display = ['title', 'published_at', 'is_published']
    list_editable = ['is_published']
    list_filter = ['is_published']
    prepopulated_fields = {'slug': ('title',)}
    search_fields = ['title', 'excerpt', 'body']


@admin.register(ContactRequest)
class ContactRequestAdmin(admin.ModelAdmin):
    list_display = ['name', 'phone', 'email', 'source', 'created_at']
    list_filter = ['source', 'created_at']
    search_fields = ['name', 'email', 'phone']
    readonly_fields = ['created_at']
