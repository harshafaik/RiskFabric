import polars as pl
import xgboost as xgb
import clickhouse_connect
from sklearn.model_selection import train_test_split
from sklearn.metrics import classification_report, roc_auc_score
import os

def train_model():
    print("🚀 Connecting to ClickHouse...")
    client = clickhouse_connect.get_client(host='localhost', database='riskfabric')

    print("📊 Loading Gold Master Table...")
    query = "SELECT * FROM fact_transactions_gold"
    df = pl.from_arrow(client.query_arrow(query))
    
    # 1. CLEANING & LEAKAGE REMOVAL
    # We remove anything that contains the answer or look-ahead stats
    target_col = 'is_fraud'
    
    # Explicitly dropping features that cause leakage
    drop_cols = [
        'transaction_id', 't.transaction_id', 'timestamp', 'feature_calculated_at',
        'is_fraud', 'fraud_target',
        'cf_fraud_rate', 'mf_fraud_rate', # <--- THE BIG LEAKAGE: Contains current label info
        'geo_anomaly', 'device_anomaly', 'ip_anomaly' # Exclude injector metadata
    ]
    
    feature_cols = [c for c in df.columns if c not in drop_cols]
    
    print(f"🧠 Training on {len(feature_cols)} HONEST features:")
    print(f"   {feature_cols}")

    # Handle Categoricals
    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical) for c in string_cols])

    # 2. Train/Test Split
    train_idx, test_idx = train_test_split(
        range(df.height), 
        test_size=0.2, 
        random_state=42, 
        stratify=df[target_col].to_numpy()
    )

    X_train = df[train_idx].select(feature_cols)
    y_train = df[train_idx].select(target_col).to_numpy().flatten()
    X_test = df[test_idx].select(feature_cols)
    y_test = df[test_idx].select(target_col).to_numpy().flatten()

    # 3. Training
    model = xgb.XGBClassifier(
        n_estimators=100,
        max_depth=6,
        learning_rate=0.1,
        objective='binary:logistic',
        tree_method='hist',
        enable_categorical=True,
        random_state=42
    )

    model.fit(X_train, y_train)

    # 4. Evaluation
    y_prob = model.predict_proba(X_test)[:, 1]
    auc = roc_auc_score(y_test, y_prob)
    
    print(f"\n✨ HONEST ROC AUC Score: {auc:.4f}")
    print("\n🔝 Top Features by Importance:")
    importance = model.feature_importances_
    feat_imp = sorted(zip(feature_cols, importance), key=lambda x: x[1], reverse=True)
    for feat, imp in feat_imp[:10]:
        print(f" - {feat}: {imp:.4f}")

if __name__ == "__main__":
    train_model()
