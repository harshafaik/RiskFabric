import polars as pl
import xgboost as xgb
import clickhouse_connect
from sklearn.model_selection import train_test_split
from sklearn.metrics import classification_report, roc_auc_score
import os

def train_model():
    print("🚀 Connecting to ClickHouse...")
    client = clickhouse_connect.get_client(host='localhost', database='riskfabric')

    print("📊 Loading Gold Master Table via Polars...")
    # Fetch data as a Polars DataFrame (via pyarrow)
    query = "SELECT * FROM fact_transactions_gold"
    df = pl.from_arrow(client.query_arrow(query))
    
    print(f"✅ Loaded {df.height} rows with {df.width} columns.")

    # 1. Data Cleaning & Feature Selection
    drop_cols = ['transaction_id', 'timestamp', 'feature_calculated_at', 'is_fraud']
    
    # Identify target and features
    target_col = 'fraud_target'
    feature_cols = [str(c) for c in df.columns if str(c) not in drop_cols and str(c) != target_col]

    # Convert categorical strings to pl.Categorical for XGBoost
    cat_cols = ['merchant_category', 'transaction_channel']
    
    # Preparation for XGBoost categorical support
    df = df.with_columns([
        pl.col(c).cast(pl.Categorical) for c in cat_cols
    ])

    X = df.select(feature_cols)
    y = df.select(target_col)

    # 2. Train/Test Split
    train_idx, test_idx = train_test_split(
        range(df.height), 
        test_size=0.2, 
        random_state=42, 
        stratify=y.to_numpy().flatten()
    )

    X_train, X_test = X[train_idx], X[test_idx]
    y_train, y_test = y[train_idx], y[test_idx]

    print(f"🧠 Training XGBoost Model on {len(feature_cols)} features...")
    
    model = xgb.XGBClassifier(
        n_estimators=100,
        max_depth=6,
        learning_rate=0.1,
        objective='binary:logistic',
        tree_method='hist',
        enable_categorical=True,
        random_state=42,
        eval_metric='auc'
    )

    model.fit(X_train, y_train)

    # 3. Evaluation
    print("\n📈 Evaluating Model Performance...")
    y_pred = model.predict(X_test)
    y_prob = model.predict_proba(X_test)[:, 1]

    print("\nClassification Report (Ground Truth):")
    print(classification_report(y_test.to_numpy().flatten(), y_pred))

    auc = roc_auc_score(y_test.to_numpy().flatten(), y_prob)
    print(f"ROC AUC Score: {auc:.4f}")

    # 4. Feature Importance
    print("\n🔝 Top 10 Features by Importance:")
    importance = model.feature_importances_
    feat_imp = sorted(zip(feature_cols, importance), key=lambda x: x[1], reverse=True)
    for feat, imp in feat_imp[:10]:
        print(f" - {feat}: {imp:.4f}")

    # 5. Save Model
    os.makedirs('models', exist_ok=True)
    model_path = 'models/fraud_model_v1.json'
    model.save_model(model_path)
    print(f"\n✨ Model successfully saved to {model_path}")

if __name__ == "__main__":
    train_model()
