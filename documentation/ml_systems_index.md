# Machine Learning Systems

This section documents the model training pipelines, real-time inference services, and metadata utilities required for detecting synthetic fraud patterns.

## Python ML Pipeline (`src/ml/`)

```mermaid
flowchart TB
    classDef script fill:#22252a,stroke:#4d535b,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    classDef store fill:#1b2a3a,stroke:#304e70,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    subgraph TRAIN["Training & Evaluation Pipeline"]
        direction TB
        subgraph T1["Training & Calibration"]
            direction LR
            TRAIN_PY["train_xgboost.py<br/>XGBoost Model"]:::script --> CAL["calibrate_model.py<br/>Platt / Isotonic"]:::script --> THRESH["compute_thresholds.py<br/>PR Threshold Opt"]:::script
        end
        subgraph T2["Explainability & Export"]
            direction LR
            SHAP["shap_analysis.py<br/>Global / Local SHAP"]:::script --> LOCAL_SHAP["verify_leakage.py<br/>Leakage & Drift Audit"]:::script --> DUMP["dump_model.py<br/>Model Serialization"]:::script
        end
        T1 --> T2
    end
    MODEL[("Serialized Model<br/>Artifact")]:::store
    DUMP --> MODEL

    subgraph SERVING["Real-time Serving Pipeline"]
        direction LR
        SEED["seed_redis.py<br/>Pre-seed Redis"]:::script --> SCORER["scorer.py<br/>Kafka → XGBoost"]:::script --> INGEST["ingest_cases.py<br/>ClickHouse → Postgres"]:::script
    end
    MODEL --> SCORER

    subgraph BATCH["Batch Utilities"]
        direction LR
        GEN_1M["generate_1m_transactions.py<br/>Large-scale Gen"]:::script
        GEN_SCORE["generate_and_score.py<br/>Combined Pipeline"]:::script
    end
    MODEL -.-> GEN_SCORE

    style TRAIN fill:#28201b,stroke:#4c392c,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
    style T1 fill:none,stroke:none,color:#cfd2d9
    style T2 fill:none,stroke:none,color:#cfd2d9
    style SERVING fill:#1c2423,stroke:#2e403d,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
    style BATCH fill:#231e2d,stroke:#3f3354,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
```

**<a id="fig-7"></a>Figure 7:** Python ML Pipeline

## Modules

- [Training Pipeline](components/ml_training.md)
- [Model Calibration](components/ml_calibration.md)
- [Real-Time Scoring Service](components/realtime_scorer.md)
- [Model Explainability (SHAP)](components/ml_explainability.md)
- [Model Robustness & Drift](components/ml_robustness.md)
- [Analysis & Utility Scripts](ml_analysis_scripts.md)
