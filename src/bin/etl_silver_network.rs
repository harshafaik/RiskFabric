use polars::prelude::*;
use riskfabric::etl::features::network::transform_network_features;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Network Relationship Features...");

    // 1. Load Data from ClickHouse
    let query = "
        SELECT 
            transaction_id, 
            customer_id, 
            merchant_id, 
            user_agent, 
            ip_address, 
            toUInt32(is_fraud) as is_fraud 
        FROM fact_transactions_bronze 
        FORMAT Parquet
    ";
    let output = Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", query]).output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

    println!("📊 Loaded {} transactions. Analyzing network clusters...", df.height());

    // 2. Apply Transformations
    let enriched_lf = transform_network_features(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Network features calculated. Prepared {} rows.", enriched_df.height());

    // 3. Sink to ClickHouse
    Command::new("clickhouse-client").args(["--database", "riskfabric", "--query", "
        CREATE TABLE IF NOT EXISTS network_features_silver (
            customer_id String,
            related_entity_id String,
            relationship_type String,
            shared_entity_fraud_rate Float64,
            degree UInt32,
            suspicious_cluster_member UInt32
        ) ENGINE = MergeTree() ORDER BY customer_id
    "]).status()?;

    let temp_path = "data/tmp_silver_network.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("clickhouse-client --database riskfabric --query \"INSERT INTO network_features_silver FORMAT Parquet\" < {}", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated network_features_silver!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
