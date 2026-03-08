use polars::prelude::*;
use riskfabric::etl::features::customer::transform_customer_features;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Silver ETL: Customer Features...");

    // 1. Load Data from ClickHouse
    // Transactions
    println!("Loading bronze transactions...");
    let tx_query = "SELECT transaction_id, customer_id, merchant_id, merchant_category, merchant_country, amount, timestamp, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let tx_out = Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", tx_query]).output()?;
    let tx_df = ParquetReader::new(std::io::Cursor::new(tx_out.stdout)).finish()?;

    // Customers
    println!("Loading dim_customers...");
    let cust_query = "SELECT customer_id, name, toUInt32(age) as age, email, location, state, location_type, home_latitude, home_longitude, home_h3r5, home_h3r7, toUInt32(credit_score) as credit_score, monthly_spend, customer_risk_score, toUInt32(is_fraud) as is_fraud, registration_date, registration_year, toUInt32(registration_month) as registration_month, toUInt32(registration_day) as registration_day FROM dim_customers FORMAT Parquet";
    let cust_out = Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", cust_query]).output()?;
    let cust_df = ParquetReader::new(std::io::Cursor::new(cust_out.stdout)).finish()?;

    // Accounts
    println!("Loading dim_accounts...");
    let acc_query = "SELECT * FROM dim_accounts FORMAT Parquet";
    let acc_out = Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", acc_query]).output()?;
    let acc_df = ParquetReader::new(std::io::Cursor::new(acc_out.stdout)).finish()?;

    println!("📊 Data Loaded. Processing features for {} customers...", cust_df.height());

    // 2. Apply Transformations
    let enriched_lf = transform_customer_features(tx_df.lazy(), cust_df.lazy(), acc_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    println!("✅ Customer features calculated. Prepared {} rows.", enriched_df.height());

    // 3. Sink to ClickHouse (Create table if not exists first)
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "
        CREATE TABLE IF NOT EXISTS customer_features_silver (
            customer_id String,
            name String,
            email String,
            account_count UInt32,
            total_transactions UInt32,
            total_fraud_transactions UInt32,
            fraud_rate Float64,
            avg_transaction_amount Float64,
            night_transaction_ratio Float64,
            weekend_transaction_ratio Float64,
            first_transaction_ts Nullable(DateTime64(3, 'UTC')),
            last_transaction_ts Nullable(DateTime64(3, 'UTC'))
        ) ENGINE = ReplacingMergeTree() ORDER BY customer_id
    "]).status()?;

    let temp_path = "data/tmp_silver_customer.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO customer_features_silver FORMAT Parquet\"", temp_path))
        .status()?;

    if status.success() {
        println!("✨ Successfully populated customer_features_silver!");
        std::fs::remove_file(temp_path)?;
    }

    Ok(())
}
