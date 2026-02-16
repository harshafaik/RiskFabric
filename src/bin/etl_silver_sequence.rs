use polars::prelude::*;
use riskfabric::etl::features::sequence::transform_sequence_features;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Sequence Features...");

    // 1. Load Bronze Data from ClickHouse
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
    let output = Command::new("clickhouse-client")
        .args(["--database", "riskfabric", "--query", query])
        .output()?;

    if !output.status.success() {
        return Err(format!("ClickHouse error: {}", String::from_utf8_lossy(&output.stderr)).into());
    }

    let cursor = std::io::Cursor::new(output.stdout);
    let df = ParquetReader::new(cursor).finish()?;
    
    // 1.1 Load Fraud Metadata from ClickHouse
    println!("   ... loading fraud metadata with UInt32 casting");
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
    let meta_out = Command::new("clickhouse-client")
        .args(["--database", "riskfabric", "--query", meta_query])
        .output()?;
    let meta_df = ParquetReader::new(std::io::Cursor::new(meta_out.stdout)).finish()?;

    println!("📊 Loaded {} transactions and {} metadata rows.", df.height(), meta_df.height());

    let lf = df.lazy();
    let meta_lf = meta_df.lazy();

    // 2. Apply Transformations
    let enriched_lf = transform_sequence_features(lf, meta_lf);
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Features calculated. Prepared {} rows for Silver layer.", enriched_df.height());

    // 3. Sink to ClickHouse fact_transactions_silver
    Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "TRUNCATE TABLE fact_transactions_silver"]).status()?;

    let temp_path = "data/tmp_silver_sequence.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("clickhouse-client --database riskfabric --query \"INSERT INTO fact_transactions_silver FORMAT Parquet\" < {}", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated fact_transactions_silver!");
        std::fs::remove_file(temp_path)?;
    } else {
        println!("❌ Failed to write to ClickHouse.");
    }

    Ok(())
}
