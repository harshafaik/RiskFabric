# Machine Learning Training Pipeline (`train_xgboost.py`)

## Summary
The `train_xgboost.py` script is the primary model development engine for RiskFabric. It extracts features from the ClickHouse "Gold" layer and trains an XGBoost classifier to detect synthetic fraud patterns. It evaluates the learnability of the generated fraud signatures by industry-standard algorithms.

## Architectural Decisions
An **"Operational Feature" policy** is implemented in the training script to prevent data leakage. While the synthetic generator provides explicit labels like `geo_anomaly` and `fraud_type` for verification, these are strictly excluded from training. Instead, the model is forced to learn from behavioral proxies such as `amount_deviation_z_score`, `spatial_velocity`, and `hour_deviation_from_norm`. This ensures that the model's performance reflects real-world detectability rather than just learning internal generator flags.

The choice of **XGBoost with Native Categorical Support** allows the model to process high-cardinality fields like `merchant_category` and `transaction_channel` directly, without the memory overhead of one-hot encoding. This maintains performance as the synthetic merchant population scales.

## System Integration
The training pipeline is the final "offline" consumer of the **Data Warehouse layer**. It uses the `clickhouse-connect` library to pull data directly into Polars DataFrames for training. The resulting model is serialized to `models/fraud_model_v1.json`, which is consumed by the `scorer.py` service for real-time inference in the streaming pipeline.

## Known Issues
A simple 80/20 train/test split with stratification is currently used, but **time-series validation** is missing. Since fraud patterns evolve over time, a random split can lead to optimistic performance estimates by allowing the model to see future patterns during training. A walk-forward validation strategy is required to better simulate real production deployments. Additionally, XGBoost hyperparameters (like `max_depth=6`) are currently hardcoded; these should be moved to a `ml_tuning.yaml` configuration file to allow for automated hyperparameter optimization.
