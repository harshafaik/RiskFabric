import polars as pl
import xgboost as xgb
import duckdb
import numpy as np
import os
import pickle
import glob
import argparse
from sklearn.model_selection import train_test_split
from sklearn.metrics import (
    roc_auc_score,
    average_precision_score,
    brier_score_loss
)

RNG_SEED = 42


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


def run_drift_simulation():
    parser = argparse.ArgumentParser(description="Simulate concept drift across fraud evasion scenarios")
    parser.add_argument("--snapshot", type=str, default=None, help="Specific snapshot directory")
    parser.add_argument("--seed", type=int, default=RNG_SEED, help="Random seed for reproducibility")
    args = parser.parse_args()

    rng = np.random.default_rng(args.seed)

    gold_path = find_gold_snapshot(args.snapshot)
    print(f"📊 Loading Gold snapshot: {gold_path}")

    conn = duckdb.connect()
    query = f"SELECT * FROM '{gold_path}'"
    df = pl.from_arrow(conn.execute(query).arrow())
    conn.close()

    target_col = "is_fraud"
    feature_cols = [
        "time_since_last_transaction",
        "transaction_sequence_number",
        "spatial_velocity",
        "hour_deviation_from_norm",
        "amount_deviation_z_score",
        "rapid_fire_transaction_flag",
        "escalating_amounts_flag",
        "merchant_category_switch_flag",
        "transaction_channel",
        "card_present",
        "merchant_category",
    ]

    feature_cols = [c for c in feature_cols if c in df.columns]

    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical).to_physical().alias(c) for c in string_cols])

    X = df.select(feature_cols)
    y = df.select(target_col).to_numpy().flatten()

    print("🧠 Loading Isotonic Calibrated Model...")
    with open("models/calibrated_fraud_model_isotonic.pkl", "rb") as f:
        cal_model = pickle.load(f)

    _, X_eval, _, y_eval = train_test_split(
        X, y, test_size=0.5, random_state=args.seed, stratify=y
    )

    fraud_mask = (y_eval == 1)

    # --- MODERATE DRIFT ---
    X_mod_drift = X_eval.clone().to_pandas()
    X_mod_drift.loc[fraud_mask, "spatial_velocity"] *= 0.6
    X_mod_drift.loc[fraud_mask, "amount_deviation_z_score"] += 0.5

    rf_flags = X_mod_drift.loc[fraud_mask, "rapid_fire_transaction_flag"].values.copy()
    flip_indices = rng.choice(len(rf_flags), int(len(rf_flags) * 0.25), replace=False)
    rf_flags[flip_indices] = 0
    X_mod_drift.loc[fraud_mask, "rapid_fire_transaction_flag"] = rf_flags

    # --- SEVERE DRIFT ---
    X_sev_drift = X_eval.clone().to_pandas()
    X_sev_drift.loc[fraud_mask, "spatial_velocity"] *= 0.15
    X_sev_drift.loc[fraud_mask, "amount_deviation_z_score"] -= 1.5

    rf_flags_sev = X_sev_drift.loc[fraud_mask, "rapid_fire_transaction_flag"].values.copy()
    flip_indices_sev = rng.choice(len(rf_flags_sev), int(len(rf_flags_sev) * 0.70), replace=False)
    rf_flags_sev[flip_indices_sev] = 0
    X_sev_drift.loc[fraud_mask, "rapid_fire_transaction_flag"] = rf_flags_sev

    datasets = {
        "Baseline (No Drift)": (X_eval.to_pandas(), y_eval),
        "Moderate Drift": (X_mod_drift, y_eval),
        "Severe Drift": (X_sev_drift, y_eval)
    }

    results = {}
    print("\nRunning predictions across drift levels...")
    for name, (X_data, y_data) in datasets.items():
        probs = cal_model.predict_proba(X_data)[:, 1]
        results[name] = {
            'ROC-AUC': roc_auc_score(y_data, probs),
            'PR-AUC': average_precision_score(y_data, probs),
            'Brier': brier_score_loss(y_data, probs),
            'ECE': calculate_ece(y_data, probs, n_bins=10),
            'probs': probs
        }

    print("\n" + "="*90)
    print("       DRIFT SIMULATION REPORT (ISOTONIC MODEL DEGRADATION ANALYSIS)")
    print("="*90)
    print(f"{'Drift Scenario':<25} | {'ROC-AUC':<10} | {'PR-AUC':<10} | {'Brier Loss':<12} | {'ECE':<10}")
    print("-" * 90)

    statuses = {
        "Baseline (No Drift)": "Perfect Calibration",
        "Moderate Drift": "Graceful Degradation",
        "Severe Drift": "Catastrophic Failure"
    }

    for name, metrics in results.items():
        print(f"{name:<25} | {metrics['ROC-AUC']:<10.5f} | {metrics['PR-AUC']:<10.5f} | {metrics['Brier']:<12.6f} | {metrics['ECE']:<10.5f}")

    print("="*90)

    # Bin comparison
    print("\nEXPECTED vs ACTUAL PROBABILITIES BY BIN ACROSS DRIFT LEVELS")
    print("-" * 110)
    bin_edges = np.linspace(0, 1, 11)
    print(f"{'Bin Range':<12} | {'Baseline (Pred / Act / Err)':<28} | {'Moderate (Pred / Act / Err)':<28} | {'Severe (Pred / Act / Err)':<28}")
    print("-" * 110)

    for i in range(10):
        bin_lower = bin_edges[i]
        bin_upper = bin_edges[i+1]
        bin_range = f"[{bin_lower:.1f}, {bin_upper:.1f})"
        row_str = f"{bin_range:<12} | "

        for name in datasets.keys():
            probs = results[name]['probs']
            in_bin = (probs >= bin_lower) & (probs < bin_upper) if i < 9 else (probs >= bin_lower) & (probs <= bin_upper)
            cnt = np.sum(in_bin)
            if cnt > 0:
                mean_p = np.mean(probs[in_bin])
                act_f = np.mean(y_eval[in_bin])
                err = abs(mean_p - act_f)
                val_str = f"{mean_p:.3f} / {act_f:.3f} / {err:.3f}"
            else:
                val_str = "N/A"
            row_str += f"{val_str:<28} | "

        print(row_str.rstrip(" |"))
    print("-" * 110)

if __name__ == "__main__":
    run_drift_simulation()
