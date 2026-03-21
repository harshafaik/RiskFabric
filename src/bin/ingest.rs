use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = "data/output";
    let db = "riskfabric";

    println!("🧹 Cleaning up existing tables...");
    let cleanup = vec![
        "fact_transactions_bronze", "fact_transactions_bronze_raw", 
        "dim_customers", "dim_accounts", "dim_cards", "fact_fraud_metadata_bronze"
    ];
    for table in cleanup {
        Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", &format!("DROP TABLE IF EXISTS {}", table)]).status()?;
    }

    // 1. Transactions (The most complex one)
    println!("📦 Ingesting Transactions (Full Schema)...");
    let tx_schema_raw = "
        transaction_id String, card_id String, account_id String, customer_id String,
        merchant_id String, merchant_name String, merchant_category String, merchant_country String,
        amount Float64, currency String, timestamp String, transaction_channel String,
        card_present UInt8, user_agent String, ip_address String, status String,
        auth_status String, failure_reason Nullable(String), is_fraud UInt8,
        chargeback UInt8, chargeback_days Nullable(Int32),
        location_lat Float64, location_long Float64, h3_r7 String
    ";
    
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", 
        &format!("CREATE TABLE fact_transactions_bronze_raw ({}) ENGINE = MergeTree() ORDER BY tuple()", tx_schema_raw)]).status()?;

    let ingest_tx = format!("cat {}/transactions.parquet | podman exec -i riskfabric_clickhouse clickhouse-client --database {} --query \"INSERT INTO fact_transactions_bronze_raw FORMAT Parquet\"", output_dir, db);
    Command::new("sh").arg("-c").arg(&ingest_tx).status()?;

    println!("   -> Converting timestamps and creating final bronze table...");
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", "
        CREATE TABLE fact_transactions_bronze ENGINE = MergeTree() ORDER BY timestamp AS 
        SELECT 
            transaction_id, card_id, account_id, customer_id,
            merchant_id, merchant_name, merchant_category, merchant_country,
            amount, currency, 
            parseDateTime64BestEffort(timestamp, 3, 'UTC') as timestamp, 
            transaction_channel, card_present, user_agent, ip_address, 
            status, auth_status, failure_reason, is_fraud, 
            chargeback, chargeback_days,
            location_lat, location_long, h3_r7 
        FROM fact_transactions_bronze_raw
    "]).status()?;

    // 2. Customers
    println!("👤 Ingesting Customers...");
    let cust_schema = "
        customer_id String, name String, age UInt32, email String, location String, 
        state String, location_type String, home_latitude Float64, home_longitude Float64, 
        home_h3r5 String, home_h3r7 String, credit_score UInt32, monthly_spend Float64, 
        customer_risk_score Float64, is_fraud UInt8, registration_date Date, 
        registration_year UInt32, registration_month UInt32, registration_day UInt32
    ";
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", 
        &format!("CREATE TABLE dim_customers ({}) ENGINE = MergeTree() ORDER BY customer_id", cust_schema)]).status()?;

    let ingest_cust = format!("cat {}/customers.parquet | podman exec -i riskfabric_clickhouse clickhouse-client --database {} --query \"INSERT INTO dim_customers FORMAT Parquet\"", output_dir, db);
    Command::new("sh").arg("-c").arg(&ingest_cust).status()?;

    // 3. Accounts
    println!("💰 Ingesting Accounts...");
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", 
        "CREATE TABLE dim_accounts (account_id String, customer_id String, account_type String, open_date Date, balance Float64, status String) ENGINE = MergeTree() ORDER BY account_id"]).status()?;
    
    let ingest_acc = format!("cat {}/accounts.parquet | podman exec -i riskfabric_clickhouse clickhouse-client --database {} --query \"INSERT INTO dim_accounts FORMAT Parquet\"", output_dir, db);
    Command::new("sh").arg("-c").arg(&ingest_acc).status()?;

    // 4. Cards
    println!("💳 Ingesting Cards...");
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", 
        "CREATE TABLE dim_cards (card_id String, account_id String, card_type String, card_network String, expiry_date Date, status String) ENGINE = MergeTree() ORDER BY card_id"]).status()?;
    
    let ingest_card = format!("cat {}/cards.parquet | podman exec -i riskfabric_clickhouse clickhouse-client --database {} --query \"INSERT INTO dim_cards FORMAT Parquet\"", output_dir, db);
    Command::new("sh").arg("-c").arg(&ingest_card).status()?;

    // 5. Fraud Metadata
    println!("🔍 Ingesting Fraud Metadata...");
    let meta_schema = "
        transaction_id String, fraud_target UInt8, fraud_type String, 
        label_noise String, injector_version String, 
        geo_anomaly UInt8, device_anomaly UInt8, ip_anomaly UInt8, 
        burst_session UInt8, burst_seq String, 
        campaign_id Nullable(String), campaign_type Nullable(String), 
        campaign_phase String, campaign_day_number Int32
    ";
    Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", 
        &format!("CREATE TABLE fact_fraud_metadata_bronze ({}) ENGINE = MergeTree() ORDER BY transaction_id", meta_schema)]).status()?;
    
    let ingest_meta = format!("cat {}/fraud_metadata.parquet | podman exec -i riskfabric_clickhouse clickhouse-client --database {} --query \"INSERT INTO fact_fraud_metadata_bronze FORMAT Parquet\"", output_dir, db);
    Command::new("sh").arg("-c").arg(&ingest_meta).status()?;

    let count = Command::new("podman").args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", "SELECT count() FROM fact_transactions_bronze"]).output()?;
    println!("\n✅ All layers ingested! Total transactions: {}", String::from_utf8_lossy(&count.stdout).trim());

    Ok(())
}
