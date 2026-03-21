import polars as pl
import xgboost as xgb
import clickhouse_connect
import os

def verify_leakage():
    print("🚀 Connecting to ClickHouse...")
    client = clickhouse_connect.get_client(
        host='localhost', 
        port=8123,
        username='riskfabric_user',
        password='123',
        database='riskfabric'
    )

    print("📊 Loading Test Data (Seed 8888)...")
    query = "SELECT * FROM fact_transactions_gold"
    df = pl.from_arrow(client.query_arrow(query))
    
    feature_cols = [
        'time_since_last_transaction', 'transaction_sequence_number', 'spatial_velocity',
        'hour_deviation_from_norm', 'amount_deviation_z_score', 'rapid_fire_transaction_flag',
        'escalating_amounts_flag', 'merchant_category_switch_flag', 'transaction_channel',
        'card_present', 'merchant_category', 'suspicious_cluster_member',
    ]
    
    # Handle Categoricals
    string_cols = [c for c in feature_cols if df[c].dtype == pl.String]
    if string_cols:
        df = df.with_columns([pl.col(c).cast(pl.Categorical) for c in string_cols])

    print("🧠 Loading Model...")
    model = xgb.XGBClassifier()
    model.load_model("models/fraud_model_v1.json")

    print("🔮 Running Predictions...")
    X_test = df.select(feature_cols)
    y_prob = model.predict_proba(X_test)[:, 1]
    df = df.with_columns(pl.Series("prob", y_prob))

    # Threshold analysis
    THRESHOLD = 0.957
    high_prob_df = df.filter(pl.col("prob") >= THRESHOLD)
    
    print(f"\n🔍 Leakage Analysis at Threshold {THRESHOLD}:")
    print(f"{'Category':<20} {'Global Share':>12} {'Flag Share':>12} {'Index':>10}")
    print("-" * 60)

    total_count = len(df)
    flag_count = len(high_prob_df)

    # Calculate distributions
    global_dist = df.group_by("merchant_category").count().with_columns(
        (pl.col("count") / total_count).alias("global_share")
    )
    flag_dist = high_prob_df.group_by("merchant_category").count().with_columns(
        (pl.col("count") / flag_count).alias("flag_share")
    )

    analysis = global_dist.join(flag_dist, on="merchant_category", how="left").fill_null(0)
    analysis = analysis.with_columns(
        (pl.col("flag_share") / pl.col("global_share")).alias("index")
    ).sort("index", descending=True)

    for row in analysis.iter_rows(named=True):
        print(f"{row['merchant_category']:<20} "
              f"{row['global_share']:>12.2%} "
              f"{row['flag_share']:>12.2%} "
              f"{row['index']:>10.2f}x")

    print(f"\nTotal Flags at this threshold: {flag_count}")

if __name__ == "__main__":
    verify_leakage()
