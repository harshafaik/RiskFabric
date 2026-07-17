from django.conf import settings
from django.core.exceptions import ValidationError
from django.db import models
from django.utils import timezone
from decimal import Decimal
from typing import Optional


class CaseStatus(models.TextChoices):
    PENDING = 'pending', 'Pending'
    INVESTIGATING = 'investigating', 'Investigating'
    CONFIRMED_FRAUD = 'confirmed_fraud', 'Confirmed Fraud'
    CLEARED = 'cleared', 'Cleared'
    FALSE_POSITIVE = 'false_positive', 'False Positive'


ALLOWED_TRANSITIONS: dict[str, set[str]] = {
    CaseStatus.PENDING: {CaseStatus.INVESTIGATING},
    CaseStatus.INVESTIGATING: {CaseStatus.CONFIRMED_FRAUD, CaseStatus.CLEARED, CaseStatus.FALSE_POSITIVE},
    CaseStatus.CONFIRMED_FRAUD: set(),
    CaseStatus.CLEARED: set(),
    CaseStatus.FALSE_POSITIVE: set(),
}


class Case(models.Model):
    transaction_id = models.CharField(
        max_length=255,
        unique=True,
        help_text="Reference to the transaction ID from the pipeline"
    )
    score = models.DecimalField(
        max_digits=5,
        decimal_places=4,
        help_text="Fraud probability score from the model"
    )
    status = models.CharField(
        max_length=50,
        choices=CaseStatus.choices,
        default=CaseStatus.PENDING,
        db_index=True,
        help_text="Current review status of the case"
    )
    flagged_at = models.DateTimeField(
        default=timezone.now,
        db_index=True,
        help_text="Timestamp when the transaction was flagged"
    )
    reviewer = models.ForeignKey(
        settings.AUTH_USER_MODEL,
        null=True,
        blank=True,
        on_delete=models.SET_NULL,
        help_text="Analyst who reviewed the case"
    )
    reviewed_at = models.DateTimeField(
        null=True,
        blank=True,
        help_text="Timestamp when the review was completed"
    )
    notes = models.TextField(
        null=True,
        blank=True,
        help_text="Free-text analyst investigation notes"
    )
    flag_reasons = models.JSONField(
        null=True,
        blank=True,
        help_text="SHAP top features or rule triggers that flagged this transaction"
    )

    class Meta:
        db_table = 'cases'
        ordering = ['-flagged_at']

    def __str__(self) -> str:
        return f"Case #{self.id} [Tx: {self.transaction_id[:8]}...] Status: {self.status}"

    def clean(self) -> None:
        if self.pk is not None:
            original = Case.objects.get(pk=self.pk)
            if self.status != original.status:
                allowed = ALLOWED_TRANSITIONS.get(original.status, set())
                if self.status not in allowed:
                    raise ValidationError(
                        f"Cannot transition from '{original.status}' to '{self.status}'. "
                        f"Allowed transitions from '{original.status}': {', '.join(sorted(allowed)) or 'none (terminal state)'}."
                    )

    def save(self, *args: object, **kwargs: object) -> None:
        self.full_clean()
        super().save(*args, **kwargs)

    @property
    def score_float(self) -> float:
        return float(self.score) if self.score is not None else 0.0
