import polars as pl
import xgboost as xgb
import duckdb
import numpy as np
from sklearn.model_selection import train_test_split
from sklearn.metrics import roc_auc_score
import os
import glob
from datetime import datetime
from argparse import ArgumentParser


def find_gold_snapshot(snapshot: str | None = None) -> str:
    if snapshot:
        path = f"data/gold/{snapshot}/fact_transactions_gold.parquet"
        if os.path.exists(path):
            return path
        raise FileNotFoundError(f"Snapshot not found: {path}")

    snapshots = sorted(glob.glob("data/gold/*/fact_transactions_gold.parquet"), reverse=True)
    if not snapshots:
        raise FileNotFoundError("No Gold snapshots found in data/gold/*/. Run `cargo run --bin etl -- gold-master` first.")
    return snapshots[0]


def train_model():
    parser = ArgumentParser(description="Train XGBoost fraud detection model from Gold Parquet snapshots")
    parser.add_argument("--snapshot", type=str, default=None, help="Specific snapshot directory (e.g., 20260716_120000)")
    args = parser.parse_args()

    gold_path = find_gold_snapshot(args.snapshot)
    print(f"📊 Loading Gold snapshot: {gold_path}")

    conn = duckdb.connect()
    query = f"SELECT * FROM '{gold_path}'"
    df = pl.from_arrow(conn.execute(query).arrow())
    conn.close()

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
    ]

    available_cols = df.columns
    feature_cols = [c for c in feature_cols if c in available_cols]
    missing = set(feature_cols) - set(available_cols)
    if missing:
        print(f"   ⚠️ Missing from Gold: {missing}")

    print(f"🧠 Training on {len(feature_cols)} features:")
    print(f"   {feature_cols}")

    train_idx, test_idx = train_test_split(
        range(len(df)),
        test_size=0.2,
        random_state=42,
        stratify=df[target_col],
    )

    train_df = df[train_idx]
    test_df = df[test_idx]

    print(f"   Train: {len(train_df):,} rows, Test: {len(test_df):,} rows (stratified split)")

    X_train = train_df.select(feature_cols).to_pandas()
    y_train = train_df[target_col].to_numpy()
    X_test = test_df.select(feature_cols).to_pandas()
    y_test = test_df[target_col].to_numpy()

    categorical_cols = [c for c in feature_cols if train_df[c].dtype == pl.String]
    if categorical_cols:
        print(f"   Categorical features: {categorical_cols}")
        for c in categorical_cols:
            X_train[c] = X_train[c].astype("category")
            X_test[c] = X_test[c].astype("category")

    fraud_count = y_train.sum()
    legitimate_count = len(y_train) - fraud_count
    scale_pos_weight = legitimate_count / fraud_count
    print(f"⚖️ Scale positive weight: {scale_pos_weight:.4f}")

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

    y_prob = model.predict_proba(X_test)[:, 1]
    auc = roc_auc_score(y_test, y_prob)

    print(f"\n✨ ROC AUC: {auc:.4f}")
    print("\n📊 Feature Importance (gain):")
    importance = model.feature_importances_
    total = sum(importance)
    feat_imp = sorted(zip(feature_cols, importance), key=lambda x: x[1], reverse=True)
    for feat, imp in feat_imp:
        print(f"  {feat:40s} {imp:10.4f} ({imp/total*100:5.1f}%)")

    os.makedirs("models", exist_ok=True)
    ts = datetime.now().strftime("%Y%m%d")
    name = f"models/fraud_model_v4.json"
    suffix = 2
    while os.path.exists(name):
        name = f"models/fraud_model_v4({suffix}).json"
        suffix += 1
    model.save_model(name)
    print(f"\n💾 Saved: {name}")


if __name__ == "__main__":
    train_model()
