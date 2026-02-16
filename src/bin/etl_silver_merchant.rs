use polars::prelude::*;
use riskfabric::etl::features::merchant::transform_merchant_features;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Merchant Features...");

    // 1. Load Data from ClickHouse
    // Casting is_fraud to UInt32 to avoid Polars engine panics
    let query = "
        SELECT 
            transaction_id, 
            merchant_id, 
            customer_id, 
            amount, 
            toUInt32(is_fraud) as is_fraud,
            toUInt32(card_present) as card_present,
            toUInt32(chargeback) as chargeback
        FROM fact_transactions_bronze 
        FORMAT Parquet
    ";
    let output = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", query]).output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

    println!("📊 Loaded {} transactions. Processing merchant aggregates...", df.height());

    // 2. Apply Transformations
    let enriched_lf = transform_merchant_features(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Merchant features calculated. Prepared {} rows.", enriched_df.height());

    // 3. Sink to ClickHouse
    Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "
        CREATE TABLE IF NOT EXISTS merchant_features_silver (
            merchant_id String,
            total_transactions UInt32,
            total_fraud_transactions UInt32,
            unique_customers_count UInt32,
            avg_transaction_amount Float64,
            std_transaction_amount Float64,
            merchant_fraud_rate Float64
        ) ENGINE = ReplacingMergeTree() ORDER BY merchant_id
    "]).status()?;

    let temp_path = "data/tmp_silver_merchant.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("clickhouse-client --database riskfabric --query \"INSERT INTO merchant_features_silver FORMAT Parquet\" < {}", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated merchant_features_silver!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
