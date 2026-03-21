# Machine Learning Strategy

## Summary
RiskFabric's machine learning strategy is built around the "Operational Model" philosophy. Instead of training on perfect, latent labels provided by the generator, the strategy forces models to learn from behavioral proxies in a multi-stage pipeline that mirrors real-world deployment challenges.

## Design Intent
The ML pipeline serves as a **Calibration Bench** for the generator. Achieving 100% recall on synthetic data indicates that the fraud signatures are insufficient in complexity. **Label Noise** (FP/FN) and **Sanitized Feature Sets** are explicitly introduced to create a realistic "Information Gap" between the generator and the learner. 

The architecture utilizes **XGBoost** as its primary classifier, leveraging its native categorical handling and gradient-boosting strengths for tabular financial data. This enables researchers to evaluate feature importance in an interpretable manner, identifying which synthetic signals (e.g., spatial velocity vs. amount deviation) are the most predictive.

---

## 🏗️ The Training Pipeline
1.  **Ingestion & ETL**: Data is extracted from the ClickHouse "Gold" layer via `train_xgboost.py`.
2.  **Sanitization**: Internal generator flags (e.g., `fraud_type`, `geo_anomaly`) are dropped to prevent data leakage.
3.  **Training**: XGBoost utilizes a `binary:logistic` objective with a 20% stratified test split.
4.  **Verification**: Models are evaluated against both the noisy `is_fraud` label and the perfect `fraud_target`.

---

## Known Issues
The current use of **Random Stratified Splitting** for validation is an architectural limitation. In a financial stream, data is temporally ordered; random splitting allows for "look-ahead bias," where the model may be exposed to a customer's future patterns during training. Transitioning to **Out-of-Time (OOT) Validation**—training on the first nine months and testing exclusively on the final three—is necessary.

Furthermore, the model is currently **static**, without a "Concept Drift" simulation to account for fraud signatures changing over time. This makes the accuracy metrics potentially misleading as they do not reflect adversarial evolution. Implementing a **Retraining Scheduler** is required to evaluate precision degradation as fraud profiles evolve.
