import polars as pl
import xgboost as xgb
import clickhouse_connect
from sklearn.metrics import classification_report, roc_auc_score, confusion_matrix
import os

def test_model():
    print("🚀 Connecting to ClickHouse...")
    client = clickhouse_connect.get_client(
        host='localhost', 
        port=8123,
        username='riskfabric_user',
        password='123',
        database='riskfabric'
    )

    print("📊 Loading Fresh Test Data (Seed 2026)...")
    query = "SELECT * FROM fact_transactions_gold"
    df = pl.from_arrow(client.query_arrow(query))
    
    target_col = 'is_fraud'
    feature_cols = [
        'time_since_last_transaction',
        'transaction_sequence_number',
        'spatial_velocity',
        'hour_deviation_from_norm',
        'amount_deviation_z_score',
        'rapid_fire_transaction_flag',
        'escalating_amounts_flag',
        'merchant_category_switch_flag',
        'transaction_channel',
        'card_present',
        'merchant_category',
        'suspicious_cluster_member',
    ]
    
    # Ensure columns exist
    available_cols = df.columns
    feature_cols = [c for c in feature_cols if c in available_cols]
    
    # Handle Categoricals
    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical) for c in string_cols])

    print(f"🧠 Loading Model: models/fraud_model_v1.json")
    model = xgb.XGBClassifier()
    model.load_model("models/fraud_model_v1.json")

    X_test = df.select(feature_cols)
    y_test = df.select(target_col).to_numpy().flatten()

    print("🔮 Running Predictions...")
    y_prob = model.predict_proba(X_test)[:, 1]
    y_pred = (y_prob > 0.5).astype(int)

    # Evaluation
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
