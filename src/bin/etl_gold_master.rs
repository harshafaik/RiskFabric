use polars::prelude::*;
use riskfabric::etl::gold::gold_master::create_gold_master_table;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Gold ETL: Master ML Table...");

    // 1. Load All Layers from ClickHouse
    println!("   ... loading Bronze and Silver layers");
    
    // Bronze: Cast UInt8 to UInt32
    let tx_query = "
        SELECT 
            transaction_id, card_id, account_id, customer_id, merchant_id, 
            merchant_name, merchant_category, merchant_country,
            amount, currency, timestamp, transaction_channel, 
            toUInt32(card_present) as card_present, 
            user_agent, ip_address, status, auth_status, failure_reason, 
            toUInt32(is_fraud) as is_fraud, 
            toUInt32(chargeback) as chargeback, 
            chargeback_days, location_lat, location_long, h3_r7
        FROM fact_transactions_bronze 
        FORMAT Parquet
    ";
    let tx_out = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", tx_query]).output()?;
    let tx_df = ParquetReader::new(std::io::Cursor::new(tx_out.stdout)).finish()?;

    // Silver Sequence: Load expanded columns
    let seq_query = "
        SELECT 
            transaction_id,
            time_since_last_transaction,
            transaction_sequence_number,
            hours_since_midnight,
            toUInt32(is_weekend) as is_weekend,
            toUInt32(amount_round_number_flag) as amount_round_number_flag,
            toUInt32(rapid_fire_transaction_flag) as rapid_fire_transaction_flag,
            toUInt32(escalating_amounts_flag) as escalating_amounts_flag,
            toUInt32(merchant_category_switch_flag) as merchant_category_switch_flag,
            toUInt32(fraud_target) as fraud_target,
            toUInt32(geo_anomaly) as geo_anomaly,
            toUInt32(device_anomaly) as device_anomaly,
            toUInt32(ip_anomaly) as ip_anomaly
        FROM fact_transactions_silver 
        FORMAT Parquet
    ";
    let seq_out = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", seq_query]).output()?;
    let seq_df = ParquetReader::new(std::io::Cursor::new(seq_out.stdout)).finish()?;

    // Customer Silver
    let cust_out = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "SELECT * FROM customer_features_silver FORMAT Parquet"]).output()?;
    let cust_df = ParquetReader::new(std::io::Cursor::new(cust_out.stdout)).finish()?;

    // Merchant Silver
    let merch_out = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "SELECT * FROM merchant_features_silver FORMAT Parquet"]).output()?;
    let merch_df = ParquetReader::new(std::io::Cursor::new(merch_out.stdout)).finish()?;

    // Device Silver
    let dev_out = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "SELECT * FROM device_features_silver FORMAT Parquet"]).output()?;
    let dev_df = ParquetReader::new(std::io::Cursor::new(dev_out.stdout)).finish()?;

    // Campaign Silver
    let camp_out = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "SELECT * FROM campaign_features_silver FORMAT Parquet"]).output()?;
    let camp_df = ParquetReader::new(std::io::Cursor::new(camp_out.stdout)).finish()?;

    // Network Silver
    let net_out = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "SELECT * FROM network_features_silver FORMAT Parquet"]).output()?;
    let net_df = ParquetReader::new(std::io::Cursor::new(net_out.stdout)).finish()?;

    // 2. Execute Joins
    let gold_lf = create_gold_master_table(
        tx_df.lazy(),
        seq_df.lazy(),
        camp_df.lazy(),
        cust_df.lazy(),
        merch_df.lazy(),
        dev_df.lazy(),
        net_df.lazy(),
    );

    let mut gold_df = gold_lf.select([
        col("transaction_id"),
        col("timestamp"),
        col("amount"),
        col("merchant_category"),
        col("transaction_channel"),
        col("card_present"),
        col("is_fraud"), // Noisy label
        col("fraud_target"), // Ground truth
        col("time_since_last_transaction"),
        col("transaction_sequence_number"),
        col("rapid_fire_transaction_flag"),
        col("escalating_amounts_flag"),
        col("merchant_category_switch_flag"),
        col("geo_anomaly"),
        col("device_anomaly"),
        col("ip_anomaly"),
        col("fraud_rate").alias("cf_fraud_rate"),
        col("night_transaction_ratio").alias("cf_night_tx_ratio"),
        col("merchant_fraud_rate").alias("mf_fraud_rate"),
        col("device_fraud_rate").alias("df_fraud_rate"),
        col("unique_customers_per_device").alias("df_unique_cust"),
        col("net_suspicious_cluster_member"),
        col("net_avg_shared_entity_fraud_rate"),
        col("feature_calculated_at"),
    ]).collect()?;
    println!("✅ Gold table created. Rows: {}", gold_df.height());

    // 3. Sink to ClickHouse
    Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "TRUNCATE TABLE fact_transactions_gold"]).status()?;

    let temp_path = "data/tmp_gold_master.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut gold_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("clickhouse-client --database riskfabric --query \"INSERT INTO fact_transactions_gold FORMAT Parquet\" < {}", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated fact_transactions_gold!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
