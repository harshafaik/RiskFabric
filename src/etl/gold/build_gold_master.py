import polars as pl
import clickhouse_connect
import datetime
import sys
import os

def build_gold_master():
    print("🚀 Connecting to ClickHouse...")
    client = clickhouse_connect.get_client(
        host='localhost', 
        port=8123,
        username='riskfabric_user',
        password='123',
        database='riskfabric'
    )

    tmp_dir = "data/tmp_gold"
    os.makedirs(tmp_dir, exist_ok=True)

    def export_to_parquet(table, query, filename):
        print(f"   -> Exporting {table} to Parquet...")
        path = os.path.join(tmp_dir, filename)
        df = pl.from_arrow(client.query_arrow(query))
        df.write_parquet(path)
        return path

    # 1. Export Pruned Tables
    tx_path = export_to_parquet("transactions", "SELECT transaction_id, timestamp, amount, merchant_id, customer_id, merchant_category, transaction_channel, toUInt32(card_present) as card_present, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze", "tx.parquet")
    seq_path = export_to_parquet("sequence", "SELECT transaction_id, fraud_target, fraud_type, time_since_last_transaction, transaction_sequence_number, spatial_velocity, hour_deviation_from_norm, amount_deviation_z_score, rapid_fire_transaction_flag, escalating_amounts_flag, merchant_category_switch_flag, geo_anomaly, device_anomaly, ip_anomaly, campaign_id as simulation_campaign_id FROM fact_transactions_silver", "seq.parquet")
    cust_path = export_to_parquet("customer", "SELECT customer_id, fraud_rate as cf_fraud_rate, night_transaction_ratio as cf_night_tx_ratio FROM customer_features_silver", "cust.parquet")
    merch_path = export_to_parquet("merchant", "SELECT merchant_id, merchant_fraud_rate as mf_fraud_rate FROM merchant_features_silver", "merch.parquet")
    
    # NOTE: The following signals are currently disabled due to unresolved issues with 
    # campaign fraud and device/ip feature signal reliability.
    # net_path = export_to_parquet("network", "SELECT transaction_id, shared_entity_fraud_rate, degree, suspicious_cluster_member FROM network_features_silver", "net.parquet")
    # camp_path = export_to_parquet("campaign", "SELECT transaction_id, campaign_txn_count, campaign_total_amount, campaign_merchant_diversity FROM campaign_features_silver", "camp.parquet")

    # 2. Lazy Streaming Join
    print("🧠 Building streaming join graph...")
    gold_lf = pl.scan_parquet(tx_path).join(pl.scan_parquet(seq_path), on="transaction_id", how="left")
    gold_lf = gold_lf.join(pl.scan_parquet(cust_path), on="customer_id", how="left")
    gold_lf = gold_lf.join(pl.scan_parquet(merch_path), on="merchant_id", how="left")
    
    # NOTE: Network and Campaign joins are currently skipped.
    # gold_lf = gold_lf.join(pl.scan_parquet(net_path), on="transaction_id", how="left")
    # gold_lf = gold_lf.join(pl.scan_parquet(camp_path), on="transaction_id", how="left")

    gold_lf = gold_lf.with_columns([
        # Initialize missing signal columns to 0 for schema compatibility
        pl.lit(0.0).alias("shared_entity_fraud_rate"),
        pl.lit(0).alias("degree"),
        pl.lit(0).alias("suspicious_cluster_member"),
        pl.lit(0).alias("campaign_txn_count"),
        pl.lit(0.0).alias("campaign_total_amount"),
        pl.lit(0).alias("campaign_merchant_diversity"),
        pl.lit(datetime.datetime.now()).alias("feature_calculated_at")
    ])

    final_parquet_path = os.path.join(tmp_dir, "gold_master_final.parquet")
    print(f"🚀 Sinking joined data to disk: {final_parquet_path}...")
    gold_lf.sink_parquet(final_parquet_path)

    # 3. Stream from Final Parquet to ClickHouse in small chunks
    print("💾 Streaming final Parquet into ClickHouse...")
    client.command("DROP TABLE IF EXISTS fact_transactions_gold")
    
    # We use pl.read_parquet with use_pyarrow=True or scan again to iterate
    # But for ClickHouse ingestion, we'll read the final file in chunks.
    batch_size = 500_000
    df_iter = pl.read_parquet(final_parquet_path).iter_slices(n_rows=batch_size)
    
    for i, chunk in enumerate(df_iter):
        print(f"   -> Uploading batch {i+1}...")
        client.insert_df("fact_transactions_gold", chunk)

    # Cleanup
    print("🧹 Cleaning up temporary files...")
    # Update cleanup list to exclude commented-out paths
    for p in [tx_path, seq_path, cust_path, merch_path, final_parquet_path]:
        if os.path.exists(p):
            os.remove(p)
    os.rmdir(tmp_dir)

    print("✅ Gold Master build complete!")

if __name__ == "__main__":
    build_gold_master()
