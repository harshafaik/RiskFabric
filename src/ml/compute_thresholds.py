import polars as pl
import xgboost as xgb
import numpy as np
import os
import pickle
from ml_utils import load_gold_dataframe


def compute_threshold_layers():
    print("📊 Loading Gold snapshot...")
    df = load_gold_dataframe()

    target_col = "is_fraud"
    # Load the calibrated model first to know which features it expects
    print("🧠 Loading Isotonic Calibrated Model...")
    with open("models/calibrated_fraud_model_isotonic.pkl", "rb") as f:
        cal_model = pickle.load(f)

    base_estimator = cal_model.calibrated_classifiers_[0].estimator
    feature_cols = list(base_estimator.get_booster().feature_names)
    feature_cols = [c for c in feature_cols if c in df.columns]

    # Handle Categoricals
    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical).to_physical().alias(c) for c in string_cols])

    X = df.select(feature_cols)
    y = df.select(target_col).to_numpy().flatten()

    print("🔮 Generating Calibrated Probabilities...")
    y_prob = cal_model.predict_proba(X)[:, 1]
    
    total_txns = len(y)
    total_fraud = y.sum()
    
    print(f"\n📊 Total Dataset: {total_txns:,} Transactions")
    print(f"🎯 Total Fraud:   {total_fraud:,} Cases ({total_fraud/total_txns:.4%})")

    # Define operational threshold layers
    layers = [
        {
            "name": "🔴 AUTO-BLOCKING",
            "desc": "Immediate transaction decline, low latency required.",
            "min_prob": 0.90,
            "max_prob": 1.01
        },
        {
            "name": "🟡 MANUAL INVESTIGATION",
            "desc": "Route to analyst review queue, hold/release workflow.",
            "min_prob": 0.30,
            "max_prob": 0.90
        },
        {
            "name": "🟢 PASSIVE DETECTION",
            "desc": "Alert logging, batch investigation, retrospective profile updates.",
            "min_prob": 0.05,
            "max_prob": 0.30
        }
    ]

    # Operational metrics calculation
    print("\n" + "="*95)
    print("📈 RISK TIER / THRESHOLD MAPPING (ISOTONIC-CALIBRATED)")
    print("="*95)
    print(f"{'Operational Tier':<22} | {'Calibrated Prob':<16} | {'Precision':<10} | {'Recall':<9} | {'Txn Vol %':<10} | {'Daily Queue Size (per 100k txns)'}")
    print("-" * 105)

    cumulative_recall = 0.0
    cumulative_vol = 0.0
    
    for l in layers:
        min_p = l["min_prob"]
        max_p = l["max_prob"]
        
        # Select transactions within probability boundaries
        in_layer = (y_prob >= min_p) & (y_prob < max_p)
        layer_count = np.sum(in_layer)
        layer_fraud = np.sum(y[in_layer])
        
        precision = layer_fraud / layer_count if layer_count > 0 else 0.0
        recall = layer_fraud / total_fraud if total_fraud > 0 else 0.0
        vol_pct = layer_count / total_txns
        
        expected_size_per_100k = vol_pct * 100000
        
        cumulative_recall += recall
        cumulative_vol += vol_pct
        
        prob_range = f"[{min_p:.2f}, {max_p if max_p <= 1.0 else 1.0:.2f})"
        print(f"{l['name']:<22} | {prob_range:<16} | {precision:<10.2%} | {recall:<9.2%} | {vol_pct:<10.3%} | {expected_size_per_100k:<10.1f}")
        
    print("="*105)
    
    # Cumulative Operational Analysis
    print("\n" + "="*70)
    print("🔄 CUMULATIVE OPERATIONAL PROTECTION STACK")
    print("="*70)
    
    # Cumulative Layer 1
    l1_mask = (y_prob >= 0.90)
    l1_cnt = np.sum(l1_mask)
    l1_fraud = np.sum(y[l1_mask])
    print(f"🔴 Stack 1: Auto-Blocking (>= 0.90)")
    print(f"   -> Catches {l1_fraud/total_fraud:.2%} of total fraud.")
    print(f"   -> Auto-declines {l1_cnt/total_txns:.3%} of all transaction traffic.")
    print(f"   -> Customer False Positive Rate (FPR): {(l1_cnt - l1_fraud)/total_txns:.3%}")
    print(f"   -> Precision: {l1_fraud/l1_cnt:.2%}")
    
    # Cumulative Layer 1 + 2
    l12_mask = (y_prob >= 0.30)
    l12_cnt = np.sum(l12_mask)
    l12_fraud = np.sum(y[l12_mask])
    print(f"\n🟡 Stack 2: Auto-Blocking + Manual Investigation (>= 0.30)")
    print(f"   -> Catches {l12_fraud/total_fraud:.2%} of total fraud.")
    print(f"   -> Flags {l12_cnt/total_txns:.3%} of total transaction traffic.")
    print(f"   -> Analysts must review {l12_cnt * (100000/total_txns):.1f} reviews per 100,000 txns.")
    print(f"   -> Overall Stack Precision: {l12_fraud/l12_cnt:.2%}")

    # Cumulative Layer 1 + 2 + 3
    l123_mask = (y_prob >= 0.05)
    l123_cnt = np.sum(l123_mask)
    l123_fraud = np.sum(y[l123_mask])
    print(f"\n🟢 Stack 3: Full Stack - Auto-Block + Manual + Passive Alert (>= 0.05)")
    print(f"   -> Total Fraud Coverage (Full Recall): {l123_fraud/total_fraud:.2%}")
    print(f"   -> Total System Alerts: {l123_cnt/total_txns:.3%} of total traffic.")
    print(f"   -> Overall System Precision: {l123_fraud/l123_cnt:.2%}")
    print("="*70)

if __name__ == "__main__":
    compute_threshold_layers()
