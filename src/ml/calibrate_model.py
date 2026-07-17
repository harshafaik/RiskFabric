import polars as pl
import xgboost as xgb
import duckdb
import numpy as np
import os
import pickle
import glob
from model_utils import load_model, get_model_features
from sklearn.model_selection import train_test_split
from sklearn.calibration import CalibratedClassifierCV, calibration_curve
from sklearn.frozen import FrozenEstimator
from sklearn.metrics import (
    roc_auc_score,
    average_precision_score,
    brier_score_loss
)
from argparse import ArgumentParser


def calculate_ece(y_true, y_prob, n_bins=10):
    bin_edges = np.linspace(0, 1, n_bins + 1)
    ece = 0.0
    total_samples = len(y_prob)

    for i in range(n_bins):
        bin_lower = bin_edges[i]
        bin_upper = bin_edges[i+1]
        in_bin = (y_prob >= bin_lower) & (y_prob < bin_upper) if i < n_bins - 1 else (y_prob >= bin_lower) & (y_prob <= bin_upper)
        bin_count = np.sum(in_bin)

        if bin_count > 0:
            actual_frac = np.mean(y_true[in_bin])
            pred_mean = np.mean(y_prob[in_bin])
            bin_weight = bin_count / total_samples
            bin_error = abs(pred_mean - actual_frac)
            ece += bin_weight * bin_error

    return ece


def find_gold_snapshot(snapshot=None):
    if snapshot:
        path = f"data/gold/{snapshot}/fact_transactions_gold.parquet"
        if os.path.exists(path):
            return path
        raise FileNotFoundError(f"Snapshot not found: {path}")

    snapshots = sorted(glob.glob("data/gold/*/fact_transactions_gold.parquet"), reverse=True)
    if not snapshots:
        raise FileNotFoundError("No Gold snapshots found.")
    return snapshots[0]


def calibrate_and_evaluate():
    parser = ArgumentParser(description="Calibrate XGBoost model probabilities using Gold Parquet snapshots")
    parser.add_argument("--snapshot", type=str, default=None, help="Specific snapshot directory")
    args = parser.parse_args()

    gold_path = find_gold_snapshot(args.snapshot)
    print(f"📊 Loading Gold snapshot: {gold_path}")

    conn = duckdb.connect()
    query = f"SELECT * FROM '{gold_path}'"
    df = pl.from_arrow(conn.execute(query).arrow())
    conn.close()

    target_col = "is_fraud"
    feature_cols = get_model_features()

    feature_cols = [c for c in feature_cols if c in df.columns]
    missing = set(feature_cols) - set(df.columns)
    if missing:
        print(f"   ⚠️ Model expects features not in Gold: {missing}")

    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical).to_physical().alias(c) for c in string_cols])

    X = df.select(feature_cols).to_pandas()
    y = df.select(target_col).to_numpy().flatten()

    print("🧠 Loading Pre-trained Model...")
    base_model = load_model()

    print("✂️ Splitting data into Calibration and Evaluation sets...")
    X_cal, X_eval, y_cal, y_eval = train_test_split(
        X, y, test_size=0.5, random_state=42, stratify=y
    )

    print(f"   -> Calibration Set Size:  {len(y_cal):,}")
    print(f"   -> Evaluation Set Size:   {len(y_eval):,}")
    print(f"   -> Legitimate / Fraud:   {np.sum(y_eval==0):,} / {np.sum(y_eval==1):,}")

    print("\n⚖️ Fitting Platt Scaling Calibrator (Sigmoid)...")
    cal_platt = CalibratedClassifierCV(estimator=FrozenEstimator(base_model), method='sigmoid')
    cal_platt.fit(X_cal, y_cal)

    print("📈 Fitting Isotonic Regression Calibrator...")
    cal_isotonic = CalibratedClassifierCV(estimator=FrozenEstimator(base_model), method='isotonic')
    cal_isotonic.fit(X_cal, y_cal)

    print("\n🔮 Evaluating raw and calibrated models on held-out test data...")
    y_prob_raw = base_model.predict_proba(X_eval)[:, 1]
    y_prob_platt = cal_platt.predict_proba(X_eval)[:, 1]
    y_prob_isotonic = cal_isotonic.predict_proba(X_eval)[:, 1]

    results = {}
    models_to_eval = {
        'Raw Model (Uncalibrated)': y_prob_raw,
        'Platt Scaling (Sigmoid)': y_prob_platt,
        'Isotonic Regression': y_prob_isotonic
    }

    for name, probs in models_to_eval.items():
        results[name] = {
            'ROC-AUC': roc_auc_score(y_eval, probs),
            'PR-AUC': average_precision_score(y_eval, probs),
            'Brier Loss': brier_score_loss(y_eval, probs),
            'ECE': calculate_ece(y_eval, probs, n_bins=10),
        }

    print("\n" + "="*80)
    print("       POST-TRAINING CALIBRATION METRICS COMPARISON (EVALUATION SET)")
    print("="*80)
    print(f"{'Calibration Strategy':<30} | {'ROC-AUC':<10} | {'PR-AUC':<10} | {'Brier Loss':<12} | {'ECE'}")
    print("-" * 80)
    for name, metrics in results.items():
        print(f"{name:<30} | {metrics['ROC-AUC']:<10.5f} | {metrics['PR-AUC']:<10.5f} | {metrics['Brier Loss']:<12.6f} | {metrics['ECE']:.5f}")
    print("="*85)

    os.makedirs("models", exist_ok=True)

    with open("models/calibrated_fraud_model_platt.pkl", "wb") as f:
        pickle.dump(cal_platt, f)

    with open("models/calibrated_fraud_model_isotonic.pkl", "wb") as f:
        pickle.dump(cal_isotonic, f)

    print("\n💾 Calibrated models saved to models/")


if __name__ == "__main__":
    calibrate_and_evaluate()
