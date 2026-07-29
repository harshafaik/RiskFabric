# Model Explainability (SHAP)

## Overview

Two SHAP scripts provide complementary views into the model's decision-making: `shap_analysis.py` answers "what matters overall?" with global and per-fraud-profile feature importance plots, while `local_shap_explanation.py` answers "why was *this* transaction flagged?" with instance-level waterfall explanations. Together they form a complete explainability story — global patterns validated by local verification.

Both scripts load the trained XGBoost model via `model_utils.load_model()` and use SHAP's `TreeExplainer` for exact, fast shap value computation without sampling approximations.

## Global SHAP Analysis (`shap_analysis.py`)

### What It Does

Samples 50,000 transactions from the gold master (keeping all fraud cases to preserve rare profiles), computes Tree SHAP values, and generates plots stratified by the five fraud profiles: `upi_scam`, `card_not_present`, `account_takeover`, `velocity_abuse`, and `friendly_fraud`.

### Outputs

| File | Content |
| :--- | :--- |
| `reports/shap/global_summary.png` | Beeswarm plot showing all features ranked by mean absolute SHAP value across the full sample. |
| `reports/shap/profile_{fraud_type}.png` | Per-profile beeswarm plots showing which features drive predictions for each fraud signature. The top 3 drivers per profile are also printed to stdout. |

### Why Per-Profile Breakdowns Matter

Different fraud signatures exploit different behavioral patterns. `velocity_abuse` should show high SHAP impact on `spatial_velocity` and `rapid_fire_transaction_flag`. `upi_scam` should be driven by `transaction_channel` and `amount_deviation_z_score`. If the wrong features dominate a profile, it signals either a training data imbalance or a feature engineering gap — the model is catching the fraud through an unintended signal.

## Local SHAP Explanations (`local_shap_explanation.py`)

### What It Does

Selects one representative true-positive transaction per fraud profile and computes per-feature SHAP contributions. For each transaction it prints:

- Raw model logit and probability vs. calibrated probability (showing the calibration adjustment)
- Top features pushing the score **up** (fraud risk drivers) with ASCII bar charts
- Top features pulling the score **down** (mitigating drivers)

The SHAP values are validated by reconstructing the logit: `base_value + sum(shap_values)` must equal the model's raw margin output, converted to probability via sigmoid.

### Why Local Explanations Matter

Global feature importance tells you `spatial_velocity` is important. Local SHAP tells you *this specific transaction* was flagged because the card traveled 800 km in 12 minutes while switching merchant categories — a narrative a fraud analyst can act on. This bridges the gap between model development and operational use, where explainability is required for regulatory compliance and analyst trust.

## Consumed Artifacts

| Script | Consumes |
| :--- | :--- |
| `shap_analysis.py` | Latest model JSON (via `model_utils`), Gold Parquet snapshot |
| `local_shap_explanation.py` | Latest model JSON (via `model_utils`), `models/calibrated_fraud_model_isotonic.pkl`, Gold Parquet snapshot |

## Current Limitations

`local_shap_explanation.py` silently skips fraud profiles with zero true positives in the gold master (warning only, no error). It also calls `load_model()` without `enable_categorical=False`, which may use a different default than the training pipeline.
