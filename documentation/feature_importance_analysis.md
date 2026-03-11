# Feature Importance & Leakage Analysis

This document details the analysis of model feature importance and the discovery of the "Amount Shortcut," which masked weak behavioral signals in the initial training iterations.

## 🚨 The "Amount" Shortcut

In early training runs, the XGBoost model achieved a high AUC (~0.91) but showed a dangerous reliance on a single feature: `amount`.

### Feature Importance Distribution (Initial)
| Feature | Importance |
| :--- | :--- |
| **amount** | **96.32%** |
| escalating_amounts_flag | 0.93% |
| cf_night_tx_ratio | 0.49% |
| others | < 2% combined |

### Root Cause: Distribution Separability
A statistical audit of the `fact_transactions_gold` table revealed that the fraud injector was producing amounts that were too easily separable from legitimate traffic, creating a "cheat code" for the model.

| Metric | Legitimate (0) | Fraudulent (1) |
| :--- | :--- | :--- |
| **Average Amount** | ₹588.63 | ₹42,817.64 |
| **Median Amount** | ₹380.13 | ₹625.36 |
| **75th Percentile** | ₹640.51 | ₹15,000.00 |
| **95th Percentile** | ₹1,204.18 | ₹150,000.00 |

**Discovery**: While the medians are relatively close, the upper tail of the fraud distribution is orders of magnitude higher than legitimate spend. The model learned that "High Value = Fraud" rather than learning behavioral patterns like velocity or location anomalies.

---

## 🧪 Behavioral Signal Strength

To test the strength of actual behavioral features, we performed a "Stress Test" by binning and then removing the amount feature entirely.

### Experiment 1: Amount Binning
- **Approach**: Binned raw `amount` into `[micro, low, medium, high, very_high]`.
- **Result**: AUC 0.87. `amount_bin` importance remained at **96.6%**. 
- **Conclusion**: The distribution gap is so large that even coarse binning provides a near-perfect split.

### Experiment 2: Amount Removal (The "Honest" Baseline)
- **Approach**: Removed all references to transaction value. Forced model to rely on `time_since_last_tx`, `rapid_fire_flag`, `merchant_category`, etc.
- **Result**: **AUC 0.5868** (Slightly better than random).
- **New Top Feature**: `escalating_amounts_flag` (**88.6%**).

**Insight**: Without the "Amount Shortcut," the model struggles to identify fraud. This indicates that while the generator injects behavioral anomalies (ATO, UPI scams), the **signal-to-noise ratio** for these features is currently too low for the model to generalize effectively.

---

## 🛠️ Remediation Strategy

To build a high-fidelity fraud detection model that generalizes to real-world behavioral patterns, the following steps are required:

1.  **Generator Amount Jittering**: Re-tune the `FraudRules` to ensure fraudulent transaction amounts overlap significantly with the 75th-95th percentiles of legitimate users.
2.  **Signal Amplification**: Increase the "sharpness" of behavioral anomalies (e.g., making `rapid_fire` sequences more distinct or `geo_anomalies` more frequent in certain profiles).
3.  **New Feature Engineering**:
    *   **H3 Spatial Velocity**: Distance traveled between subsequent transactions.
    *   **Merchant Category Entropy**: Measuring the "randomness" of categories visited in a short window.
    *   **Device/IP Reputation**: Strengthening the weight of shared entity signals in the Silver layer.
