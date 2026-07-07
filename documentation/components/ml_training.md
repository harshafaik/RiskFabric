# Machine Learning Training Pipeline

## Overview

The training pipeline (`train_xgboost.py` and `dump_model.py`) trains and inspects the XGBoost fraud classifier that powers the real-time scoring service. It reads the full `fact_transactions_gold` table from ClickHouse via `clickhouse-connect` into a Polars DataFrame, applies an operational feature policy to select 12 behavioral features, trains an `XGBClassifier` with class-imbalance weighting, and serializes the resulting model to `models/fraud_model_v1.json`. `dump_model.py` is a post-training inspection utility that reads the serialized booster to verify the feature schema before the model is deployed to `scorer.py`.

## Schema

### Training Feature Set

| Feature | Type | Source Column in Gold Table |
| :--- | :--- | :--- |
| `time_since_last_transaction` | `float` | Sequence feature — seconds since prior transaction per card. |
| `transaction_sequence_number` | `int` | Sequence feature — cumulative transaction count per customer. |
| `spatial_velocity` | `float` | Sequence feature — estimated travel speed in km/h between consecutive transactions. |
| `hour_deviation_from_norm` | `float` | Sequence feature — deviation from the customer's mean transaction hour. |
| `amount_deviation_z_score` | `float` | Sequence feature — per-customer z-score of the transaction amount. |
| `rapid_fire_transaction_flag` | `int` | Sequence feature — 1 if time since last transaction ≤ 300 seconds. |
| `escalating_amounts_flag` | `int` | Sequence feature — 1 if the last three transactions show strictly increasing amounts. |
| `merchant_category_switch_flag` | `int` | Sequence feature — 1 if merchant category differs from the prior transaction. |
| `transaction_channel` | `categorical` | Transaction context — e.g., `"upi"`, `"cards"`, `"online"`. |
| `card_present` | `int` | Transaction context — 1 if physical card was present. |
| `merchant_category` | `categorical` | Transaction context — standardized merchant category string. |
| `suspicious_cluster_member` | `int` | Network feature — currently hardcoded to 0 in ETL; included for schema compatibility. |

The following columns are **explicitly excluded** from training despite being present in `fact_transactions_gold`: `geo_anomaly`, `device_anomaly`, `ip_anomaly`, `fraud_type`, `fraud_target`, `campaign_id`, and all UUID identifier columns. These are generator-internal labels that would constitute direct data leakage — the model would learn to detect injected signals rather than behavioral anomalies.

### `XGBClassifier` Hyperparameters

| Parameter | Value | Rationale |
| :--- | :--- | :--- |
| `n_estimators` | `100` | Fixed tree count; not tuned. |
| `max_depth` | `6` | Fixed tree depth; not tuned. |
| `learning_rate` | `0.1` | Fixed shrinkage factor; not tuned. |
| `objective` | `binary:logistic` | Binary fraud classification. |
| `tree_method` | `hist` | Histogram-based split finding for performance on large datasets. |
| `enable_categorical` | `True` | Native categorical support; `transaction_channel` and `merchant_category` are cast to `pl.Categorical` before training. |
| `eval_metric` | `aucpr` | Area under precision-recall curve, appropriate for imbalanced fraud datasets. |
| `scale_pos_weight` | `legitimate_count / fraud_count` | Computed per run from the training split to compensate for class imbalance. |
| `random_state` | `42` | Fixed seed for reproducibility. |

**Operational Feature Policy** is the primary leakage-prevention mechanism. All columns that encode generator ground truth — `geo_anomaly`, `device_anomaly`, `ip_anomaly`, `fraud_type`, `fraud_target` — are excluded from `feature_cols` by explicit omission, not by filtering. The model is trained exclusively on features that would be observable in a real payment system at transaction time. This ensures that reported performance metrics reflect genuine behavioral detectability rather than the model learning internal simulation flags.

**Dynamic Class Weighting** is applied at training time via `scale_pos_weight`, computed as the ratio of legitimate to fraudulent samples in the training split. This is recalculated on every run, meaning the weight adjusts automatically as the fraud injection rate in the synthetic population changes between generation runs. No fixed weight is hardcoded.

**Stratified 80/20 Split** is used for train/test partitioning with `random_state=42`. The `stratify` argument ensures that the fraud prevalence rate is preserved in both splits. The test set is used only for computing the final ROC AUC score and top-10 feature importances printed to stdout; no model selection or threshold tuning is performed on it.

**Model Inspection via `dump_model.py`** is the post-training verification step. After `train_xgboost.py` saves `models/fraud_model_v1.json`, `dump_model.py` loads the booster and prints `feature_names` and `feature_types` directly from the booster object. It also parses the `learner.gradient_booster.model.cats` block in the JSON to extract the categorical level encodings the model observed during training. This provides a verifiable source of truth for the feature schema that `scorer.py` uses to reorder and type-cast its inference DataFrames.

`train_xgboost.py` is the terminal consumer of the **Data Warehouse layer** and the entry point to the **Machine Learning layer**. It reads from `fact_transactions_gold` (produced by `etl.rs`) and writes `models/fraud_model_v1.json`, which is consumed by `scorer.py` for real-time inference.

## Known Issues

All XGBoost hyperparameters — `n_estimators=100`, `max_depth=6`, `learning_rate=0.1` — are hardcoded directly in the training function with no configuration file. This prevents hyperparameter search without modifying source code. Moving these to `ml_tuning.yaml` and wiring in a sweep library (e.g., `optuna`) is required before the model can be systematically optimized.

A random 80/20 stratified split is used rather than a time-based split. Because the synthetic dataset spans 365 days and fraud campaigns evolve over that period, a random split allows the model to see future fraud patterns during training. This produces optimistically biased performance metrics. A walk-forward or expanding-window validation strategy aligned to the transaction timestamp is required to produce metrics that are representative of deployment performance.

`dump_model.py` extracts categorical levels from the XGBoost JSON using `re.findall` on the raw JSON string of the `cats` block. This is an unreliable parser that depends on the specific serialization format of the installed XGBoost version and will break silently if the internal JSON schema changes across library versions. The utility also only prints to stdout — it produces no machine-readable output. Replacing the regex approach with a proper JSON path traversal and exporting a structured `schema.yaml` would allow `scorer.py` to load the feature schema automatically rather than relying on the booster's `feature_names` attribute at runtime.
