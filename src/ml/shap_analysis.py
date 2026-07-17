import polars as pl
import xgboost as xgb
import shap
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import os
import numpy as np
from model_utils import load_model, get_model_features
from ml_utils import load_gold_dataframe


def run_shap_analysis():
    print("📊 Loading Gold snapshot...")
    df = load_gold_dataframe()
    
    feature_cols = get_model_features()
    feature_cols = [c for c in feature_cols if c in df.columns]
    
    print(f"🧠 Using {len(feature_cols)} features: {feature_cols}")

    sample_size = 50000
    if df.height > sample_size:
        print(f"🎲 Sampling data to {sample_size} rows for SHAP analysis...")
        fraud_df = df.filter(pl.col('is_fraud') == 1)
        legit_sample = min(sample_size - fraud_df.height, df.height - fraud_df.height)
        legit_df = df.filter(pl.col('is_fraud') == 0).sample(n=max(legit_sample, 1))
        df = pl.concat([fraud_df, legit_df])
        print(f"   -> Sample contains {fraud_df.height} fraud and {legit_df.height} legitimate transactions.")

    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical).to_physical().alias(c) for c in string_cols])

    print("💾 Loading Model...")
    model = load_model(enable_categorical=False)

    X = df.select(feature_cols).to_pandas()
    
    print("🔮 Calculating SHAP values...")
    explainer = shap.TreeExplainer(model)
    shap_values = explainer.shap_values(X)

    os.makedirs("reports/shap", exist_ok=True)
    
    # Global summary
    plt.figure(figsize=(14, 10))
    shap.summary_plot(shap_values, X, show=False)
    plt.title("Global SHAP Feature Importance")
    plt.tight_layout()
    plt.savefig("reports/shap/global_summary.png", dpi=150)
    plt.close()

    # Bar chart
    plt.figure(figsize=(12, 8))
    shap.summary_plot(shap_values, X, plot_type="bar", show=False)
    plt.title("SHAP Feature Importance (mean |SHAP|)")
    plt.tight_layout()
    plt.savefig("reports/shap/global_bar.png", dpi=150)
    plt.close()

    print("📊 Global SHAP (mean |SHAP|):")
    mean_abs = np.abs(shap_values).mean(axis=0)
    ranked = sorted(zip(feature_cols, mean_abs), key=lambda x: x[1], reverse=True)
    for feat, val in ranked:
        print(f"  {feat:<40s} {val:.5f}")

    # Per fraud-type breakdown
    if 'fraud_type' in df.columns:
        print("\n🔍 Analyzing by Fraud Profile...")
        fraud_types = df['fraud_type'].unique().to_list()
        fraud_types = [t for t in fraud_types if t and t != 'none' and t != 'None']
        
        for ftype in fraud_types:
            idx = df.with_row_index().filter(pl.col('fraud_type') == ftype)['index'].to_list()
            if not idx:
                continue
            
            X_sub = X.iloc[idx]
            shap_sub = shap_values[idx]
            
            plt.figure(figsize=(14, 10))
            shap.summary_plot(shap_sub, X_sub, show=False)
            plt.title(f"SHAP Explanations for: {ftype}")
            plt.tight_layout()
            plt.savefig(f"reports/shap/profile_{ftype}.png", dpi=150)
            plt.close()
            
            mean_shap = np.abs(shap_sub).mean(axis=0)
            feat_imp = sorted(zip(feature_cols, mean_shap), key=lambda x: x[1], reverse=True)
            print(f"      Top 3 drivers for {ftype}:")
            for feat, val in feat_imp[:3]:
                print(f"         - {feat}: {val:.4f}")

    print("\n✅ SHAP Analysis complete! Reports saved to reports/shap/")


if __name__ == "__main__":
    run_shap_analysis()
