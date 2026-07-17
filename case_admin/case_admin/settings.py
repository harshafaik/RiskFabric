import os
from dataclasses import dataclass
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent

SECRET_KEY = 'django-insecure-oltp-case-management-secret-key-for-local-dev'

DEBUG = True

ALLOWED_HOSTS = ['*']


@dataclass
class DBConfig:
    host: str = os.getenv('DB_HOST', 'localhost')
    port: str = os.getenv('DB_PORT', '5432')
    name: str = os.getenv('DB_NAME', 'riskfabric_oltp')
    user: str = os.getenv('DB_USER', 'riskfabric_oltp_user')
    password: str = os.getenv('DB_PASSWORD', '123')


db_config = DBConfig()

DATABASES = {
    'default': {
        'ENGINE': 'django.db.backends.postgresql',
        'NAME': db_config.name,
        'USER': db_config.user,
        'PASSWORD': db_config.password,
        'HOST': db_config.host,
        'PORT': db_config.port,
    }
}

INSTALLED_APPS = [
    'jazzmin',
    'django.contrib.admin',
    'django.contrib.auth',
    'django.contrib.contenttypes',
    'django.contrib.sessions',
    'django.contrib.messages',
    'django.contrib.staticfiles',
    'cases',
]

MIDDLEWARE = [
    'django.middleware.security.SecurityMiddleware',
    'django.contrib.sessions.middleware.SessionMiddleware',
    'django.middleware.common.CommonMiddleware',
    'django.middleware.csrf.CsrfViewMiddleware',
    'django.contrib.auth.middleware.AuthenticationMiddleware',
    'django.contrib.messages.middleware.MessageMiddleware',
    'django.middleware.clickjacking.XFrameOptionsMiddleware',
]

ROOT_URLCONF = 'case_admin.urls'

TEMPLATES = [
    {
        'BACKEND': 'django.template.backends.django.DjangoTemplates',
        'DIRS': [],
        'APP_DIRS': True,
        'OPTIONS': {
            'context_processors': [
                'django.template.context_processors.debug',
                'django.template.context_processors.request',
                'django.contrib.auth.context_processors.auth',
                'django.contrib.messages.context_processors.messages',
            ],
        },
    },
]

WSGI_APPLICATION = 'case_admin.wsgi.application'

AUTH_PASSWORD_VALIDATORS = []

LANGUAGE_CODE = 'en-us'
TIME_ZONE = 'UTC'
USE_I18N = True
USE_TZ = True

STATIC_URL = 'static/'
DEFAULT_AUTO_FIELD = 'django.db.models.BigAutoField'

JAZZMIN_SETTINGS = {
    "site_title": "RiskFabric Fraud Intel",
    "site_header": "RiskFabric",
    "site_brand": "RiskFabric Intel",
    "site_logo_classes": "img-circle",
    "welcome_sign": "Welcome to RiskFabric Case Intel Portal",
    "copyright": "RiskFabric Ltd",
    "search_model": ["cases.Case"],
    "topmenu_links": [
        {"name": "Home",  "url": "admin:index", "permissions": ["auth.view_user"]},
        {"model": "cases.Case"},
    ],
    "show_sidebar": True,
    "navigation_expanded": True,
    "order_with_respect_to": ["cases"],
    "icons": {
        "auth": "fas fa-users-cog",
        "auth.user": "fas fa-user",
        "auth.Group": "fas fa-users",
        "cases.Case": "fas fa-shield-alt",
    },
    "default_icon_parents": "fas fa-chevron-circle-right",
    "default_icon_children": "fas fa-circle",
    "related_modal_active": True,
    "show_ui_builder": True,
    "changeform_format": "horizontal_tabs",
}

JAZZMIN_UI_TWEAKS = {
    "theme": "darkly",
    "dark_mode_theme": "darkly",
    "navbar-theme": "navbar-dark",
    "sidebar": "sidebar-dark-primary",
    "sidebar_nav_child_indent": True,
    "navbar": "navbar-dark navbar-navy",
    "brand_colour": "navbar-navy",
}
