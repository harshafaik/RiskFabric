import polars as pl
import xgboost as xgb
import clickhouse_connect
import shap
import matplotlib.pyplot as plt
import os
import numpy as np

def run_shap_analysis():
    print("🚀 Connecting to ClickHouse...")
    client = clickhouse_connect.get_client(
        host='localhost', 
        port=8123,
        username='riskfabric_user',
        password='123',
        database='riskfabric'
    )

    print("📊 Loading Gold Master Table...")
    # Fetching both features and ground truth fraud_type for segmentation
    query = "SELECT * FROM fact_transactions_gold"
    df = pl.from_arrow(client.query_arrow(query))
    
    # Clean feature list (Match training)
    feature_cols = [
        # Behavioral sequence
        'time_since_last_transaction',
        'transaction_sequence_number',
        'spatial_velocity',
        'hour_deviation_from_norm',
        'amount_deviation_z_score',
        'rapid_fire_transaction_flag',
        'escalating_amounts_flag',
        'merchant_category_switch_flag',
        
        # Transaction context
        'transaction_channel',
        'card_present',
        'merchant_category',
        
        # Network structural (not label-derived)
        'suspicious_cluster_member',
    ]
    
    available_cols = df.columns
    feature_cols = [c for c in feature_cols if c in available_cols]
    
    print(f"🧠 Using features: {feature_cols}")

    # 1b. SAMPLING FOR PERFORMANCE (6M rows is too much for SHAP in one go)
    # We want to ensure we have enough samples of EACH fraud type
    sample_size = 50000
    if df.height > sample_size:
        print(f"🎲 Sampling data to {sample_size} rows for SHAP analysis...")
        # Take all fraud cases if they fit, plus some legitimate cases
        fraud_df = df.filter(pl.col('is_fraud') == 1)
        legit_df = df.filter(pl.col('is_fraud') == 0).sample(n=min(sample_size, df.height - fraud_df.height))
        df = pl.concat([fraud_df, legit_df])
        print(f"   -> Sample contains {fraud_df.height} fraud and {legit_df.height} legitimate transactions.")

    # Handle Categoricals (must match training)
    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical) for c in string_cols])

    # 2. LOAD MODEL
    print("💾 Loading Model...")
    model = xgb.XGBClassifier()
    model.load_model("models/fraud_model_v1.json")

    # 3. SHAP CALCULATION
    print("🔮 Calculating SHAP values...")
    # Using TreeExplainer for XGBoost (very fast)
    X = df.select(feature_cols).to_pandas()
    explainer = shap.TreeExplainer(model)
    shap_values = explainer.shap_values(X)

    # 4. GLOBAL SHAP SUMMARY
    print("📈 Generating Global SHAP Summary...")
    os.makedirs("reports/shap", exist_ok=True)
    
    plt.figure(figsize=(12, 8))
    shap.summary_plot(shap_values, X, show=False)
    plt.title("Global SHAP Feature Importance")
    plt.tight_layout()
    plt.savefig("reports/shap/global_summary.png")
    plt.close()

    # 5. SEGMENTED ANALYSIS BY FRAUD PROFILE
    print("🔍 Analyzing by Fraud Profile...")
    # Get indices for each fraud type (from ground truth)
    fraud_types = df['fraud_type'].unique().to_list()
    # Remove 'None' if present (legitimate transactions)
    fraud_types = [t for t in fraud_types if t != 'None' and t is not None]

    for ftype in fraud_types:
        print(f"   -> Processing profile: {ftype}")
        idx = df.with_row_index().filter(pl.col('fraud_type') == ftype)['index'].to_list()
        
        if not idx:
            continue
            
        X_sub = X.iloc[idx]
        shap_sub = shap_values[idx]
        
        plt.figure(figsize=(12, 8))
        shap.summary_plot(shap_sub, X_sub, show=False)
        plt.title(f"SHAP Explanations for: {ftype}")
        plt.tight_layout()
        plt.savefig(f"reports/shap/profile_{ftype}.png")
        plt.close()
        
        # Calculate mean absolute SHAP for this profile to print top features
        mean_shap = np.abs(shap_sub).mean(axis=0)
        feat_imp = sorted(zip(feature_cols, mean_shap), key=lambda x: x[1], reverse=True)
        print(f"      Top 3 drivers for {ftype}:")
        for feat, val in feat_imp[:3]:
            print(f"         - {feat}: {val:.4f}")

    print("\n✅ SHAP Analysis complete! Reports saved to reports/shap/")

if __name__ == "__main__":
    run_shap_analysis()
