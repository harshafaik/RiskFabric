# Machine Learning Model Metrics

This document tracks the performance and evolution of the fraud detection models trained on RiskFabric synthetic data.

## v1 Iteration (Dry Run)
**Date**: February 15, 2026  
**Model Type**: XGBoost (Binary Classifier)  
**Dataset Size**: 358,830 rows (Gold Master Table)  
**Training Target**: `fraud_target` (Ground Truth)

### Training Environment
- **Library**: `xgboost` (via Polars integration)
- **Features**: 19 (including behavioral rates, anomaly flags, and categorical categories)
- **Parameters**: 
  - `n_estimators`: 100
  - `max_depth`: 6
  - `learning_rate`: 0.1
  - `enable_categorical`: True

### Performance Results
- **Accuracy**: 0.95
- **ROC AUC Score**: 0.9782

#### Classification Report (Test Set)
| Class | Precision | Recall | F1-Score | Support |
| :--- | :--- | :--- | :--- | :--- |
| 0 (Legit) | 0.96 | 1.00 | 0.98 | 67,426 |
| 1 (Fraud) | 0.85 | 0.30 | 0.44 | 4,340 |
| **Weighted Avg** | **0.95** | **0.95** | **0.94** | **71,766** |

### Feature Importance (Top 10)
| Feature | Importance Score |
| :--- | :--- |
| `mf_fraud_rate` (Merchant) | 0.3670 |
| `geo_anomaly` | 0.1411 |
| `device_anomaly` | 0.1013 |
| `ip_anomaly` | 0.0920 |
| `amount` | 0.0882 |
| `df_fraud_rate` (Device) | 0.0855 |
| `escalating_amounts_flag` | 0.0491 |
| `cf_fraud_rate` (Customer) | 0.0194 |
| `net_suspicious_cluster_member` | 0.0183 |
| `net_avg_shared_entity_fraud_rate` | 0.0124 |

### Observations
- **Recall Gap**: The model has high precision (0.85) but low recall (0.30) for fraud. This suggests it is conservative—when it flags fraud, it is usually right, but it misses a significant portion of the synthetic fraud patterns.
- **Top Signals**: Merchant-level fraud rates and anomaly flags (`geo`, `device`, `ip`) are the dominant predictors, proving that the Silver layer feature engineering is effective.
- **Next Steps**: Tune hyperparameters (e.g., `scale_pos_weight`) to improve recall on the minority fraud class.
