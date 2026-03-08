use polars::prelude::*;
use riskfabric::etl::features::merchant::transform_merchant_features;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Merchant Features...");

    // 1. Load Data
    println!("Loading bronze transactions...");
    let tx_query = "SELECT transaction_id, merchant_id, merchant_name, merchant_category, amount, timestamp, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let tx_out = Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", tx_query]).output()?;
    let tx_df = ParquetReader::new(std::io::Cursor::new(tx_out.stdout)).finish()?;

    println!("📊 Loaded {} transactions. Processing merchant aggregates...", tx_df.height());

    // 2. Apply Transformations
    let enriched_lf = transform_merchant_features(tx_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Merchant features calculated. Prepared {} rows.", enriched_df.height());

    // 3. Sink
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "
        CREATE TABLE IF NOT EXISTS merchant_features_silver (
            merchant_id String,
            merchant_name String,
            merchant_category String,
            total_transactions UInt32,
            total_amount Float64,
            avg_transaction_amount Float64,
            total_fraud_transactions UInt32,
            merchant_fraud_rate Float64
        ) ENGINE = ReplacingMergeTree() ORDER BY merchant_id
    "]).status()?;

    let temp_path = "data/tmp_silver_merchant.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO merchant_features_silver FORMAT Parquet\"", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated merchant_features_silver!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
