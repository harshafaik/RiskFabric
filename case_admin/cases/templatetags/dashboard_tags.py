from django import template

register = template.Library()


@register.filter
def percentage(part, whole):
    try:
        whole = max(whole, 1)
        return round((part / whole) * 100, 1)
    except (ZeroDivisionError, TypeError, ValueError):
        return 0
