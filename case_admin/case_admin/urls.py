from django.urls import path
from django.shortcuts import redirect
from cases.admin import admin_site

urlpatterns = [
    path('', lambda request: redirect('admin/', permanent=False)),
    path('admin/', admin_site.urls),
]
