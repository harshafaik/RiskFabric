from typing import Optional
from django.contrib import admin
from django.contrib.admin import AdminSite
from django.utils import timezone
from django.utils.html import format_html
from django.contrib.auth.models import User, Group
from django.contrib.auth.admin import UserAdmin, GroupAdmin
from django.db.models import Count
from django.http import HttpRequest
import json
from .models import Case


class RiskFabricAdminSite(AdminSite):
    site_header = "RiskFabric"
    site_title = "RiskFabric Fraud Intel"
    index_title = "Fraud Investigation Dashboard"

    def index(self, request: HttpRequest, extra_context: Optional[dict] = None) -> object:
        extra_context = extra_context or {}
        try:
            total_cases = Case.objects.count()
            status_qs = Case.objects.values('status').annotate(total=Count('status'))
            status_counts = {item['status']: item['total'] for item in status_qs}

            pending = status_counts.get('pending', 0)
            investigating = status_counts.get('investigating', 0)
            confirmed = status_counts.get('confirmed_fraud', 0)
            cleared = status_counts.get('cleared', 0)
            false_positive = status_counts.get('false_positive', 0)

            total_reviewed = confirmed + false_positive
            if total_reviewed > 0:
                fpr = false_positive / total_reviewed * 100
                fpr_display = f"{fpr:.2f}%"
            else:
                fpr = None
                fpr_display = "N/A"

            extra_context.update({
                'total_cases': total_cases,
                'pending_cases': pending,
                'investigating_cases': investigating,
                'confirmed_cases': confirmed,
                'cleared_cases': cleared,
                'false_positive_cases': false_positive,
                'false_positive_rate': fpr_display,
            })
        except Exception as e:
            print("Dashboard calculation error:", e)

        return super().index(request, extra_context)


admin_site = RiskFabricAdminSite(name='riskfabric_admin')


class CaseAdmin(admin.ModelAdmin):
    list_display = ('id', 'transaction_id', 'score_badge', 'status', 'flagged_at', 'reviewer', 'reviewed_at', 'notes')

    list_editable = ('status', 'notes')

    list_filter = ('status', 'flagged_at')

    search_fields = ('transaction_id', 'notes', 'reviewer__username')

    date_hierarchy = 'flagged_at'

    readonly_fields = ('flagged_at', 'reviewed_at', 'reviewer', 'flag_reasons_rendered')

    autocomplete_fields = ('reviewer',)

    fieldsets = (
        ('Case Info', {
            'fields': ('transaction_id', 'score', 'flagged_at')
        }),
        ('Investigation details', {
            'fields': ('status', 'notes', 'flag_reasons_rendered')
        }),
        ('Review Audit', {
            'fields': ('reviewer', 'reviewed_at')
        }),
    )

    def score_badge(self, obj: Case) -> str:
        score_val = obj.score_float
        if score_val >= 0.90:
            color = '#dc3545'
            label = 'High Risk'
        elif score_val >= 0.70:
            color = '#fd7e14'
            label = 'Medium Risk'
        else:
            color = '#28a745'
            label = 'Low Risk'

        score_percent = f"{score_val:.2%}"
        return format_html(
            '<span class="badge" style="background-color: {0}; color: #fff; padding: 5px 8px; font-weight: bold;">{1} ({2})</span>',
            color, score_percent, label
        )
    score_badge.short_description = 'Fraud Score'
    score_badge.admin_order_field = 'score'

    def flag_reasons_rendered(self, obj: Case) -> str:
        if not obj.flag_reasons:
            return "No analysis reasons recorded."

        try:
            data = obj.flag_reasons
            if isinstance(data, str):
                data = json.loads(data)

            html = '<div style="max-width: 600px; margin-top: 5px;">'
            html += '<table class="table table-bordered table-striped" style="margin-bottom: 0;">'
            html += '<thead><tr style="background-color: #343a40; color: #fff;"><th>Feature/Trigger</th><th>Value</th></tr></thead>'
            html += '<tbody>'
            for key, val in data.items():
                if key == 'amount':
                    val_str = f"<strong>${val:,.2f}</strong>"
                elif key == 'score_threshold_crossed':
                    badge_color = 'danger' if val else 'success'
                    val_str = f'<span class="badge badge-{badge_color}">{val}</span>'
                else:
                    val_str = str(val)

                html += f'<tr><td><code>{key}</code></td><td>{val_str}</td></tr>'
            html += '</tbody></table></div>'
            return format_html(html)
        except Exception as e:
            return f"Error parsing triggers: {e}"
    flag_reasons_rendered.short_description = "Fraud Analysis Indicators"

    def save_model(self, request: HttpRequest, obj: Case, form: object, change: bool) -> None:
        if change:
            if obj.reviewer is None:
                obj.reviewer = request.user
                obj.reviewed_at = timezone.now()
        super().save_model(request, obj, form, change)


admin_site.register(Case, CaseAdmin)
admin_site.register(User, UserAdmin)
admin_site.register(Group, GroupAdmin)
