# ML Analysis & Utility Scripts

## Summary
Beyond the core training (`src/ml/train_xgboost.py`) and scoring (`src/ml/scorer.py`) pipelines, the `src/ml/` directory contains a suite of analysis, calibration, evaluation, and data generation scripts. Each operates independently against DuckDB/Parquet Gold snapshots or the XGBoost model artifacts in `models/`.

## Script Reference

### `src/ml/compute_thresholds.py`
Calculates precision-recall trade-off operating points for the fraud model. Queries the Gold Parquet snapshot via DuckDB, loads the isotonic calibrated model, computes precision-recall curves, and writes `data/config/runtime_thresholds.json` — which contains the `flagging_threshold` at ~50% recall plus operational layer boundaries — for consumption by `src/ml/scorer.py`.

**Consumes:** `models/calibrated_fraud_model_isotonic.pkl`, Gold Parquet snapshot.
**Produces:** `data/config/runtime_thresholds.json` (runtime config), operational threshold report.

### `src/ml/verify_leakage.py`
Validates that training-serving feature alignment is maintained. Queries the Gold Parquet snapshot via DuckDB, reconstructs features using the same transform pipeline as `src/etl/features/`, and compares against the stored values. Detects mismatches indicating training-serving skew.

**Consumes:** Latest model JSON from `models/` (auto-discovered via `model_utils`), Gold Parquet snapshot.
**Produces:** Leakage verification report.

### `src/ml/test_scorer_invariants.py`
Pre-deployment invariant tests for the real-time scorer. Four assertions catch silent training-serving skew: (1) every model feature must be produced by the scorer's feature dictionary; (2) `time_since_last_transaction` must never be negative; (3) `time_since_last_transaction` must be positive when a prior timestamp exists in Redis; (4) `hour_deviation_from_norm` must vary with the transaction hour. Uses mocked Redis — no live infrastructure required.

**Consumes:** Latest model JSON from `models/` (auto-discovered via `model_utils`).
**Produces:** Test pass/fail report via pytest.

### `src/ml/test_model.py`
Lightweight model smoke test. Loads the trained model, reads the Gold Parquet snapshot via DuckDB, runs inference on the full dataset, and outputs a sklearn `classification_report` with AUC, confusion matrix, and per-class precision/recall. Used as a sanity check after training or before deploying a new model version.

**Consumes:** Latest model JSON from `models/` (auto-discovered via `model_utils`), Gold Parquet snapshot.
**Produces:** Classification report.

### `src/ml/generate_and_score_transactions.py`
End-to-end pipeline harness. Generates synthetic transactions (using Python-side logic that mirrors the Rust `generate.rs` binary), scores them through the trained XGBoost model, seeds Redis with customer running statistics for the real-time scorer, and writes flagged cases to OLTP Postgres. Used for local, non-Rust integration testing.

**Consumes:** Redis, OLTP Postgres, latest model JSON from `models/` (auto-discovered via `model_utils`).
**Produces:** Scored transactions in Postgres `cases` table, Redis customer stats.

### `src/ml/generate_1m_transactions.py`
Bulk transaction generator for load testing. Generates 1 million synthetic transactions via Python-side logic, writes them to Redis and directly to the OLTP Postgres database. Used to stress-test the real-time scorer and case admin ingestion pipeline at scale.

**Consumes:** Redis, OLTP Postgres.
**Produces:** 1M transactions in Redis and Postgres.

### `src/ml/dashboard.py` (DEPRECATED)
Replaced by Grafana (`docker/grafana/dashboards/fraud-monitoring.json`). Reads from ClickHouse (`fraud_scores` table) to display live fraud score distributions, volume trends, and alert dashboards. The file is kept for reference only.

**Consumes:** ClickHouse `fraud_scores` table.
**Produces:** Grafana web dashboard on port 3000 (replaced Streamlit on port 8501).

### `src/ml/dump_model.py`
Inspects a trained XGBoost model to verify the feature schema before deployment. Reads the latest model JSON from `models/` via `model_utils`, prints `feature_names` and `feature_types` from the booster object, and extracts categorical level encodings from the JSON.

**Consumes:** Latest model JSON from `models/` (auto-discovered via `model_utils`).

## Pipeline Position
All analysis scripts read from Gold Parquet snapshots via DuckDB and the trained model artifacts in `models/`. They operate independently of the streaming pipeline and do not write back to Redis. The `generate_and_score_transactions.py` and `generate_1m_transactions.py` scripts are the exception — they write directly to Postgres and Redis for integration testing and load testing.
