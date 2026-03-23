# Machine Learning Metrics & Model Progression

This document tracks the performance and evolution of the fraud detection models trained on RiskFabric synthetic data, progressing from initial leakage-prone baselines to a robust, behavioral production configuration.

---

## Section 1: Early Iterations

The development process began with basic feature sets to establish a baseline for fraud detection performance.

### v1 Iteration (Baseline)
The initial model established core feature sets including amount deviations and spatial velocity on a sample population.

*   **Accuracy**: 0.95
*   **ROC AUC Score**: 0.9782
*   **Recall (Fraud)**: 0.30 (Identified significant "Recall Gap")

### v2 High-Fidelity (Leakage Detected)
Scaling to larger datasets revealed massive performance inflation due to generator artifacts in metadata fields.

*   **ROC AUC Score**: 0.9993
*   **Leakage Identified**: Synthetic metadata fields (`fraud_target`, `burst_seq`) were providing a "static bypass" for the model.

### v2 Iteration (Leakage Prevention)
The feature vector was sanitized to exclude metadata, shifting the focus to behavioral signals.

*   **ROC AUC Score**: 0.9746
*   **Recall (Noisy Labels)**: 0.72
*   **Sanitization**: Transitioned from `fraud_target` to the noisy `is_fraud` label.

**Note**: In addition to the leakage issues documented below, v1 and v2 iterations were trained on an incomplete feature set. Behavioral features computed in the Rust ETL layer — including `amount_deviation_z_score`, `spatial_velocity`, and granular anomaly flags — were silently dropped before reaching XGBoost due to a narrow Gold table join. The inflated AUC figures in these iterations reflect both metadata leakage and the absence of the features that would have provided genuine behavioral signal.

## Section 2: v3 — Production Configuration (Final)

The final model configuration focuses on pure behavioral signals, specifically tuned to handle the extreme class imbalance (1.4% fraud rate) found in realistic production environments.

### Training Setup
*   **Dataset:** 1.5M transactions (Seed 42).
*   **Fraud Rate:** 1.41% (`target_share`: 0.01, `fp_rate`: 0.005).
*   **Model:** XGBoost binary classifier.
*   **Scale Pos Weight:** 69.57 (Computed dynamically from training imbalance).
*   **Eval Metric:** `aucpr` (Area Under Precision-Recall Curve).
*   **Label Noise:** 0.5% False Positives and 1% False Negatives deliberately injected.
*   **Theoretical Recall Ceiling:** 66.7% (Derived from the intentional label noise ratio).

### Feature Importance
The model prioritizes physical and financial anomalies over static identifiers.

| Feature | Importance | Description |
|:--- | :--- | :--- |
| `spatial_velocity` | 25.38% | Impossible travel speed between transactions |
| `amount_deviation_z_score` | 20.80% | Spending magnitude relative to customer norm |
| `time_since_last_transaction` | 12.72% | Temporal burst and frequency detection |
| `transaction_channel` | 11.60% | Risk associated with specific payment methods |
| `merchant_category` | 11.08% | Contextual risk of the merchant type |
| `hour_deviation_from_norm` | 7.40% | Circadian rhythm anomalies |
| `merchant_category_switch_flag` | 2.89% | Unexpected shifts in merchant category |
| `card_present` | 2.45% | Physical vs. digital transaction risk |
| `transaction_sequence_number` | 1.95% | Position within the account lifecycle |
| `rapid_fire_transaction_flag` | 1.88% | High-velocity sequence identification |

For a detailed narrative of the discovery and resolution of these artifacts, see the [Feature Leakage Case Study](feature_importance_analysis.md).

### Generalization Results
Validated against three independent populations to ensure robust performance across different random seeds.

| Test Population | Seed | Transactions | AUC |
|:--- | :--- | :--- | :--- |
| Holdout | 42 (Same) | 1.5M | 84.72% |
| Independent | 8888 (Different) | 1.5M | 79.94% |
| Independent | 5555 (Different) | 3.0M | 79.81% |

Note: The higher AUC on the holdout set is due to distributional overlap with the training population, while the ~80% AUC on independent seeds represents the model's true behavioral generalization.

---

## Section 3: Threshold Operating Points

In a production environment, the model's probability output is mapped to specific operational actions.

| Operating Mode | Threshold | Precision | Recall | F1 | Use Case |
|:--- | :--- | :--- | :--- | :--- | :--- |
| **Detection Layer** | 0.495 | 10% | 60% | 0.172 | Review queue — broad capture |
| **Triage** | 0.645 | 18% | 55% | 0.268 | Early analyst filtering |
| **Investigation** | 0.736 | 31% | 50% | 0.385 | Analyst workbench |
| **High Confidence** | 0.842 | 57% | 45% | 0.502 | Escalation decisions |
| **Blocking** | 0.945 | 73% | 40% | 0.517 | Automatic card block |

The Detection Layer feeds a review queue for manual inspection, while the Blocking Layer is reserved for automated enforcement. The tradeoff between these layers is an operational business decision, not a model failure.

---

## Section 4: Merchant Category Audit

Leakage verification at the "Blocking" threshold (0.945) confirms that overrepresentation reflects genuine category risk levels rather than static bypasses.

| Category | Global Share | Flag Share | Index | Verified Fraud Rate |
|:--- | :--- | :--- | :--- | :--- |
| **GAMBLING** | 0.07% | 1.09% | 17x | 17.68% |
| **ENTERTAINMENT** | 1.10% | 14.35% | 13x | 11.20% |
| **LUXURY** | 1.62% | 8.63% | 5x | 4.91% |
| **ELECTRONICS** | 3.39% | 10.22% | 3x | 2.40% |
| **TRAVEL** | 6.14% | 16.29% | 2.6x | 2.53% |
| **SERVICES** | 5.15% | 11.92% | 2.3x | 2.53% |

All verified fraud rates fall below the 20% threshold, confirming that no single category acts as a near-deterministic fraud rule. The model uses category as a Bayesian prior requiring behavioral confirmation rather than a static classifier.

The `GAMBLING` index was previously at 103x (documented in the leakage case study); its reduction to 17x after generator retuning and the verified fraud rate confirms it is now a legitimate signal.

---

## Section 5: Known Limitations

### Recall Ceiling (66.7%)
Theoretical maximum recall is imposed by deliberate label noise design. The 0.5% false positive rate in `fp_rate` creates labels that are behaviorally unlearnable. Recall approaching this ceiling represents optimal behavior.

### Silver ETL Eager Execution
Sequence features using `.over()` window functions trigger eager in-memory execution despite Polars lazy API usage. Datasets significantly exceeding available RAM will hit memory pressure. Roadmap: transition to a stateful streaming pre-aggregation pass.

### Campaign Detection
Coordinated attack signatures require graph-based reasoning over entity relationships. Individual transactions in a campaign are often behaviorally indistinguishable from legitimate ones when viewed in isolation—this is a structural limitation of single-transaction classifiers.
