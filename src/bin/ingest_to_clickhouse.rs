use std::process::Command;

fn ingest_file(table: &str, file_path: &str, schema: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Ingesting {} into ClickHouse table {}...", file_path, table);
    
    // 1. Create table with appropriate schema if not exists
    let create_query = format!("CREATE TABLE IF NOT EXISTS {} ({}) ENGINE = MergeTree() ORDER BY tuple()", table, schema);
    let status = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", &create_query])
        .status()?;
    
    if !status.success() {
        return Err(format!("Failed to create table {}", table).into());
    }

    // 2. Ingest Parquet data
    // We use cat and pipe it into the container's clickhouse-client
    let ingest_cmd = format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO {} FORMAT Parquet\"", file_path, table);
    let status = Command::new("sh")
        .arg("-c")
        .arg(&ingest_cmd)
        .status()?;

    if status.success() {
        println!("✅ Success!");
    } else {
        println!("❌ Failed to ingest {}", file_path);
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = "data/output";

    // Bronze Layer: Raw Data from Generator
    ingest_file(
        "fact_transactions_bronze", 
        &format!("{}/transactions.parquet", output_dir),
        "transaction_id String, customer_id String, merchant_id String, amount Float64, timestamp DateTime64(3, 'UTC'), currency String, merchant_category String, merchant_sub_category String, merchant_latitude Float64, merchant_longitude Float64, merchant_country String, entry_mode String, pin_entry String, is_fraud UInt8"
    )?;

    ingest_file(
        "dim_customers", 
        &format!("{}/customers.parquet", output_dir),
        "customer_id String, name String, age UInt32, email String, location String, state String, location_type String, home_latitude Float64, home_longitude Float64, home_h3r5 String, home_h3r7 String, credit_score UInt32, monthly_spend Float64, customer_risk_score Float64, is_fraud UInt8, registration_date Date, registration_year UInt32, registration_month UInt32, registration_day UInt32"
    )?;

    ingest_file(
        "dim_accounts", 
        &format!("{}/accounts.parquet", output_dir),
        "account_id String, customer_id String, account_type String, open_date Date, balance Float64, status String"
    )?;

    ingest_file(
        "dim_cards", 
        &format!("{}/cards.parquet", output_dir),
        "card_id String, account_id String, card_type String, card_network String, expiry_date Date, status String"
    )?;

    println!("\n✨ Bronze layer ingestion complete!");
    Ok(())
}
