import logging
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

logger = logging.getLogger(__name__)


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
        except Exception:
            logger.exception("Dashboard calculation error")

        return super().index(request, extra_context)


admin_site = RiskFabricAdminSite(name='riskfabric_admin')


class CaseAdmin(admin.ModelAdmin):
    list_display = ('transaction_id', 'score_badge', 'status', 'flagged_at', 'reviewer', 'reviewed_at', 'notes')
    list_editable = ('status',)
    list_filter = ('status', 'flagged_at')
    search_fields = ('transaction_id', 'notes', 'reviewer__username')
    date_hierarchy = 'flagged_at'
    list_select_related = ('reviewer',)
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

    actions = ['mark_investigating', 'mark_cleared']

    def score_badge(self, obj: Case) -> str:
        score_val = obj.score_float
        if score_val >= 0.90:
            css_class = 'high'
            label = 'High Risk'
        elif score_val >= 0.70:
            css_class = 'medium'
            label = 'Medium Risk'
        else:
            css_class = 'low'
            label = 'Low Risk'

        score_percent = f"{score_val:.2%}"
        return format_html(
            '<span class="score-badge {}">{}</span>',
            css_class, f"{score_percent} — {label}"
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

            rows = []
            for key, val in data.items():
                if key == 'amount':
                    val_str = format_html('<strong>\u20b9{:,.2f}</strong>', val)
                elif key == 'score_threshold_crossed':
                    badge_class = 'danger' if val else 'success'
                    val_str = format_html('<span class="badge badge-{}">{}</span>', badge_class, val)
                else:
                    val_str = str(val)
                    if len(val_str) > 60:
                        val_str = format_html(
                            '<span class="shap-truncated" title="{}">{}</span>',
                            str(val), val_str[:57] + '...'
                        )

                rows.append(format_html(
                    '<tr><td><code>{}</code></td><td>{}</td></tr>',
                    key, val_str
                ))

            html = format_html(
                '<div class="shap-table"><table class="table table-bordered table-striped"><thead><tr><th>Feature / Trigger</th><th>Value</th></tr></thead><tbody>{}</tbody></table></div>',
                ''.join(rows) if rows else '<tr><td colspan="2">No triggers</td></tr>'
            )
            return html
        except Exception as e:
            return format_html('<span class="text-danger">Error parsing triggers: {}</span>', str(e))
    flag_reasons_rendered.short_description = "Fraud Analysis Indicators"

    @admin.action(description="Mark selected as Investigating")
    def mark_investigating(self, request, queryset):
        updated = queryset.filter(status='pending').update(status='investigating')
        self.message_user(request, f"{updated} case(s) moved to Investigating.")

    @admin.action(description="Mark selected as Cleared")
    def mark_cleared(self, request, queryset):
        updated = queryset.filter(status='investigating').update(status='cleared')
        self.message_user(request, f"{updated} case(s) marked as Cleared.")

    def save_model(self, request: HttpRequest, obj: Case, form: object, change: bool) -> None:
        if change and obj.reviewer is None:
            obj.reviewer = request.user
            obj.reviewed_at = timezone.now()
        super().save_model(request, obj, form, change)


admin_site.register(Case, CaseAdmin)
admin_site.register(User, UserAdmin)
admin_site.register(Group, GroupAdmin)
