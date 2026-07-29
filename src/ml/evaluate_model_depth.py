import polars as pl
import xgboost as xgb
import numpy as np
import os
from sklearn.metrics import (
    roc_auc_score,
    average_precision_score,
    brier_score_loss,
    precision_recall_curve,
    confusion_matrix
)
from sklearn.calibration import calibration_curve
from model_utils import load_model, get_model_features
from ml_utils import load_gold_dataframe, split_by_timestamp


def run_deep_evaluation():
    print("📊 Loading Gold snapshot...")
    df = load_gold_dataframe()

    target_col = "is_fraud"
    feature_cols = get_model_features()
    feature_cols = [c for c in feature_cols if c in df.columns]

    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]

    _, test_df = split_by_timestamp(df, test_size=0.2)
    test_start = test_df["timestamp"].min()
    test_end = test_df["timestamp"].max()
    print(f"   Evaluating on held-out test period: {test_start} → {test_end} ({len(test_df):,} rows)")

    print("🧠 Loading model...")
    model = load_model(enable_categorical=True)

    X = test_df.select(feature_cols).to_pandas()
    y = test_df.select(target_col).to_numpy().flatten()
    fraud_types = test_df.select("fraud_type").to_numpy().flatten()

    for c in string_cols:
        X[c] = X[c].astype("category")

    print("🔮 Generating Predictions & Probabilities...")
    y_prob = model.predict_proba(X)[:, 1]

    # --- 1. Precision-Recall AUC & ROC AUC ---
    roc_auc = roc_auc_score(y, y_prob)
    pr_auc = average_precision_score(y, y_prob)

    print("\n" + "="*50)
    print("🏆 CORE DISCRIMINATION METRICS")
    print("="*50)
    print(f"ROC-AUC:                  {roc_auc:.5f}")
    print(f"PR-AUC (Average Precision): {pr_auc:.5f}")
    print(f"Base Fraud Rate:          {(y.sum() / len(y)):.4%}")

    # --- 2. Calibration of Predicted Probabilities ---
    brier = brier_score_loss(y, y_prob)
    
    # Calculate ECE (Expected Calibration Error) manually
    prob_true, prob_pred = calibration_curve(y, y_prob, n_bins=10, strategy='uniform')
    
    # Custom ECE calculation
    bin_edges = np.linspace(0, 1, 11)
    ece = 0.0
    total_samples = len(y_prob)
    
    print("\n" + "="*50)
    print("📊 PROBABILITY CALIBRATION (10 Uniform Bins)")
    print("="*50)
    print(f"Brier Score Loss: {brier:.6f}")
    print(f"{'Bin Range':<15} {'Mean Pred':<12} {'Actual Fraction':<16} {'Count':<10} {'Error':<10}")
    print("-" * 70)
    
    for i in range(10):
        bin_lower = bin_edges[i]
        bin_upper = bin_edges[i+1]
        in_bin = (y_prob >= bin_lower) & (y_prob < bin_upper) if i < 9 else (y_prob >= bin_lower) & (y_prob <= bin_upper)
        bin_count = np.sum(in_bin)
        
        if bin_count > 0:
            actual_frac = np.mean(y[in_bin])
            pred_mean = np.mean(y_prob[in_bin])
            bin_weight = bin_count / total_samples
            bin_error = abs(pred_mean - actual_frac)
            ece += bin_weight * bin_error
            print(f"[{bin_lower:3.1f}, {bin_upper:3.1f})     {pred_mean:<12.4f} {actual_frac:<16.4f} {bin_count:<10} {bin_error:<10.4f}")
        else:
            print(f"[{bin_lower:3.1f}, {bin_upper:3.1f})     {'N/A':<12} {'N/A':<16} {0:<10} {'0.0000':<10}")
            
    print(f"\nExpected Calibration Error (ECE): {ece:.5f}")

    # --- 3. Scenario-Specific Performance & Feature Stability ---
    print("\n" + "="*50)
    print("🎯 FRAUD SCENARIO SUBGROUP ANALYSIS")
    print("="*50)
    print(f"{'Scenario':<20} {'Txn Count':<10} {'Detection Rate (Recall)':<24} {'Avg Predicted Prob':<20}")
    print("-" * 80)

    # Scenarios listed
    unique_scenarios = [s for s in np.unique(fraud_types) if s != 'none']
    
    # Store scenario recalls for final reporting
    scenario_results = {}
    
    for s in unique_scenarios:
        s_mask = (fraud_types == s)
        s_count = np.sum(s_mask)
        
        if s_count > 0:
            s_recall = np.mean(y_prob[s_mask] >= 0.5)
            s_mean_prob = np.mean(y_prob[s_mask])
            scenario_results[s] = {
                'count': int(s_count),
                'recall': float(s_recall),
                'mean_prob': float(s_mean_prob)
            }
            print(f"{s:<20} {s_count:<10} {s_recall:<24.2%} {s_mean_prob:<20.4f}")
            
    # Calculate feature stability/sensitivity by looking at mean feature values for True Positives of each scenario
    print("\n" + "="*50)
    print("🛡️ FEATURE STABILITY / SENSITIVITY CROSS-ANALYSIS")
    print("="*50)
    print("Average normalized feature values (Z-scores) across true positive detections of each fraud scenario:")
    
    # We want to scale continuous features to Z-score relative to legitimate transactions ('none')
    none_mask = (fraud_types == 'none')
    
    continuous_features = [
        "time_since_last_transaction",
        "transaction_sequence_number",
        "spatial_velocity",
        "hour_deviation_from_norm",
        "amount_deviation_z_score"
    ]
    
    # Calculate baseline means and stds
    baselines = {}
    for col in continuous_features:
        baselines[col] = {
            'mean': float(test_df.filter(none_mask)[col].mean()),
            'std': float(test_df.filter(none_mask)[col].std()) or 1.0
        }
        
    # Table headers
    print(f"\n{'Feature':<30}", end="")
    for s in unique_scenarios:
        print(f" {s[:10]:>11}", end="")
    print("\n" + "-" * (30 + 12 * len(unique_scenarios)))
    
    for col in continuous_features:
        print(f"{col:<30}", end="")
        for s in unique_scenarios:
            s_tp_mask = (fraud_types == s) & (y_prob >= 0.5)
            if np.sum(s_tp_mask) > 0:
                s_mean = test_df.filter(s_tp_mask)[col].mean()
                z_score = (s_mean - baselines[col]['mean']) / baselines[col]['std']
                print(f" {z_score:>11.2f}", end="")
            else:
                print(f" {'N/A':>11}", end="")
        print()

    # Categorical/Flag features: print positive activation rates
    flag_features = [
        "rapid_fire_transaction_flag",
        "escalating_amounts_flag",
        "merchant_category_switch_flag",
        "card_present"
    ]
    
    print("\nFlag Activation Rates (True Positives vs Normal Base):")
    print(f"\n{'Feature':<30} {'Normal Base':>11}", end="")
    for s in unique_scenarios:
        print(f" {s[:10]:>11}", end="")
    print("\n" + "-" * (42 + 12 * len(unique_scenarios)))
    
    for col in flag_features:
        base_rate = test_df.filter(none_mask)[col].mean()
        print(f"{col:<30} {base_rate:>11.2%}", end="")
        for s in unique_scenarios:
            s_tp_mask = (fraud_types == s) & (y_prob >= 0.5)
            if np.sum(s_tp_mask) > 0:
                s_rate = test_df.filter(s_tp_mask)[col].mean()
                print(f" {s_rate:>11.2%}", end="")
            else:
                print(f" {'N/A':>11}", end="")
        print()

if __name__ == "__main__":
    run_deep_evaluation()
