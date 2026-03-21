import polars as pl
import xgboost as xgb
import clickhouse_connect
from sklearn.model_selection import train_test_split
from sklearn.metrics import classification_report, roc_auc_score
import os


def train_model():
    print("🚀 Connecting to ClickHouse...")
    client = clickhouse_connect.get_client(
        host="localhost",
        port=8123,
        username="riskfabric_user",
        password="123",
        database="riskfabric",
    )

    print("📊 Loading Gold Master Table...")
    query = "SELECT * FROM fact_transactions_gold"
    df = pl.from_arrow(client.query_arrow(query))

    # Target label
    target_col = "is_fraud"

    # Clean feature list (No identifiers, No label-leakage, No low-signal stubs)
    feature_cols = [
        # Behavioral sequence
        "time_since_last_transaction",
        "transaction_sequence_number",
        "spatial_velocity",
        "hour_deviation_from_norm",
        "amount_deviation_z_score",
        "rapid_fire_transaction_flag",
        "escalating_amounts_flag",
        "merchant_category_switch_flag",
        # Transaction context
        "transaction_channel",
        "card_present",
        "merchant_category",
        # Network structural (currently disabled/zeroed in ETL due to unresolved signal issues)
        "suspicious_cluster_member",
    ]

    # Ensure columns exist and clean any naming artifacts (like 'g.' prefixes from SQL joins)
    available_cols = df.columns
    feature_cols = [c for c in feature_cols if c in available_cols]

    print(f"🧠 Training on {len(feature_cols)} Operational features:")
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
        stratify=df[target_col].to_numpy(),
    )

    X_train = df[train_idx].select(feature_cols)
    y_train = df[train_idx].select(target_col).to_numpy().flatten()
    X_test = df[test_idx].select(feature_cols)
    y_test = df[test_idx].select(target_col).to_numpy().flatten()

    # 3. Training
    print("⚖️ Calculating scale_pos_weight...")
    fraud_count = y_train.sum()
    legitimate_count = len(y_train) - fraud_count
    scale_pos_weight = legitimate_count / fraud_count
    print(f"   -> Positive Weight: {scale_pos_weight:.4f}")

    model = xgb.XGBClassifier(
        n_estimators=100,
        max_depth=6,
        learning_rate=0.1,
        objective="binary:logistic",
        tree_method="hist",
        enable_categorical=True,
        random_state=42,
        scale_pos_weight=scale_pos_weight,
        eval_metric="aucpr",
    )

    print("🚀 Training Model...")
    model.fit(X_train, y_train)

    print("💾 Saving Model...")
    os.makedirs("models", exist_ok=True)
    model.save_model("models/fraud_model_v1.json")

    # 4. Evaluation
    y_prob = model.predict_proba(X_test)[:, 1]
    auc = roc_auc_score(y_test, y_prob)

    print(f"\n✨ Operational ROC AUC Score: {auc:.4f}")
    print("\n🔝 Top Features by Importance:")
    importance = model.feature_importances_
    feat_imp = sorted(zip(feature_cols, importance), key=lambda x: x[1], reverse=True)
    for feat, imp in feat_imp[:10]:
        print(f" - {feat}: {imp:.4f}")


if __name__ == "__main__":
    train_model()
