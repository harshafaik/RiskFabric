# Machine Learning Systems

This section documents the model training pipelines, real-time inference services, and metadata utilities required for detecting synthetic fraud patterns.

## Python ML Pipeline (`src/ml/`)

```d2
direction: down

classes: {
  train: {
    style.fill: "#28201b"
    style.stroke: "#4c392c"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
  score: {
    style.fill: "#2e1f26"
    style.stroke: "#573a46"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
  ui: {
    style.fill: "#251e36"
    style.stroke: "#483a68"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
  ops: {
    style.fill: "#1c2423"
    style.stroke: "#2e403d"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
  eval: {
    style.fill: "#1b2a3a"
    style.stroke: "#304e70"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
}

TRAIN: "train_xgboost.py\nTrain classifier on\nDuckDB/Parquet Gold" {
  class: train
}
CALIB: "calibrate_model.py\nPlatt / Isotonic\ncalibration" {
  class: eval
}
THRESH: "compute_thresholds.py\nPrecision-recall\nthreshold opt" {
  class: eval
}
SHAP: "shap_analysis.py\nGlobal feature\nimportance" {
  class: eval
}
LocalSHAP: "local_shap_explanation.py\nPer-transaction\nSHAP values" {
  class: eval
}
LEAK: "verify_leakage.py\nFeature leakage\naudit" {
  class: eval
}
DRIFT: "drift_simulation.py\nData drift\ntesting" {
  class: eval
}
DEPTH: "evaluate_model_depth.py\nDepth-based\neval" {
  class: eval
}
DUMP: "dump_model.py\nModel serialization" {
  class: eval
}

TRAIN -> CALIB -> THRESH -> SHAP -> LocalSHAP -> LEAK -> DRIFT -> DEPTH -> DUMP

SEED: "seed_redis.py\nPre-seed Redis from\nDuckDB/Parquet" {
  class: score
}
SCORER: "scorer.py\nReal-time Kafka consumer\n→ XGBoost → fraud_scores" {
  class: score
  style.stroke-width: 2
}
DASH: "Grafana\nReal-time monitoring\n(3000)" {
  class: ui
}
SEED -> SCORER -> DASH

CASES: "ingest_cases.py\nfraud_scores → Postgres\ncases table (OLTP)" {
  class: ops
}
SCORER -> CASES

G1M: "generate_1m_transactions.py\nLarge-scale gen\nscript" {
  class: score
}
G2M: "generate_and_score.py\nCombined gen +\nscoring" {
  class: score
}
G1M -- G2M
```
