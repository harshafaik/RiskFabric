import polars as pl
import xgboost as xgb
from sklearn.metrics import classification_report, roc_auc_score, confusion_matrix
import os
from model_utils import load_model, get_model_features
from ml_utils import load_gold_dataframe, split_by_timestamp


def test_model():
    print("📊 Loading Gold snapshot...")
    df = load_gold_dataframe()
    
    target_col = 'is_fraud'
    feature_cols = get_model_features()
    feature_cols = [c for c in feature_cols if c in df.columns]

    _, test_df = split_by_timestamp(df, test_size=0.2)
    test_start = test_df["timestamp"].min()
    test_end = test_df["timestamp"].max()
    print(f"   Evaluating on held-out test period: {test_start} → {test_end} ({len(test_df):,} rows)")
    
    string_cols = [c for c in feature_cols if test_df[c].dtype == pl.String]

    print(f"🧠 Loading Model ({len(feature_cols)} features)")
    model = load_model(enable_categorical=True)

    X_test = test_df.select(feature_cols).to_pandas()
    y_test = test_df.select(target_col).to_numpy().flatten()

    for c in string_cols:
        X_test[c] = X_test[c].astype("category")

    print("🔮 Running Predictions...")
    y_prob = model.predict_proba(X_test)[:, 1]
    y_pred = (y_prob > 0.5).astype(int)

    auc = roc_auc_score(y_test, y_prob)
    print(f"\n✨ TEST ROC AUC Score: {auc:.4f}")
    
    print("\n📝 Classification Report (at 0.5 threshold):")
    print(classification_report(y_test, y_pred))

    from sklearn.metrics import precision_recall_curve
    import numpy as np
    
    precisions, recalls, thresholds = precision_recall_curve(y_test, y_prob)

    print("\n🔍 Threshold Analysis:")
    print(f"{'Threshold':>10} {'Precision':>10} {'Recall':>10} {'F1':>10}")
    print("-" * 45)

    for target_recall in [0.60, 0.55, 0.50, 0.45, 0.40]:
        idx = np.argmin(np.abs(recalls - target_recall))
        if idx < len(thresholds):
            denom = precisions[idx] + recalls[idx]
            f1 = 2 * (precisions[idx] * recalls[idx]) / denom if denom > 0 else 0
            print(f"{thresholds[idx]:>10.3f} "
                  f"{precisions[idx]:>10.2%} "
                  f"{recalls[idx]:>10.2%} "
                  f"{f1:>10.3f}")

    print("\n📉 Confusion Matrix (at 0.5):")
    print(confusion_matrix(y_test, y_pred))

    print("\n🔝 Feature Importance (from loaded model):")
    importance = model.feature_importances_
    feat_imp = sorted(zip(feature_cols, importance), key=lambda x: x[1], reverse=True)
    for feat, imp in feat_imp[:10]:
        print(f" - {feat}: {imp:.4f}")

if __name__ == "__main__":
    test_model()
