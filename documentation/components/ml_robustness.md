# Model Robustness & Drift

## Overview

Two scripts evaluate how the model behaves outside ideal training conditions. `drift_simulation.py` tests degradation under adversarial feature manipulation — answering "what if fraudsters get smarter?" `evaluate_model_depth.py` runs a depth-sensitivity sweep to justify the production hyperparameter choice and surfaces per-fraud-profile recall to spot blind spots.

## Drift Simulation (`drift_simulation.py`)

### What It Tests

Loads the isotonic-calibrated model and evaluates it on the held-out test split under three conditions:

| Scenario | Manipulation | What It Simulates |
| :--- | :--- | :--- |
| **Baseline (No Drift)** | Unmodified evaluation set | Production performance under ideal conditions. |
| **Moderate Drift** | `spatial_velocity` reduced 40%, `amount_deviation_z_score` shifted +0.5, 25% of `rapid_fire` flags zeroed | Fraudsters partially adapting — spoofing location better, slowing down, but still leaving some signal. |
| **Severe Drift** | `spatial_velocity` reduced 85%, `amount_deviation_z_score` shifted -1.5, 70% of `rapid_fire` flags zeroed | Highly evasive attack — micro-transactions at spoofed locations, near-perfect mimicry of legitimate behavior. |

### Metrics Tracked

Same four metrics as calibration (ROC-AUC, PR-AUC, Brier Loss, ECE) across all three scenarios, plus a per-bin predicted-vs-actual probability breakdown showing exactly where calibration breaks down under drift.

### Why It Matters

A model with 0.95 ROC-AUC on static test data is useless if a 40% feature shift drops it to 0.65. Drift simulation quantifies the model's fragility surface — how much adversarial effort is required to degrade it to ineffective levels — and identifies which features are most sensitive to manipulation.

## Depth Sensitivity Analysis (`evaluate_model_depth.py`)

### What It Does

Despite its name suggesting it trains models at multiple depths for comparison, the current implementation loads the production model and runs a comprehensive single-model evaluation covering:

- **Discrimination metrics**: ROC-AUC and PR-AUC on the full gold master
- **Calibration**: Brier score, ECE with per-bin breakdown, and calibration curve
- **Per-fraud-profile recall**: Detection rate at the 0.5 threshold for each of the five fraud profiles
- **Feature stability cross-analysis**: Z-scored feature means for true positives of each profile compared against the legitimate baseline, plus flag activation rates

### Why the Name

The script was originally designed to iterate through `max_depth` values 3–8, training and evaluating a model at each depth. The sweep loop was removed during refactoring but the filename and documentation were not updated. The script in its current form is a deep model evaluation utility, not a depth sweep.

## Consumed Artifacts

| Script | Consumes |
| :--- | :--- |
| `drift_simulation.py` | `models/calibrated_fraud_model_isotonic.pkl`, Gold Parquet snapshot via DuckDB |
| `evaluate_model_depth.py` | Latest model JSON (via `model_utils`), Gold Parquet snapshot via DuckDB |

Both scripts use `split_by_timestamp()` to evaluate on a held-out chronological test set rather than the full dataset or a random split.

## Current Limitations

`evaluate_model_depth.py` is named for a depth sweep loop that was removed during refactoring. The script should be renamed or the sweep restored.
