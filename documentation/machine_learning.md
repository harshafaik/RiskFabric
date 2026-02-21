# Machine Learning Pipeline

RiskFabric uses **XGBoost**, a high-performance gradient boosting framework, to train fraud detection models on the generated synthetic data. The pipeline is designed to simulate a real-world environment where labels are noisy and ground-truth metadata is unavailable at inference time.

## 🧠 Model Architecture

- **Engine**: XGBoost (Binary Classifier)
- **Frameworks**: Python, Polars, Scikit-Learn
- **Input**: Gold Master Table (`fact_transactions_gold`)
- **Target**: `is_fraud` (**Noisy Label**)

## 🛡️ Leakage Prevention Strategy

A critical part of the RiskFabric ML pipeline is ensuring the model does not "cheat" by using synthetic markers that would not exist in a real production system.

### 1. The Sanitized Feature Vector
We explicitly exclude ground-truth metadata from the training set:
- **Excluded**: `geo_anomaly`, `device_anomaly`, `ip_anomaly`, `fraud_type`, `fraud_target`.
- **Reasoning**: These are "generator flags." If included, the model achieves a ~0.99 AUC but fails to learn actual human behavior (amounts, locations, timing).

### 2. Noisy Label Training
Instead of training on the perfect `fraud_target` (ground truth), the model trains on `is_fraud`. This label incorporates simulated **label noise** (False Positives and False Negatives), forcing the model to be robust against real-world data imperfections.

---

## 🚀 Training Workflow

The training script (`src/ml/train_xgboost.py`) performs the following steps:

1.  **Data Loading**: Connects to ClickHouse via `clickhouse_connect` and streams the Gold Master Table into a Polars DataFrame.
2.  **Categorical Encoding**: Utilizes Polars' native `Categorical` type to handle high-cardinality features like `merchant_category` and `transaction_channel`.
3.  **Stratified Split**: Performs an 80/20 train-test split, maintaining the low fraud prevalence ratio in both sets.
4.  **Training**: Executes an XGBoost Classifier with `tree_method='hist'` and native categorical support enabled.
5.  **Evaluation**: Calculates ROC AUC and generates a detailed classification report.

## 📊 Feature Importance

The model learns to identify fraud primarily through behavioral signals engineered in the Silver ETL layer:
- **Transaction Amount**: The dominant predictor for UPI and high-value scams.
- **Merchant Reputation**: Leverages the historical fraud rates of specific merchants.
- **Entity sharing**: Identifies high-risk IP and Device clusters.

## 📁 Model Artifacts
Trained models are saved in JSON format in the `/models` directory for versioning and inference:
```bash
models/fraud_model_v1.json
```
