# Generated manually
from django.conf import settings
from django.db import migrations, models
import django.db.models.deletion
import django.utils.timezone


class Migration(migrations.Migration):

    dependencies = [
        migrations.swappable_dependency(settings.AUTH_USER_MODEL),
        ('cases', '0001_initial'),
    ]

    operations = [
        migrations.AddField(
            model_name='case',
            name='reviewer',
            field=models.ForeignKey(
                blank=True,
                null=True,
                on_delete=django.db.models.deletion.SET_NULL,
                to=settings.AUTH_USER_MODEL,
                help_text='Analyst who reviewed the case',
            ),
        ),
        migrations.RunSQL(
            sql="UPDATE cases SET reviewer_id = (SELECT id FROM auth_user WHERE username = reviewed_by) WHERE reviewed_by IS NOT NULL;",
            reverse_sql=migrations.RunSQL.noop,
        ),
        migrations.RemoveField(
            model_name='case',
            name='reviewed_by',
        ),
        migrations.AlterField(
            model_name='case',
            name='score',
            field=models.DecimalField(
                decimal_places=4,
                help_text='Fraud probability score from the model',
                max_digits=5,
            ),
        ),
        migrations.AlterField(
            model_name='case',
            name='status',
            field=models.CharField(
                choices=[
                    ('pending', 'Pending'),
                    ('investigating', 'Investigating'),
                    ('confirmed_fraud', 'Confirmed Fraud'),
                    ('cleared', 'Cleared'),
                    ('false_positive', 'False Positive'),
                ],
                db_index=True,
                default='pending',
                help_text='Current review status of the case',
                max_length=50,
            ),
        ),
        migrations.AlterField(
            model_name='case',
            name='flagged_at',
            field=models.DateTimeField(
                db_index=True,
                default=django.utils.timezone.now,
                help_text='Timestamp when the transaction was flagged',
            ),
        ),
    ]
