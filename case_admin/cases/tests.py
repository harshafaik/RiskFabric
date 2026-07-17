from decimal import Decimal
from django.test import TestCase
from django.contrib.auth.models import User
from django.utils import timezone
from .models import Case, CaseStatus


class CaseModelTest(TestCase):
    def setUp(self) -> None:
        self.user = User.objects.create_user(username='analyst1', password='testpass')
        self.case = Case.objects.create(
            transaction_id='tx-001',
            score=Decimal('0.9500'),
            status=CaseStatus.PENDING,
            flagged_at=timezone.now(),
        )

    def test_create_case(self) -> None:
        self.assertEqual(self.case.transaction_id, 'tx-001')
        self.assertEqual(self.case.score, Decimal('0.9500'))
        self.assertEqual(self.case.status, CaseStatus.PENDING)
        self.assertIsNone(self.case.reviewer)
        self.assertIsNone(self.case.reviewed_at)

    def test_score_float_property(self) -> None:
        self.assertAlmostEqual(self.case.score_float, 0.95)

    def test_str_representation(self) -> None:
        expected = f"Case #{self.case.id} [Tx: tx-001...] Status: pending"
        self.assertEqual(str(self.case), expected)

    def test_reviewer_assignment(self) -> None:
        self.case.reviewer = self.user
        self.case.reviewed_at = timezone.now()
        self.case.save()
        updated = Case.objects.get(id=self.case.id)
        self.assertEqual(updated.reviewer, self.user)
        self.assertIsNotNone(updated.reviewed_at)

    def test_status_choices(self) -> None:
        for status, _ in CaseStatus.choices:
            self.case.status = status
            self.case.save()
            self.assertEqual(Case.objects.get(id=self.case.id).status, status)

    def test_flag_reasons_store_json(self) -> None:
        reasons = {'amount': 15000.0, 'score_threshold_crossed': True}
        self.case.flag_reasons = reasons
        self.case.save()
        self.assertEqual(Case.objects.get(id=self.case.id).flag_reasons, reasons)

    def test_default_status_is_pending(self) -> None:
        case = Case.objects.create(
            transaction_id='tx-002',
            score=Decimal('0.5000'),
        )
        self.assertEqual(case.status, CaseStatus.PENDING)

    def test_ordering(self) -> None:
        earlier = Case.objects.create(
            transaction_id='tx-010',
            score=Decimal('0.5000'),
            flagged_at=timezone.make_aware(timezone.datetime(2024, 1, 1)),
        )
        later = Case.objects.create(
            transaction_id='tx-011',
            score=Decimal('0.6000'),
            flagged_at=timezone.make_aware(timezone.datetime(2024, 6, 1)),
        )
        cases = list(Case.objects.all()[:3])
        self.assertEqual(cases[0], later)
        self.assertEqual(cases[2], earlier)

    def test_notes_and_reviewer_nullable(self) -> None:
        case = Case.objects.create(
            transaction_id='tx-003',
            score=Decimal('0.3000'),
        )
        self.assertIsNone(case.notes)
        self.assertIsNone(case.reviewer)
