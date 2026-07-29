# Real-Time Scoring Service

## Overview

`scorer.py` consumes from the `raw_transactions` Kafka topic (produced by `stream.rs`), computes behavioral features via Redis, and applies the XGBoost model to produce per-transaction fraud probabilities written to ClickHouse `fraud_scores`. Auto-discovers the latest model JSON from `models/` via `model_utils.load_model()`.

## Schema

### ClickHouse `fraud_scores`

| Field | Type | Description |
| :--- | :--- | :--- |
| `transaction_id`, `card_id`, `customer_id` | `String` | Entity keys |
| `amount` | `Float64` | Transaction amount |
| `timestamp` | `DateTime` | Transaction timestamp from Kafka |
| `kafka_received_at`, `scored_at` | `DateTime64(3)` | Consumer receive time; prediction completion time |
| `fraud_probability` | `Float64` | XGBoost output [0.0, 1.0] |
| `flagged` | `UInt8` | 1 if `fraud_probability >= flagging_threshold` |

### Redis Keys

| Key Pattern | Type | Purpose |
| :--- | :--- | :--- |
| `cust:{id}:stats` | Hash (`count`, `mean`, `M2`) | Welford accumulator for amount z-score |
| `cust:{id}:agg` | Hash (`fraud_rate`, `mean_hour`) | Pre-seeded customer aggregates |
| `card:{id}:last_ts` | String (Unix ts) | Previous transaction timestamp |
| `card:{id}:burst` | Sorted Set (ts → score) | 60s sliding window for rapid-fire detection |
| `card:{id}:history` | List (JSON, max 10) | Last 10 txns for category switch / escalating amounts |
| `card:{id}:loc` | Hash (`lat`, `lon`) | Previous coordinates for spatial velocity |
| `card:{id}:seq` | Integer | Per-card transaction sequence counter |
| `merch:{id}:agg` | Hash | Pre-seeded merchant aggregates |

## Architecture

### Stateful Micro-Batching
Transactions consumed one at a time, features computed per-event. Predictions deferred until 50 records accumulate, then single `model.predict_proba` + single `ch.insert` to ClickHouse. Amortizes inference and network overhead.

### Welford's Online Algorithm
`amount_deviation_z_score` is computed via running `count`, `mean`, `M2` in Redis `cust:{id}:stats`. Each event: fetch state → compute z-score → incorporate new amount → write back. No historical scan needed.

### Rapid-Fire Detection
Each transaction adds to `card:{id}:burst` sorted set with Unix timestamp as score. `ZREMRANGEBYSCORE` removes entries >60s old. Flag fires when >3 members remain. O(log N), no background cleanup.

### Feature Alignment
Reads booster `feature_names`/`feature_types` pre-prediction. Missing columns filled 0.0. Columns cast to expected dtype (`c`→category, `float`→float32, `int`→int32). DataFrame reordered to match. Fallback: 0.0 for all records on failure.

### Config-Driven Threshold
Loaded from `data/config/runtime_thresholds.json` (produced by `compute_thresholds.py`). Falls back to `0.85` with warning if missing. Validated threshold as of 2026-07-20: `0.8849`.

### Invariant Tests
`test_scorer_invariants.py` enforces: (1) all model features produced; (2) `time_since_last_transaction` ≥ 0; (3) positive when prior ts exists; (4) `hour_deviation_from_norm` varies with hour. Caught `merchant_category` vs `t.merchant_category` mismatch at first run.
