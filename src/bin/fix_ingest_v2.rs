use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = "data/output";

    println!("Re-ingesting transactions with merchant_country...");

    Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "DROP TABLE IF EXISTS fact_transactions_bronze_raw"])
        .status()?;
    Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "DROP TABLE IF EXISTS fact_transactions_bronze"])
        .status()?;

    let schema = "transaction_id String, customer_id String, merchant_id String, amount Float64, timestamp String, currency String, merchant_category String, merchant_sub_category String, merchant_latitude Float64, merchant_longitude Float64, merchant_country String, entry_mode String, pin_entry String, is_fraud UInt8";
    let create_query = format!("CREATE TABLE fact_transactions_bronze ({}) ENGINE = MergeTree() ORDER BY tuple()", schema);
    Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", &create_query])
        .status()?;

    let ingest_cmd = format!("cat {}/transactions.parquet | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO fact_transactions_bronze FORMAT Parquet\"", output_dir);
    Command::new("sh").arg("-c").arg(&ingest_cmd).status()?;

    println!("Converting to final typed table with country...");
    Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "
            CREATE TABLE fact_transactions_bronze_final ENGINE = MergeTree() ORDER BY timestamp AS 
            SELECT 
                transaction_id, customer_id, merchant_id, amount, 
                parseDateTime64BestEffort(timestamp, 3, 'UTC') as timestamp, 
                currency, merchant_category, merchant_sub_category, 
                merchant_latitude, merchant_longitude, merchant_country, 
                entry_mode, pin_entry, is_fraud 
            FROM fact_transactions_bronze
        "])
        .status()?;

    Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "RENAME TABLE fact_transactions_bronze TO fact_transactions_bronze_raw, fact_transactions_bronze_final TO fact_transactions_bronze"])
        .status()?;

    println!("✅ Done.");
    Ok(())
}
