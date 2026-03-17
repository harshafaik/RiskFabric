# Real-Time Scoring Service (`scorer.py`)

## Summary
The `scorer.py` service is the production inference engine of RiskFabric. It consumes unlabeled transaction events from Kafka, performs sub-millisecond feature engineering using a Redis-backed feature store, and applies the trained XGBoost model to generate real-time fraud probabilities. The service serves as the final link in the streaming pipeline, providing the "Detection" half of the simulation.

## Architectural Decisions
This service is designed around a **Stateful Micro-Batching Architecture**. To balance high throughput with low latency, feature engineering is performed for each transaction individually, but the final model predictions are grouped into batches of 50. This reduces the overhead of XGBoost inference and ClickHouse persistence while maintaining a P99 latency of approximately 12ms per transaction.

For real-time feature engineering, **Welford’s Algorithm** is implemented to maintain running means and standard deviations within Redis. This allows for the calculation of an "Honest" `amount_deviation_z_score` for every transaction without needing to scan historical Parquet files or perform heavy SQL queries. This stateful approach is critical for simulating how behavioral anomalies are detected on a "live" stream.

The service maintains **Feature Alignment** with the training pipeline by dynamically reordering and casting incoming features to match the exact schema and types (categorical, float, int) exported from the `fraud_model_v1.json` booster. This prevents "training-serving skew," ensuring that the model's performance in production matches its performance during validation.

## System Integration
`scorer.py` sits at the exit point of the **Streaming layer**. It consumes from the `raw_transactions` Kafka topic (populated by `stream.rs`) and writes its decisions to both the `fraud_scores` ClickHouse table and a downstream Kafka topic for automated blocking. It depends on **Redis** for its behavioral context and **ClickHouse** for long-term audit logging and performance monitoring.

## Known Issues
A hardcoded `THRESHOLD = 0.85` is currently used for flagging transactions as fraud. This should be moved to a configuration file (or a dynamic service) to allow for easier tuning of the precision-recall trade-off. Furthermore, the `hour_deviation_from_norm` feature is currently a placeholder (0.0). Implementation of the temporal aggregation logic in `seed_redis.py` and fetching it from Redis is required to ensure the model has access to its full set of behavioral signals during real-time inference.
