"""Assertions that catch silent training-serving skew and data bugs.

These invariants are cheap enough to run before every model deployment
and after every feature or pipeline change.
"""

import time
import pytest
from unittest.mock import MagicMock

import redis
from model_utils import get_model_features
from scorer import compute_features


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

def _mock_hgetall(key):
    data = {
        "cust:u1:agg": {"night_ratio": "0.1", "fraud_rate": "0.01", "mean_hour": "14.5"},
        "merch:m1:agg": {"fraud_rate": "0.001"},
        "cust:u1:stats": {"count": "10", "mean": "500.0", "M2": "0.0"},
        "card:c1:loc": {},
    }
    return data.get(key, {})


@pytest.fixture
def mock_redis():
    r = MagicMock(spec=redis.Redis)
    r.hgetall.side_effect = _mock_hgetall
    r.get.return_value = None
    r.incr.return_value = 42
    r.zcard.return_value = 1
    r.lindex.return_value = None
    return r


def make_tx(customer_id="u1", card_id="c1", amount=500.0, hour=14, card_present=True):
    return {
        "transaction_id": f"tx-{int(time.time() * 1e6)}",
        "card_id": card_id,
        "customer_id": customer_id,
        "merchant_id": "m1",
        "amount": amount,
        "timestamp": f"2026-07-19T{hour:02d}:00:00Z",
        "card_present": card_present,
        "transaction_channel": "upi",
        "merchant_category": "GROCERY",
        "location_lat": 18.52,
        "location_long": 73.85,
    }


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

def test_feature_parity_model_vs_scorer(mock_redis):
    """Every feature the model was trained on MUST be produced by the scorer.

    If a model feature is absent from the scorer output dict, it silently
    becomes 0.0 at inference (scorer.py L175-176: df[f] = 0.0).  This test
    catches the exact class of bug that made hour_deviation_from_norm a
    silent 0.0 — the model trains on real values, inference gets zeros.
    """
    model_features = set(get_model_features())
    features = compute_features(make_tx(), mock_redis)
    scorer_features = set(features.keys())

    missing = model_features - scorer_features
    assert not missing, (
        f"Model expects these features that the scorer does NOT produce: {missing}\n"
        f"They will be silently filled with 0.0 at inference.\n"
        f"Either implement them in compute_features() or remove them from train_xgboost.py."
    )


def test_time_since_last_transaction_non_negative(mock_redis):
    """time_since_last_transaction must never be negative.

    A negative value means the scorer encountered a future timestamp in Redis
    (clock skew, bad seed data, or a timezone bug). The model was trained
    with non-negative values and will produce unpredictable results with
    negative input.
    """
    features = compute_features(make_tx(), mock_redis)
    assert features["time_since_last_transaction"] >= 0, (
        f"time_since_last_transaction is {features['time_since_last_transaction']:.3f}. "
        f"Negative values indicate a timestamp ordering bug."
    )


def test_time_since_last_positive_when_prior_exists(mock_redis):
    """When a prior timestamp exists in Redis, time_since_last must be > 0.

    If it's 0 with a prior timestamp, the delta computation is broken
    (e.g. subtracting a future timestamp, or misinterpreting the stored value).
    """
    now = time.time()
    prior = now - 60  # 60 seconds ago
    mock_redis.get.return_value = str(prior)

    features = compute_features(make_tx(), mock_redis)

    assert features["time_since_last_transaction"] > 0, (
        f"Expected positive time_since_last_transaction when prior timestamp exists, "
        f"got {features['time_since_last_transaction']:.3f}"
    )


def test_hour_deviation_not_silently_zero(mock_redis):
    """hour_deviation_from_norm must vary with the transaction hour.

    If every transaction returns 0.0 regardless of hour, the Redis mean_hour
    is missing or the computation is stubbed out.  This was the exact bug
    that shipped before the fix.
    """
    tx_noon = make_tx(hour=12)
    tx_night = make_tx(hour=2)

    feat_noon = compute_features(tx_noon, mock_redis)
    feat_night = compute_features(tx_night, mock_redis)

    assert feat_noon["hour_deviation_from_norm"] != 0.0 or \
           feat_night["hour_deviation_from_norm"] != 0.0, (
        "hour_deviation_from_norm is 0.0 for multiple different hours — "
        "likely missing mean_hour in Redis or stubbed computation."
    )

    # Different hours should produce different deviations from the same mean
    assert feat_noon["hour_deviation_from_norm"] != feat_night["hour_deviation_from_norm"], (
        f"hour_deviation_from_norm should differ for hours 12 and 2 "
        f"(mean=14.5), got {feat_noon['hour_deviation_from_norm']} vs "
        f"{feat_night['hour_deviation_from_norm']}"
    )
