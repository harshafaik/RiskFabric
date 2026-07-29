#!/bin/sh
set -e

echo "Waiting for database connection..."
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
until python -c "
import sys, psycopg2, os
try:
    psycopg2.connect(
        host=os.getenv('DB_HOST', 'localhost'),
        port=os.getenv('DB_PORT', '5432'),
        dbname=os.getenv('DB_NAME', 'riskfabric_oltp'),
        user=os.getenv('DB_USER', 'riskfabric_oltp_user'),
        password=os.getenv('DB_PASSWORD', '123')
    )
except Exception:
    sys.exit(-1)
sys.exit(0)
"; do
  echo "Database is unavailable - sleeping"
  sleep 1
done

echo "Database is up! Running migrations..."
python manage.py migrate --noinput

echo "Creating superuser if it doesn't exist..."
python manage.py shell -c "
from django.contrib.auth.models import User
import os
username = os.getenv('DJANGO_SUPERUSER_USERNAME', 'admin')
email = os.getenv('DJANGO_SUPERUSER_EMAIL', 'admin@example.com')
password = os.getenv('DJANGO_SUPERUSER_PASSWORD', 'admin')
if not User.objects.filter(username=username).exists():
    User.objects.create_superuser(username, email, password)
    print(f'Superuser {username} created successfully!')
else:
    print(f'Superuser {username} already exists.')
"

echo "Collecting static files..."
python manage.py collectstatic --noinput

echo "Starting Django dev server..."
exec python manage.py runserver 0.0.0.0:8000
