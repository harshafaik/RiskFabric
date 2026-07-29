# Machine Learning Training Pipeline

## Overview

The training pipeline (`train_xgboost.py` and `dump_model.py`) trains and inspects the XGBoost fraud classifier that powers the real-time scoring service. It reads the Gold Parquet snapshot via DuckDB into a Polars DataFrame, applies an operational feature policy to select 10 behavioral features, trains an `XGBClassifier` with class-imbalance weighting and proper categorical encoding, and serializes the resulting model to `models/fraud_model_v4.json`. `dump_model.py` is a post-training inspection utility that reads the serialized booster to verify the feature schema before the model is deployed to `scorer.py`. All downstream scripts use `model_utils.load_model()` or `model_utils.get_model_features()` to auto-discover the latest model and derive the feature list — no manual renaming or hardcoded feature lists are required.

## Schema

### Training Feature Set

| Feature | Type | Description |
| :--- | :--- | :--- |
| `time_since_last_transaction` | `float` | Temporal — seconds since prior transaction per card. |
| `transaction_sequence_number` | `int` | Temporal — cumulative transaction count per customer. |
| `spatial_velocity` | `float` | Spatial — estimated travel speed in km/h between consecutive transactions. |
| `hour_deviation_from_norm` | `float` | Temporal — deviation from the customer's mean transaction hour. |
| `amount_deviation_z_score` | `float` | Behavioral — per-customer z-score of the transaction amount. |
| `rapid_fire_transaction_flag` | `int` | Temporal — 1 if time since last transaction ≤ 300 seconds. |
| `escalating_amounts_flag` | `int` | Behavioral — 1 if the last three transactions show strictly increasing amounts. |
| `merchant_category_switch_flag` | `int` | Behavioral — 1 if merchant category differs from the prior transaction. |
| `transaction_channel` | `categorical` | Channel — e.g., `"upi"`, `"cards"`, `"online"`. |
| `card_present` | `int` | Channel — 1 if physical card was present. |
| `merchant_category` | `categorical` | Channel — standardized merchant category string. |

The following columns are **explicitly excluded** from training despite being present in `fact_transactions_gold`: `geo_anomaly`, `device_anomaly`, `ip_anomaly`, `fraud_type`, `fraud_target`, `campaign_id`, and all UUID identifier columns. These are generator-internal labels that would constitute direct data leakage — the model would learn to detect injected signals rather than behavioral anomalies.

### `XGBClassifier` Hyperparameters

| Parameter | Value | Rationale |
| :--- | :--- | :--- |
| `n_estimators` | `100` | Fixed tree count; not tuned. |
| `max_depth` | `6` | Fixed tree depth; not tuned. |
| `learning_rate` | `0.1` | Fixed shrinkage factor; not tuned. |
| `objective` | `binary:logistic` | Binary fraud classification. |
| `tree_method` | `hist` | Histogram-based split finding for performance on large datasets. |
| `enable_categorical` | `False` | Disabled; categoricals are pre-encoded to ordinals before XGBoost sees them. |
| `eval_metric` | `aucpr` | Area under precision-recall curve, appropriate for imbalanced fraud datasets. |
| `scale_pos_weight` | `legitimate_count / fraud_count` | Computed per run from the training split to compensate for class imbalance. |
| `random_state` | `42` | Fixed seed for reproducibility. |

**Operational Feature Policy** is the primary leakage-prevention mechanism. All columns that encode generator ground truth — `geo_anomaly`, `device_anomaly`, `ip_anomaly`, `fraud_type`, `fraud_target` — are excluded from `feature_cols` by explicit omission, not by filtering. The model is trained exclusively on features that would be observable in a real payment system at transaction time. This ensures that reported performance metrics reflect genuine behavioral detectability rather than the model learning internal simulation flags.

**Dynamic Class Weighting** is applied at training time via `scale_pos_weight`, computed as the ratio of legitimate to fraudulent samples in the training split. This is recalculated on every run, meaning the weight adjusts automatically as the fraud injection rate in the synthetic population changes between generation runs. No fixed weight is hardcoded.

**Chronological 80/20 Split** replaces the previous random stratified split. The Gold DataFrame is sorted by `timestamp` ascending, and the first 80% of rows (chronologically) form the training set while the last 20% form the test set. This ensures the model never sees future fraud patterns during training and produces metrics representative of deployment performance. The `split_by_timestamp()` utility in `ml_utils.py` is used consistently across `train_xgboost.py`, `calibrate_model.py`, `drift_simulation.py`, `test_model.py`, and `evaluate_model_depth.py`.

**Model Inspection via `dump_model.py`** is the post-training verification step. After `train_xgboost.py` saves a date-stamped model, `dump_model.py` loads the booster and prints `feature_names` and `feature_types` directly from the booster object. It also parses the `learner.gradient_booster.model.cats` block in the JSON to extract the categorical level encodings the model observed during training. This provides a verifiable source of truth for the feature schema that `scorer.py` uses to reorder and type-cast its inference DataFrames.

**Validated Performance** (2026-07-20, snapshot `20260716_145639`, 1,079,098 train / 154,157 cal / 308,314 test, chronological 70/10/20 split, 10 behavioral features):

| Metric | Uncalibrated | Platt | Isotonic |
|---|---|---|---|
| ROC-AUC | 0.7622 | 0.7622 | 0.7622 |
| PR-AUC | 0.3293 | 0.3293 | 0.3293 |
| Brier Loss | 0.1385 | — | — |
| ECE (on held-out test) | 0.3516 | 0.0036 | 0.0003 |
| ECE (on cal set) | 0.3435 | 0.0034 | 0.0000 |

ECE of 0.0003 was validated on the completely held-out last 20% chronological — isotonic calibrator fit on middle 10%, evaluated on entirely unseen data. The near-zero ECE is genuine, not overfitting.

**Precision-first operating point:** The old 97.5%-precision figure was inflated by random-split leakage. Under honest chronological evaluation, the model cannot sustain that precision. The closest precision-first posture is **~80% precision at threshold 0.885**, catching ~1.5% of fraud with ~25.9 alerts per 100K transactions per day — 4 of 5 flagged transactions are genuine fraud.

These numbers reflect true generalization on unseen future data — no random split, no target encoding, no join-ordering artifacts. Earlier reported values (0.925 → 0.798 → 0.786) were each inflated by successively removed data leakage sources. See `documentation/feature_leakage_issues.md` §7 for the split leakage analysis.

`train_xgboost.py` is the terminal consumer of the **Data Warehouse layer** and the entry point to the **Machine Learning layer**. It reads from `fact_transactions_gold` (produced by `etl.rs`) and writes a date-stamped model JSON. Downstream scripts auto-discover the latest model via `model_utils.load_model()` — no manual renaming or path hardcoding is required.
