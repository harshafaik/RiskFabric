# Model Calibration Pipeline

## Overview

`calibrate_model.py` corrects the probability estimates of the trained XGBoost classifier. Tree-based models like XGBoost produce reliable rankings but notoriously miscalibrated probabilities — scores tend to cluster near 0 and 1, making a 0.85 threshold carry far less confidence than it claims. In fraud detection, where operational decisions (auto-block, manual review, passive alert) are gated on probability thresholds, miscalibration directly translates to misallocated analyst time and incorrectly declined transactions.

The calibration pipeline reads the frozen base model from `models/` using `model_utils.load_model()`, fits two calibrators on an independent 50% calibration split, evaluates both against a held-out 50% evaluation split, and saves the resulting `CalibratedClassifierCV` wrappers as pickle files. The isotonic variant is the primary artifact consumed by all downstream threshold and analysis scripts.

## Calibration Methods

| Method | Class | Behavior |
| :--- | :--- | :--- |
| **Platt Scaling (Sigmoid)** | `CalibratedClassifierCV(method='sigmoid')` | Fits a logistic regression on the raw model's logit outputs. Produces a smooth sigmoid mapping that compresses extreme predictions toward the center. Works best when miscalibration is symmetric. |
| **Isotonic Regression** | `CalibratedClassifierCV(method='isotonic')` | Fits a non-parametric, monotonically increasing step function. More flexible than Platt — corrects arbitrary distortions in the probability distribution. The preferred method for fraud models where overconfidence patterns can be asymmetric. |

Both calibrators use `FrozenEstimator` to prevent the base XGBoost model from being retrained during calibration — only the probability mapping is learned.

## Pipeline

1. **Load gold master** from DuckDB/Parquet snapshot, deriving features from the model via `get_model_features()`.
2. **Split 50/50** into calibration and evaluation sets with `stratify` on `is_fraud`.
3. **Load the frozen base model** using `model_utils.load_model()`.
4. **Fit Platt and Isotonic calibrators** on the calibration split.
5. **Evaluate** on the held-out evaluation split, comparing raw vs. Platt vs. Isotonic across four metrics.
6. **Save** both calibrators to `models/`.

## Evaluation Metrics

| Metric | Measures | Target |
| :--- | :--- | :--- |
| **ROC-AUC** | Ranking quality — how well the model separates fraud from legitimate. | Higher is better. |
| **PR-AUC** | Precision-recall trade-off — more sensitive than ROC-AUC for imbalanced datasets. | Higher is better. |
| **Brier Loss** | Mean squared error between predicted probability and actual outcome. | Lower is better. |
| **ECE** (Expected Calibration Error) | Weighted average of the gap between predicted confidence and observed frequency across 10 probability bins. A model saying "85% confident" should see roughly 85% actual fraud in that bin. | Lower is better. Calibration's primary success metric. |

The script also prints a per-bin breakdown showing predicted probability vs. actual fraud rate for each 0.1-width bin, making miscalibration patterns visually diagnosable.

## Outputs

| File | Format | Description |
| :--- | :--- | :--- |
| `models/calibrated_fraud_model_platt.pkl` | Pickle | `CalibratedClassifierCV` wrapper with sigmoid method. |
| `models/calibrated_fraud_model_isotonic.pkl` | Pickle | `CalibratedClassifierCV` wrapper with isotonic method. Primary deployment artifact. |

## Downstream Consumers

- **`compute_thresholds.py`** — Loads the isotonic model to produce calibrated probability distributions for operational tier mapping (auto-block at ≥0.90, manual review at ≥0.30, passive alert at ≥0.05).
- **`drift_simulation.py`** — Measures how calibration degrades under feature drift scenarios (moderate and severe). Reports ECE drift alongside AUC/Brier degradation to determine recalibration triggers.
- **`local_shap_explanation.py`** — Reports both raw and calibrated probability for each SHAP-explained transaction, showing the calibration adjustment applied to individual predictions.

## Relationship to Training

Calibration is a distinct post-training stage, not embedded in the training pipeline. The separation is intentional: training produces a ranker (optimized for AUC), calibration makes its probabilities trustworthy (optimized for ECE). Changing the base model or retraining on new data requires re-running calibration, but hyperparameter tuning on the base model does not invalidate a calibrator — so long as the feature set is unchanged, the calibration mapping remains valid.

## Known Issues

The calibration script casts categorical features to Polars `Categorical` type without calling `.to_physical()`, while training produces integer ordinals via `.cast(pl.Categorical).to_physical()`. The model was trained with `enable_categorical=False` and expects numeric input — passing categorical-typed data will cause prediction failures. A shared encoding pipeline or a standardised `.to_physical()` call across training, calibration, and inference is required.
