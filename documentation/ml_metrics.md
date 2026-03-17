# Machine Learning Model Metrics

This document tracks the performance and evolution of the fraud detection models trained on RiskFabric synthetic data.

## v1 Iteration (Dry Run)
**Date**: February 15, 2026  
**Model Type**: XGBoost (Binary Classifier)  
**Dataset Size**: 358,830 rows (Gold Master Table)  
**Training Target**: `fraud_target` (Ground Truth)

### Performance Results
- **Accuracy**: 0.95
- **ROC AUC Score**: 0.9782

#### Classification Report (Test Set)
| Class | Precision | Recall | F1-Score | Support |
| :--- | :--- | :--- | :--- | :--- |
| 0 (Legit) | 0.96 | 1.00 | 0.98 | 67,426 |
| 1 (Fraud) | 0.85 | 0.30 | 0.44 | 4,340 |

### Observations
- **Recall Gap**: Low recall (0.30) for fraud suggests the model misses significant synthetic patterns.
- **Top Signals**: Merchant-level fraud rates and anomaly flags are dominant predictors.

---

## v2 High-Fidelity Iteration (Leakage Detected)
**Date**: February 13, 2026 (Full 100k run)  
**Model Type**: XGBoost (Binary Classifier)  
**Dataset Size**: 4,391,523 rows (Gold Master Table)  
**Training Target**: `fraud_target` (Ground Truth)

### Performance Results
- **Accuracy**: 0.99
- **ROC AUC Score**: 0.9993

#### Classification Report (Test Set)
| Class | Precision | Recall | F1-Score | Support |
| :--- | :--- | :--- | :--- | :--- |
| 0 (Legit) | 0.99 | 1.00 | 1.00 | 807,054 |
| 1 (Fraud) | 0.97 | 0.93 | 0.95 | 71,251 |

---

## v2 Iteration (Leakage Prevention)
**Date**: February 13, 2026  
**Model Type**: XGBoost (Binary Classifier)  
**Dataset Size**: 4,391,523 rows (Gold Master Table)  
**Training Target**: `is_fraud` (Noisy Label)

### Performance Results
- **Accuracy**: 0.97
- **ROC AUC Score**: 0.9746

#### Classification Report (Noisy Labels)
| Class | Precision | Recall | F1-Score | Support |
| :--- | :--- | :--- | :--- | :--- |
| 0 (Legit) | 0.97 | 0.99 | 0.98 | 795,892 |
| 1 (Fraud) | 0.92 | 0.72 | 0.81 | 82,413 |

### Observations: How Leakage was Prevented
1.  **Sanitized Feature Vector**: Explicitly excluded synthetic metadata fields (`geo_anomaly`, `device_anomaly`, `ip_anomaly`, `label_noise`, `fraud_type`, `fraud_target`, `burst_session`, `burst_seq`) from the training set.
2.  **Noisy Target Shift**: Shifted the training target from the perfect `fraud_target` to the noisy `is_fraud` label.
3.  **Behavioral Validation**: The model now identifies fraud based on behavioral signals like `amount` (36% importance), `mf_fraud_rate` (33%), and `df_fraud_rate` (16%).

---

## v3 Streaming Verification (Real-Time Performance)
**Date**: March 15, 2026  
**Inference Engine**: `scorer.py` (Real-time scoring via Kafka)  
**Throughput**: 100 tx/s (Rate-limited)  
**Test Window**: 1 hour (360,000 transactions)

### Performance Results (Ground Truth vs. Model Score)
| Metric | Performance |
| :--- | :--- |
| Precision | 0.91 |
| Recall | 0.68 |
| F1-Score | 0.78 |
| Latency (P99) | 12ms |

### Observations: Real-Time Performance vs. Batch
- **Recall Dip**: The recall in the streaming environment (0.68) is slightly lower than the batch environment (0.72). This is likely due to the **Feature Consistency Gap**—some features in Redis (from `seed_redis.py`) may be slightly stale compared to the real-time transaction stream.
- **Latency Benchmarks**: The P99 latency of 12ms (including Redis feature lookup and XGBoost inference) confirms that the Python-based scorer is capable of maintaining 100 tx/s on standard hardware.
- **Ground Truth Capture**: The verification mode successfully captured `ground_truth.csv`, allowing for direct row-by-row accuracy validation without manual labeling.
