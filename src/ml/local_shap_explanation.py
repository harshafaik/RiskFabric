import polars as pl
import xgboost as xgb
import numpy as np
import shap
import os
import pickle
from model_utils import load_model
from ml_utils import load_gold_dataframe


def run_local_shap_explanations():
    print("📊 Loading Gold snapshot...")
    df = load_gold_dataframe()

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

    # Filter out missing columns
    feature_cols = [c for c in feature_cols if c in df.columns]

    # Handle Categoricals
    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical).to_physical().alias(c) for c in string_cols])

    print("🧠 Loading Models (Pre-trained XGBoost + Isotonic Calibrator)...")
    model = load_model()

    with open("models/calibrated_fraud_model_isotonic.pkl", "rb") as f:
        cal_model = pickle.load(f)

    # Initialize SHAP TreeExplainer
    print("🔮 Initializing SHAP TreeExplainer...")
    explainer = shap.TreeExplainer(model)
    base_value = float(explainer.expected_value[0])
    
    # Sigmoid function for probability conversion
    def sigmoid(x):
        return 1 / (1 + np.exp(-x))

    # We want to select 1 clear True Positive transaction for each of the 5 fraud profiles
    unique_scenarios = ["upi_scam", "card_not_present", "account_takeover", "velocity_abuse", "friendly_fraud"]
    
    print("\n" + "="*90)
    print("🔬 INSTANCE-LEVEL EXPLANATIONS (LOCAL SHAP FORCE-PLOTS)")
    print("="*90)
    
    # We will loop through each scenario and find one representative txn
    for s in unique_scenarios:
        # Find transactions belonging to this scenario with is_fraud=1
        s_df = df.filter((pl.col("fraud_type") == s) & (pl.col("is_fraud") == 1))
        
        if s_df.height == 0:
            print(f"\n⚠️ No cases found for profile: {s}")
            continue
            
        # Let's pick the first transaction in the slice
        txn = s_df.row(0, named=True)
        txn_id = txn["transaction_id"]
        
        # Get feature values for this specific instance
        txn_features_df = s_df.slice(0, 1).select(feature_cols)
        X_pd = txn_features_df.to_pandas()
        
        # Run XGBoost margin prediction and Calibrated prediction
        raw_prob = float(model.predict_proba(X_pd)[0, 1])
        cal_prob = float(cal_model.predict_proba(X_pd)[0, 1])
        
        # Compute SHAP values for this instance
        shap_vals = explainer.shap_values(X_pd)[0]
        
        # Check alignment: base_value + sum(shap_vals) should equal the logit
        logit = float(base_value + np.sum(shap_vals))
        reconstructed_prob = float(sigmoid(logit))
        
        print(f"\n🎯 [PROFILE: {s.upper()}] | Txn ID: {txn_id}")
        print(f"   Details: Amount: ₹{txn['amount']:.2f} | Channel: {txn['transaction_channel']} | Category: {txn['merchant_category']}")
        print(f"   Raw Model Prediction (Logit / Prob): {logit:.4f} / {raw_prob:.2%}")
        print(f"   Calibrated Probability (Empirical):  {cal_prob:.2%}")
        print("-" * 75)
        
        # Sort features by absolute SHAP impact
        feature_impacts = []
        for col, val, shap_val in zip(feature_cols, X_pd.iloc[0].values, shap_vals):
            feature_impacts.append({
                'feature': col,
                'value': val,
                'shap_val': shap_val
            })
            
        # Separate positive (pushing towards fraud) and negative (pulling towards legit) drivers
        pos_drivers = sorted([f for f in feature_impacts if f['shap_val'] > 0.01], key=lambda x: x['shap_val'], reverse=True)
        neg_drivers = sorted([f for f in feature_impacts if f['shap_val'] < -0.01], key=lambda x: x['shap_val'])
        
        # 1. Print Positive Drivers (Pushing score UP)
        print("   🔴 Features driving score UP (Fraud Risk Drivers):")
        for f in pos_drivers:
            # Render a mini ASCII bar chart
            bar_len = int(min(20, max(1, f['shap_val'] * 4)))
            bar = "█" * bar_len
            print(f"      + {f['feature']:<30} = {str(f['value']):<15} | SHAP: +{f['shap_val']:+.3f} {bar}")
            
        # 2. Print Negative Drivers (Pulling score DOWN)
        if neg_drivers:
            print("\n   🔵 Features pulling score DOWN (Mitigating Drivers):")
            for f in neg_drivers:
                bar_len = int(min(20, max(1, abs(f['shap_val']) * 4)))
                bar = "█" * bar_len
                print(f"      - {f['feature']:<30} = {str(f['value']):<15} | SHAP: {f['shap_val']:+.3f} {bar}")
        else:
            print("\n   🔵 Mitigating Drivers: None (No features decreased the fraud log-odds).")
            
        print("=" * 80)

if __name__ == "__main__":
    run_local_shap_explanations()
