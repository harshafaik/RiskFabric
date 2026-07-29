# Machine Learning Metrics

Operational metrics for the current production model (v4.1, seed=42, chronological split). For the full leakage narrative and how these numbers were arrived at, see the [Feature Leakage Case Study](feature_leakage_issues.md). For reproducible benchmarks (hardware, N=5, percentiles), see [Performance Benchmarks](performance.md).

## Current Production Model (v4.1)

| Parameter | Value |
|---|---|
| Snapshot | `20260716_145639` |
| Train rows | 1,079,098 (first 70% chronological) |
| Calibration rows | 154,157 (middle 10% chronological) |
| Test rows | 308,314 (last 20% chronological — held out) |
| Features | 10 behavioral |
| Fraud rate | 1.41% |

### Discrimination (Held-Out Chronological Test Set)

| Metric | Uncalibrated | Platt | Isotonic |
|---|---|---|---|
| ROC-AUC | 0.7622 | 0.7622 | 0.7622 |
| PR-AUC | 0.3293 | 0.3293 | 0.3292 |
| ECE | 0.3516 | 0.0036 | 0.0003 |

The isotonic ECE of 0.0003 was measured on the completely held-out test set (disjoint from both training and calibration sets).

### Feature Importance

| Feature | Gain |
|---|---|
| `spatial_velocity` | 34.3% |
| `transaction_channel` | 18.0% |
| `amount_deviation_z_score` | 17.3% |
| `time_since_last_transaction` | 12.0% |
| `merchant_category_switch_flag` | 4.5% |
| `card_present` | 4.0% |
| `hour_deviation_from_norm` | 3.2% |
| `rapid_fire_transaction_flag` | 2.5% |
| `transaction_sequence_number` | 2.4% |
| `escalating_amounts_flag` | 1.8% |

`spatial_velocity` is the dominant behavioral signal. `transaction_channel` and `amount_deviation_z_score` follow as strong secondary features. The distribution is stable across random vs chronological splits — the leakage affects the metrics, not which features the model learns from.

### AUC Decline Traced Through Leakage Fixes

| Fix | AUC | Cause of inflation |
|---|---|---|
| Target encoding leakage removed | 0.798 | `merchant_category` label-injection removed |
| Join ordering restored | 0.798 → 0.786 | `spatial_velocity` now computed on correct prior |
| Chronological split (current) | 0.786 → 0.7622 | No future data in training |

The 2.3-point gap (0.7622 vs 0.7855 random-split) is the optimistic bias from future data leakage. For a reproducible ablation study quantifying each leak class individually at seed=42, see [Performance Benchmarks](performance.md).

## Threshold Operating Points (Isotonic Calibrated, Chronological Test Set)

Thresholds are written to `data/config/runtime_thresholds.json` by `compute_thresholds.py` and loaded at startup by `scorer.py`.

| Operating Mode | Threshold | Precision | Recall | Alerts/100K/day | Fraud caught |
|---|---|---|---|---|---|
| Auto-blocking | 0.917 | 88.9% | 0.4% | 5.8 | 16 / 4,385 |
| Manual investigation | 0.885 | 80.0% | 1.5% | 25.9 | 64 / 4,385 |
| High-recall detection | 0.128 | 49.5% | 38.2% | 1,096 | 1,674 / 4,385 |

At the precision-first threshold of 0.885, 4 of 5 flagged transactions are genuine fraud. The alternative 0.128 threshold catches 38× more fraud at 42× the analyst volume — a legitimate operational tradeoff.

> Earlier threshold tables in project history reported 97.5% precision and 40−60% recall. Those were derived from a random-split model with uncalibrated probabilities and are not representative of deployed performance.

## Merchant Category Audit

Leakage verification at the blocking threshold (0.945) confirms that overrepresentation reflects genuine category risk rather than static bypasses. All verified fraud rates fall below 20%, ruling out any single category as a near-deterministic fraud rule. The model uses category as a Bayesian prior requiring behavioral confirmation.

| Category | Global Share | Flag Share | Index | Verified Fraud Rate |
|---|---|---|---|---|
| GAMBLING | 0.07% | 1.09% | 17× | 17.68% |
| ENTERTAINMENT | 1.10% | 14.35% | 13× | 11.20% |
| LUXURY | 1.62% | 8.63% | 5× | 4.91% |
| ELECTRONICS | 3.39% | 10.22% | 3× | 2.40% |
| TRAVEL | 6.14% | 16.29% | 2.6× | 2.53% |
| SERVICES | 5.15% | 11.92% | 2.3× | 2.53% |

The GAMBLING index was previously at 103× (see [Feature Leakage Case Study](feature_leakage_issues.md)); its reduction to 17× after generator retuning confirms it is now a legitimate signal.

## Hyperparameters

Loaded from `data/config/ml_tuning.yaml` at training time.

```yaml
n_estimators: 100
max_depth: 6
learning_rate: 0.1
objective: "binary:logistic"
tree_method: "hist"
enable_categorical: true
eval_metric: "aucpr"
```

## Current Limitations

**Recall ceiling.** Theoretical maximum recall is imposed by deliberate label noise design — a 0.5% false positive rate in `fp_rate` creates labels that are behaviorally unlearnable. Recall approaching this ceiling represents optimal behavior.

**Silver ETL eager execution.** Sequence features using `.over()` window functions trigger eager in-memory execution despite Polars lazy API usage. Datasets significantly exceeding available RAM will hit memory pressure.

**Campaign detection.** Coordinated attack signatures require graph-based reasoning over entity relationships. Individual transactions in a campaign are often behaviorally indistinguishable from legitimate ones when viewed in isolation — a structural limitation of single-transaction classifiers.

**Generalization across seeds.** The model's AUC drops to ~80% on independent seed populations (the 0.7622 on the holdout set benefits from distributional overlap with the training seed). Cross-seed generalization is tracked in [Performance Benchmarks](performance.md).

## Historical Note

Versions v1−v3 of this page reported AUCs in the 0.91−0.99 range, feature importance including `merchant_category` at 11−13%, and a threshold table reaching 73% precision at 0.945. All three were products of random-split leakage and an incomplete feature set. Those numbers are documented — with corrections — in the [Feature Leakage Case Study](feature_leakage_issues.md) and are not repeated here to prevent accidental citation.
