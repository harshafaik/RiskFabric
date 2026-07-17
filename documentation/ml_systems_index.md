# Machine Learning Systems

This section documents the model training pipelines, real-time inference services, and metadata utilities required for detecting synthetic fraud patterns.

## Python ML Pipeline (`src/ml/`)

**Training & Evaluation Flow:**

```
train_xgboost.py
    │
    ▼
calibrate_model.py (Platt / Isotonic)
    │
    ▼
compute_thresholds.py (Precision-recall threshold opt)
    │
    ▼
shap_analysis.py (Global feature importance)
    │
    ▼
local_shap_explanation.py (Per-transaction SHAP)
    │
    ▼
verify_leakage.py (Feature leakage audit)
    │
    ▼
drift_simulation.py (Data drift testing)
    │
    ▼
evaluate_model_depth.py (Depth-based eval)
    │
    ▼
dump_model.py (Model serialization)
```

**Real-time Serving:**

```
seed_redis.py  ─►  scorer.py  ─►  Grafana (monitoring)
(Pre-seed Redis)   (Kafka → XGBoost)
                        │
                        ▼
                   ingest_cases.py
                   (ClickHouse → Postgres OLTP)
```

**Batch Utilities:**

```
generate_1m_transactions.py  ←─→  generate_and_score.py
(Large-scale gen)                  (Combined gen + scoring)
```
