use polars::prelude::*;
use riskfabric::etl::features::device_ip::transform_device_ip_intelligence;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Device & IP Intelligence...");

    // 1. Load Data from ClickHouse
    // Casting is_fraud to UInt32 for Polars compatibility
    let query = "
        SELECT 
            transaction_id, 
            customer_id, 
            merchant_id, 
            merchant_country,
            user_agent, 
            ip_address, 
            toUInt32(is_fraud) as is_fraud 
        FROM fact_transactions_bronze 
        FORMAT Parquet
    ";
    let output = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", query]).output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

    println!("📊 Loaded {} transactions. Processing device/IP aggregates...", df.height());

    // 2. Apply Transformations
    let enriched_lf = transform_device_ip_intelligence(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Device/IP features calculated. Prepared {} rows.", enriched_df.height());

    // 3. Sink to ClickHouse
    Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "
        CREATE TABLE IF NOT EXISTS device_features_silver (
            user_agent String,
            transaction_count UInt32,
            fraud_tx_count UInt32,
            unique_customers_per_device UInt32,
            countries_accessed_from UInt32,
            device_fraud_rate Float64
        ) ENGINE = ReplacingMergeTree() ORDER BY user_agent
    "]).status()?;

    let temp_path = "data/tmp_silver_device.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("clickhouse-client --database riskfabric --query \"INSERT INTO device_features_silver FORMAT Parquet\" < {}", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated device_features_silver!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
