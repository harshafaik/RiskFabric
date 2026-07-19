# Real-Time Scoring Service

## Overview

The scoring service (`scorer.py`) is the real-time inference engine that consumes transaction events from Kafka, computes behavioral features using a Redis-backed state store, and applies the trained XGBoost model to produce per-transaction fraud probabilities. It reads from the `raw_transactions` Kafka topic (populated by `stream.rs`), maintains per-card and per-customer state in Redis, and writes scored results to the `fraud_scores` ClickHouse table. The service uses `model_utils.load_model()` to auto-discover the latest model JSON produced by `train_xgboost.py`.

## Schema

### `fraud_scores` (ClickHouse output table)

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `transaction_id` | `String` | UUID v4 of the scored transaction, linked to `fact_transactions_bronze`. |
| `card_id` | `String` | UUID v4 of the associated card entity. |
| `customer_id` | `String` | UUID v4 of the associated customer profile. |
| `amount` | `Float64` | Transaction monetary value. |
| `timestamp` | `DateTime` | Transaction timestamp as parsed from the Kafka event. |
| `kafka_received_at` | `DateTime64(3)` | Wall-clock time when the Kafka message was received by the consumer, in millisecond precision. |
| `fraud_probability` | `Float64` | Raw XGBoost output probability score in the range [0.0, 1.0]. |
| `flagged` | `UInt8` | Binary fraud flag: 1 if `fraud_probability > 0.85`, 0 otherwise. |
| `scored_at` | `DateTime64(3)` | Wall-clock time when the batch prediction completed, in millisecond precision. |

### Redis Key Schema

| Key Pattern | Type | Description |
| :--- | :--- | :--- |
| `cust:{customer_id}:stats` | Hash (`count`, `mean`, `M2`) | Welford accumulator state for per-customer amount z-score computation. |
| `cust:{customer_id}:agg` | Hash (`night_ratio`, `mean_hour`, ...) | Pre-seeded customer aggregate features from `seed_redis.py`. `mean_hour` is the customer's mean transaction hour, used for `hour_deviation_from_norm`. |
| `card:{card_id}:last_ts` | String (Unix timestamp) | Timestamp of the card's most recent transaction, used to compute `time_since_last_transaction`. |
| `card:{card_id}:burst` | Sorted Set (score = Unix timestamp) | Sliding 60-second transaction window per card for rapid-fire detection; entries outside the window are pruned on each update. |
| `card:{card_id}:history` | List (JSON strings, max 10) | Ring buffer of the last 10 transaction payloads per card, used for `merchant_category_switch_flag` and `escalating_amounts_flag`. |
| `card:{card_id}:loc` | Hash (`lat`, `lon`) | Most recent transaction coordinates per card, used for spatial velocity computation. |
| `card:{card_id}:seq` | Integer (counter) | Monotonically incrementing transaction sequence number per card. |
| `merch:{merchant_id}:agg` | Hash | Pre-seeded merchant aggregate features from `seed_redis.py`. |

**Stateful Micro-Batching** is the core execution model. Transactions are consumed one at a time from Kafka, with feature engineering executed per-transaction. Predictions are deferred until a batch of 50 records accumulates, at which point a single `model.predict_proba` call is issued and the entire batch is inserted into ClickHouse in one `ch.insert` call. This amortizes the overhead of XGBoost inference and the ClickHouse network round-trip across 50 transactions rather than paying it per-event.

**Welford's Online Algorithm** is used to maintain running means and standard deviations for the `amount_deviation_z_score` feature entirely within Redis. The `WelfordState` class stores `count`, `mean`, and `M2` (the sum of squared deviations) as a Redis hash at `cust:{customer_id}:stats`. On each transaction, the state is fetched, the z-score is computed from the current mean and standard deviation, the new amount is incorporated into the accumulator, and the updated state is written back. This produces a stateful behavioral baseline without requiring a historical database scan.

**Rapid-Fire Detection via Sorted Set Pruning** is implemented for the `rapid_fire_transaction_flag` feature. Each transaction for a card is added to a Redis sorted set keyed by `card:{card_id}:burst` with the Unix timestamp as the score. Before counting, all entries older than 60 seconds are removed via `ZREMRANGEBYSCORE`. If the resulting set has more than 3 members, the flag is set to 1. This provides a sliding-window burst counter with O(log N) Redis operations and no background cleanup process.

**Feature Alignment at Inference Time** prevents training-serving skew. Before calling `predict_proba`, the service reads `model.get_booster().feature_names` and `feature_types` directly from the loaded booster to determine the expected column order and types. Missing feature columns are filled with `0.0`. Each column is then cast to the exact dtype the booster expects (`"c"` → `category`, `"float"` → `float32`, `"int"` → `int32`). The DataFrame is reordered to match `feature_names` before prediction. If inference fails despite alignment, the batch falls back to a probability of `0.0` for all records.

**Config-Driven Threshold** is loaded at startup from `data/config/runtime_thresholds.json`, which is produced by `compute_thresholds.py` from the precision-recall curve on the test set. If the config file is missing or malformed, the scorer falls back to a default of `0.85` and logs a warning. The `compute_thresholds.py` script computes the flagging threshold at ~50% recall and exports it alongside operational layer boundaries (auto-block, manual investigation, passive detection) into the same JSON file.

**Invariant Tests** (`test_scorer_invariants.py`) run before deployment to catch silent training-serving skew. Four assertions are enforced: (1) every model feature must be produced by the scorer's feature dictionary — missing features cause silent 0.0 fill; (2) `time_since_last_transaction` must never be negative (catches clock skew or bad seed data); (3) `time_since_last_transaction` must be positive when a prior timestamp exists in Redis; (4) `hour_deviation_from_norm` must vary with the transaction hour and not be silently zero. These tests caught the `merchant_category` vs `t.merchant_category` naming mismatch at first run.

`scorer.py` is the terminal component of the **Streaming layer**. It consumes from the `raw_transactions` Kafka topic produced by `stream.rs`, reads behavioral context from Redis seeded by `seed_redis.py`, and writes scored decisions to the `fraud_scores` ClickHouse table. It depends on a trained model JSON in `models/` being present and valid before startup.

## Known Issues

The `cf_night_tx_ratio` field is fetched from `cust:{customer_id}:agg` in Redis but is not included in the final feature dictionary returned by `compute_features` in a way that is consumed by the model — the training feature set in `train_xgboost.py` does not include `cf_night_tx_ratio` as a named column. This means the Redis fetch is wasted on every transaction. Auditing the full feature alignment between the training feature list and the inference feature dictionary is required to identify and eliminate all such mismatches.
