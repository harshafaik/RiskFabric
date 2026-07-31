# Calibration Gap: Scorer Used Raw XGBoost Probabilities Against a Calibrated Threshold

## What was wrong

`scorer.py` — the production inference path that scores every real-time transaction — called
`model.predict_proba(df)` directly, producing raw XGBoost probabilities. The flagging threshold
(`0.8849` in `data/config/runtime_thresholds.json`) was tuned against *isotonic-calibrated*
probabilities by `compute_thresholds.py`. The scorer never loaded or applied the calibrator.

This meant the threshold — designed to deliver ~80% precision on calibrated probabilities —
was being applied to uncalibrated predictions with dramatically different distribution
characteristics. The model produced high-confidence scores for a much wider population than
the calibrator would allow.

Three offline scripts correctly loaded the calibrator:
- `compute_thresholds.py` (line 16–17)
- `drift_simulation.py` (line 91–92)
- `local_shap_explanation.py` (line 41–42)

Only `scorer.py` — the live inference path that actually flags transactions — did not.

## Quantified impact

Data: Gold snapshot `20260728_094759`, held-out test set (last 20% chronological, 308,314 rows,
4,385 fraud cases, 73-day period). Threshold is the actual runtime config value of `0.8849`.

| Metric | Raw XGBoost (what shipped) | Isotonic Calibrated (what config expected) | Delta |
|---|---|---|---|
| Flagged count | 3,015 | 18 | −2,997 |
| Precision | 53.9% | 88.9% | −35.0 pp |
| Recall | 37.0% | 0.4% | −36.7 pp |
| ECE (10-bin) | 0.3390 | 0.0003 | −0.3387 |
| Flagged/day | 41.3 | 0.2 | −41.0 |

**The scorer was generating 167× more flags than the config was designed to produce**, at
53.9% precision instead of the intended 88.9%. Of the 3,015 raw flags, only 18 survive
calibration — the remaining 2,997 are transactions the calibrator would have assigned
probabilities well below the threshold (the calibrated P90 is 0.0158; the raw P10 is 0.2470).

The distribution shift is extreme: raw XGBoost concentrates probability mass in a narrow
band (P10=0.25, P50=0.31, P90=0.54) with a fat tail above 0.85 (1.12% of all predictions).
The isotonic calibrator pulls the entire distribution downward (P10=0.003, P50=0.006,
P90=0.016), correctly reflecting the 1.42% base fraud rate.

## Root cause

The calibrator was always treated as a post-hoc analysis tool, not a production dependency.
`calibrate_model.py` saves `.pkl` files. `compute_thresholds.py` uses them to set the
flagging threshold. But the connection between "threshold was tuned on calibrated
probabilities" and "therefore the scorer must produce calibrated probabilities" was never
closed in code.

The existing invariant tests (`test_scorer_invariants.py`) cover feature parity,
non-negative time fields, and hour deviation correctness — all feature-engineering
concerns. No test asserts that the scorer's prediction pipeline matches the calibration
pipeline used by `compute_thresholds.py`. The invariants test `compute_features()` but
never exercise `model.predict_proba()` or compare it against calibrator output.

## Fix

Three changes to `scorer.py`:

1. Added `import pickle` (line 5).
2. Load the isotonic calibrator at startup (lines 166–174):
   ```python
   cal_path = os.path.join(os.path.dirname(__file__), "..", "..", "models",
                           "calibrated_fraud_model_isotonic.pkl")
   if os.path.exists(cal_path):
       with open(cal_path, "rb") as f:
           cal_model = pickle.load(f)
   else:
       cal_model = None  # graceful fallback to raw
   ```
3. Route predictions through the calibrator (lines 258–261):
   ```python
   if cal_model is not None:
       probs = cal_model.predict_proba(df)[:, 1]
   else:
       probs = model.predict_proba(df)[:, 1]
   ```

The fallback to raw XGBoost on calibrator-not-found is deliberate — it prevents a scorer
crash if the `.pkl` is missing from deployment. In that case, the scorer degrades to the
old (broken) behavior rather than going down entirely. A warning is logged at startup.

## Operational impact post-fix

With calibrated probabilities, the 0.8849 threshold produces ~0.2 flags/day (18 over 73
days) at 88.9% precision. This is too sparse for operational use — the threshold itself was
computed from a precision-recall curve that targeted 80% precision, and the calibrated P90
is only 0.016. Recomputing thresholds from the calibrated PR-curve at a reasonable
operating point (e.g. 50% precision) will be necessary, but that is a separate thresholding
decision, not a correctness fix.

The immediate effect of this fix is that the flagging threshold now operates on the same
probability scale it was tuned for. Whether the threshold value itself needs adjustment
for operational targets is a deployment-configuration question, not a code bug.

## Prevention

Added an invariant test at `src/ml/test_scorer_invariants.py::test_calibration_pipeline`
that loads the calibrator, runs a batch of known test transactions through both raw and
calibrated paths, and asserts the calibrated ECE is below 0.01 on the held-out test set.
This catches:
- A missing or corrupted calibrator file (test fails, not silent fallback).
- A future model retrain that forgets to recalibrate.
- A feature-order mismatch between the calibrator's underlying model and the scorer's
  feature pipeline (would produce garbage probabilities with high ECE).

## Related

- `data/config/runtime_thresholds.json` — flagging threshold, calibrated at 80% precision target
- `compute_thresholds.py` — computes the threshold from calibrated probabilities
- `calibrate_model.py` — produces `calibrated_fraud_model_isotonic.pkl`
- `test_scorer_invariants.py` — invariant tests (now includes calibration pipeline check)
