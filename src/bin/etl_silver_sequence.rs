use polars::prelude::*;
use riskfabric::etl::features::sequence::transform_sequence_features;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Sequence Features...");

    // 1. Load Bronze Data from ClickHouse
    println!("   ... loading bronze transactions");
    let query = "
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
    let output = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", query])
        .output()?;

    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;
    
    // 1.1 Load Fraud Metadata from ClickHouse
    println!("   ... loading fraud metadata");
    let meta_query = "
        SELECT 
            transaction_id, 
            toUInt32(fraud_target) as fraud_target, 
            fraud_type, 
            label_noise, 
            injector_version, 
            toUInt32(geo_anomaly) as geo_anomaly, 
            toUInt32(device_anomaly) as device_anomaly, 
            toUInt32(ip_anomaly) as ip_anomaly, 
            toUInt32(burst_session) as burst_session, 
            burst_seq, 
            campaign_id, 
            campaign_type, 
            campaign_phase, 
            campaign_day_number
        FROM fact_fraud_metadata_bronze 
        FORMAT Parquet
    ";
    let meta_out = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", meta_query])
        .output()?;
    let meta_df = ParquetReader::new(std::io::Cursor::new(meta_out.stdout)).finish()?;

    println!("📊 Loaded {} transactions and {} metadata rows.", df.height(), meta_df.height());

    // 2. Apply Transformations
    let enriched_lf = transform_sequence_features(df.lazy(), meta_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Features calculated. Prepared {} rows.", enriched_df.height());

    // 3. Sink to ClickHouse fact_transactions_silver
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "
        CREATE TABLE IF NOT EXISTS fact_transactions_silver (
            transaction_id String,
            time_since_last_transaction Float64,
            transaction_sequence_number UInt32,
            hours_since_midnight Float64,
            is_weekend UInt32,
            amount_round_number_flag UInt32,
            rapid_fire_transaction_flag UInt32,
            escalating_amounts_flag UInt32,
            merchant_category_switch_flag UInt32,
            fraud_target UInt32,
            geo_anomaly UInt32,
            device_anomaly UInt32,
            ip_anomaly UInt32
        ) ENGINE = MergeTree() ORDER BY transaction_id
    "]).status()?;

    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "TRUNCATE TABLE fact_transactions_silver"]).status()?;

    let temp_path = "data/tmp_silver_sequence.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO fact_transactions_silver FORMAT Parquet\"", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated fact_transactions_silver!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
