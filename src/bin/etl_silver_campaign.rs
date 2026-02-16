use polars::prelude::*;
use riskfabric::etl::features::campaign::transform_fraud_campaign_features;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Fraud Campaign Features...");

    // 1. Load Data from ClickHouse
    let query = "
        SELECT 
            transaction_id, 
            customer_id, 
            merchant_id, 
            amount, 
            timestamp, 
            toUInt32(is_fraud) as is_fraud 
        FROM fact_transactions_bronze 
        FORMAT Parquet
    ";
    let output = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", query]).output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

    println!("📊 Loaded {} transactions. Identifying campaigns...", df.height());

    // 2. Apply Transformations
    let enriched_lf = transform_fraud_campaign_features(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Campaign features calculated. Prepared {} rows.", enriched_df.height());

    // 3. Sink to ClickHouse
    Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "
        CREATE TABLE IF NOT EXISTS campaign_features_silver (
            transaction_id String,
            campaign_id String,
            campaign_txn_count UInt32,
            campaign_total_amount Float64,
            campaign_merchant_diversity UInt32
        ) ENGINE = MergeTree() ORDER BY (campaign_id, transaction_id)
    "]).status()?;

    let temp_path = "data/tmp_silver_campaign.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("clickhouse-client --database riskfabric --query \"INSERT INTO campaign_features_silver FORMAT Parquet\" < {}", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated campaign_features_silver!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
