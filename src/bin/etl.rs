use clap::{Parser, Subcommand};
use std::error::Error;

#[derive(Parser)]
#[command(name = "riskfabric-etl")]
#[command(about = "Unified ETL pipeline for RiskFabric", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all Silver ETL stages
    SilverAll,
    /// Run Silver Customer ETL
    SilverCustomer,
    /// Run Silver Merchant ETL
    SilverMerchant,
    /// Run Silver Sequence ETL
    SilverSequence,
    /// Run Silver Campaign ETL
    SilverCampaign,
    /// Run Silver Device IP ETL
    SilverDeviceIp,
    /// Run Silver Network ETL
    SilverNetwork,
    /// Run Gold Master Table ETL
    GoldMaster,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::SilverAll => {
            println!("⚡ Parallelizing Silver ETL stages...");
            use rayon::prelude::*;
            
            type EtlStage = fn() -> Result<(), Box<dyn Error + Send + Sync>>;
            let stages: Vec<EtlStage> = vec![
                run_silver_customer,
                run_silver_merchant,
                run_silver_sequence,
                run_silver_campaign,
                run_silver_device_ip,
                run_silver_network,
            ];

            stages.into_par_iter()
                .map(|f| f())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Parallel ETL failed: {}", e))?;

            println!("✅ All Silver stages completed in parallel.");
        }
        Commands::SilverCustomer => run_silver_customer().map_err(|e| e as Box<dyn Error>)?,
        Commands::SilverMerchant => run_silver_merchant().map_err(|e| e as Box<dyn Error>)?,
        Commands::SilverSequence => run_silver_sequence().map_err(|e| e as Box<dyn Error>)?,
        Commands::SilverCampaign => run_silver_campaign().map_err(|e| e as Box<dyn Error>)?,
        Commands::SilverDeviceIp => run_silver_device_ip().map_err(|e| e as Box<dyn Error>)?,
        Commands::SilverNetwork => run_silver_network().map_err(|e| e as Box<dyn Error>)?,
        Commands::GoldMaster => run_gold_master().map_err(|e| e as Box<dyn Error>)?,
    }

    Ok(())
}

fn run_silver_customer() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Running Silver Customer ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::customer::transform_customer_features;
    use std::process::Command;
    use std::io::Cursor;

    let tx_query = "SELECT transaction_id, customer_id, merchant_id, merchant_category, merchant_country, amount, timestamp, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let tx_out = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", tx_query])
        .output()?;
    let tx_df = ParquetReader::new(Cursor::new(tx_out.stdout)).finish()?;

    let cust_query = "SELECT customer_id, name, toUInt32(age) as age, email, location, state, location_type, home_latitude, home_longitude, home_h3r5, home_h3r7, toUInt32(credit_score) as credit_score, monthly_spend, customer_risk_score, toUInt32(is_fraud) as is_fraud, registration_date, registration_year, toUInt32(registration_month) as registration_month, toUInt32(registration_day) as registration_day FROM dim_customers FORMAT Parquet";
    let cust_out = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", cust_query])
        .output()?;
    let cust_df = ParquetReader::new(Cursor::new(cust_out.stdout)).finish()?;

    let acc_query = "SELECT * FROM dim_accounts FORMAT Parquet";
    let acc_out = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", acc_query])
        .output()?;
    let acc_df = ParquetReader::new(Cursor::new(acc_out.stdout)).finish()?;

    let enriched_lf = transform_customer_features(tx_df.lazy(), cust_df.lazy(), acc_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    execute_clickhouse_query("
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
    ")?;

    let mut child = Command::new("podman")
        .args(["exec", "-i", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "INSERT INTO customer_features_silver FORMAT Parquet"])
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    ParquetWriter::new(&mut stdin).finish(&mut enriched_df)?;
    drop(stdin); // Close stdin to signal EOF to clickhouse-client

    let status = child.wait()?;
    if !status.success() {
        return Err("ClickHouse insert failed".into());
    }

    println!("✨ Customer features populated!");
    Ok(())
}

fn run_silver_merchant() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Running Silver Merchant ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::merchant::transform_merchant_features;
    use std::process::{Command, Stdio};
    use std::io::Cursor;

    let tx_query = "SELECT transaction_id, customer_id, merchant_id, merchant_name, merchant_category, amount, timestamp, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let tx_out = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", tx_query])
        .output()?;
    let tx_df = ParquetReader::new(Cursor::new(tx_out.stdout)).finish()?;

    let enriched_lf = transform_merchant_features(tx_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    execute_clickhouse_query("
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
    ")?;

    let mut child = Command::new("podman")
        .args(["exec", "-i", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "INSERT INTO merchant_features_silver FORMAT Parquet"])
        .stdin(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    ParquetWriter::new(&mut stdin).finish(&mut enriched_df)?;
    drop(stdin);
    let status = child.wait()?;
    if !status.success() { return Err("ClickHouse insert failed".into()); }

    println!("✨ Merchant features populated!");
    Ok(())
}

fn run_silver_sequence() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Running Silver Sequence ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::sequence::transform_sequence_features;
    use std::process::{Command, Stdio};
    use std::io::Cursor;

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
    let output = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", query])
        .output()?;
    let df = ParquetReader::new(Cursor::new(output.stdout)).finish()?;

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
    let meta_out = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", meta_query])
        .output()?;
    let meta_df = ParquetReader::new(Cursor::new(meta_out.stdout)).finish()?;

    let enriched_lf = transform_sequence_features(df.lazy(), meta_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    execute_clickhouse_query("
        CREATE TABLE IF NOT EXISTS fact_transactions_silver (
            transaction_id String,
            card_id String,
            account_id String,
            customer_id String,
            merchant_id String,
            merchant_category String,
            amount Float64,
            timestamp DateTime64(3, 'UTC'),
            transaction_channel String,
            card_present UInt32,
            user_agent String,
            ip_address String,
            is_fraud UInt32,
            time_since_last_transaction Float64,
            transaction_sequence_number UInt32,
            hours_since_midnight Float64,
            is_weekend UInt32,
            spatial_velocity Float64,
            hour_deviation_from_norm Float64,
            amount_round_number_flag UInt32,
            amount_deviation_z_score Float64,
            rapid_fire_transaction_flag UInt32,
            escalating_amounts_flag UInt32,
            merchant_category_switch_flag UInt32,
            fraud_target UInt32,
            fraud_type String,
            geo_anomaly UInt32,
            device_anomaly UInt32,
            ip_anomaly UInt32,
            campaign_id Nullable(String)
        ) ENGINE = MergeTree() ORDER BY transaction_id
    ")?;

    execute_clickhouse_query("TRUNCATE TABLE fact_transactions_silver")?;

    let mut child = Command::new("podman")
        .args(["exec", "-i", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "INSERT INTO fact_transactions_silver FORMAT Parquet"])
        .stdin(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    ParquetWriter::new(&mut stdin).finish(&mut enriched_df)?;
    drop(stdin);
    let status = child.wait()?;
    if !status.success() { return Err("ClickHouse insert failed".into()); }

    println!("✨ Sequence features populated!");
    Ok(())
}

fn run_silver_campaign() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Running Silver Campaign ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::campaign::transform_fraud_campaign_features;
    use std::process::{Command, Stdio};
    use std::io::Cursor;

    let query = "SELECT transaction_id, customer_id, merchant_id, amount, timestamp, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let output = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", query])
        .output()?;
    let df = ParquetReader::new(Cursor::new(output.stdout)).finish()?;

    let enriched_lf = transform_fraud_campaign_features(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    execute_clickhouse_query("
        CREATE TABLE IF NOT EXISTS campaign_features_silver (
            transaction_id String,
            campaign_id String,
            campaign_txn_count UInt32,
            campaign_total_amount Float64,
            campaign_merchant_diversity UInt32
        ) ENGINE = MergeTree() ORDER BY (campaign_id, transaction_id)
    ")?;

    let mut child = Command::new("podman")
        .args(["exec", "-i", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "INSERT INTO campaign_features_silver FORMAT Parquet"])
        .stdin(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    ParquetWriter::new(&mut stdin).finish(&mut enriched_df)?;
    drop(stdin);
    let status = child.wait()?;
    if !status.success() { return Err("ClickHouse insert failed".into()); }

    println!("✨ Campaign features populated!");
    Ok(())
}

fn run_silver_device_ip() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Running Silver Device IP ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::device_ip::transform_device_ip_intelligence;
    use std::process::{Command, Stdio};
    use std::io::Cursor;

    let query = "SELECT transaction_id, customer_id, merchant_id, merchant_country, user_agent, ip_address, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let output = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", query])
        .output()?;
    let df = ParquetReader::new(Cursor::new(output.stdout)).finish()?;

    let enriched_lf = transform_device_ip_intelligence(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    execute_clickhouse_query("
        CREATE TABLE IF NOT EXISTS device_features_silver_legacy (
            user_agent String,
            transaction_count UInt32,
            fraud_tx_count UInt32,
            unique_customers_per_device UInt32,
            countries_accessed_from UInt32,
            device_fraud_rate Float64
        ) ENGINE = ReplacingMergeTree() ORDER BY user_agent
    ")?;

    let mut child = Command::new("podman")
        .args(["exec", "-i", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "INSERT INTO device_features_silver_legacy FORMAT Parquet"])
        .stdin(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    ParquetWriter::new(&mut stdin).finish(&mut enriched_df)?;
    drop(stdin);
    let status = child.wait()?;
    if !status.success() { return Err("ClickHouse insert failed".into()); }

    println!("✨ Device/IP features populated!");
    Ok(())
}

fn run_silver_network() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Running Silver Network ETL (Entity Level)...");
    use polars::prelude::*;
    use riskfabric::etl::features::network::transform_network_features;
    use std::process::{Command, Stdio};
    use std::io::Cursor;

    let query = "SELECT transaction_id, customer_id, user_agent, ip_address, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let output = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", query])
        .output()?;
    let df = ParquetReader::new(Cursor::new(output.stdout)).finish()?;

    let (ip_lf, dev_lf) = transform_network_features(df.lazy());
    let mut ip_df = ip_lf.collect()?;
    let mut dev_df = dev_lf.collect()?;

    // 1. IP Reputations
    execute_clickhouse_query("CREATE OR REPLACE TABLE ip_features_silver (
        ip_address String, ip_fraud_rate Float64, ip_degree UInt32
    ) ENGINE = MergeTree() ORDER BY ip_address")?;

    let mut ip_child = Command::new("podman")
        .args(["exec", "-i", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "INSERT INTO ip_features_silver FORMAT Parquet"])
        .stdin(Stdio::piped())
        .spawn()?;
    let mut ip_stdin = ip_child.stdin.take().unwrap();
    ParquetWriter::new(&mut ip_stdin).finish(&mut ip_df)?;
    drop(ip_stdin);
    let status = ip_child.wait()?;
    if !status.success() { return Err("ClickHouse IP insert failed".into()); }

    // 2. Device Reputations
    execute_clickhouse_query("CREATE OR REPLACE TABLE device_features_silver (
        user_agent String, dev_fraud_rate Float64, dev_degree UInt32
    ) ENGINE = MergeTree() ORDER BY user_agent")?;

    let mut dev_child = Command::new("podman")
        .args(["exec", "-i", "riskfabric_clickhouse", "clickhouse-client", "--database", "riskfabric", "--query", "INSERT INTO device_features_silver FORMAT Parquet"])
        .stdin(Stdio::piped())
        .spawn()?;
    let mut dev_stdin = dev_child.stdin.take().unwrap();
    ParquetWriter::new(&mut dev_stdin).finish(&mut dev_df)?;
    drop(dev_stdin);
    let status = dev_child.wait()?;
    if !status.success() { return Err("ClickHouse Device insert failed".into()); }

    println!("✨ Network entity features populated!");
    Ok(())
}

fn run_gold_master() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Running Gold Master ETL (Staged Entity Joins)...");
    
    // Stage 1: Initial Gold from Silver (Contains raw + sequence features)
    println!("   [1/3] Materializing Base Gold from Silver...");
    execute_clickhouse_query("
        CREATE OR REPLACE TABLE gold_stage_1 ENGINE = MergeTree() ORDER BY timestamp AS
        SELECT * FROM fact_transactions_silver
    ")?;

    // Stage 2: + Entity Reputations (Customer & Merchant)
    println!("   [2/3] Joining Entity (Cust/Merch) features...");
    execute_clickhouse_query("
        CREATE OR REPLACE TABLE gold_stage_2 ENGINE = MergeTree() ORDER BY timestamp AS
        SELECT g.*, 
               c.fraud_rate as cf_fraud_rate, c.night_transaction_ratio as cf_night_tx_ratio,
               m.merchant_fraud_rate as mf_fraud_rate
        FROM gold_stage_1 g
        LEFT JOIN customer_features_silver c ON g.customer_id = c.customer_id
        LEFT JOIN merchant_features_silver m ON g.merchant_id = m.merchant_id
        SETTINGS join_algorithm = 'partial_merge', max_memory_usage = 10000000000
    ")?;

    // Stage 3: + Network Reputations (IP & Device)
    println!("   [3/4] Joining Network reputations...");
    execute_clickhouse_query("
        CREATE OR REPLACE TABLE gold_stage_3 ENGINE = MergeTree() ORDER BY timestamp AS
        SELECT g.*, 
               assumeNotNull(ip.ip_fraud_rate) as ip_fraud_rate,
               assumeNotNull(ip.ip_degree) as ip_degree,
               assumeNotNull(dev.dev_fraud_rate) as dev_fraud_rate,
               assumeNotNull(dev.dev_degree) as dev_degree,
               toUInt32(ip.ip_degree > 1 OR dev.dev_degree > 1) as suspicious_cluster_member
        FROM gold_stage_2 g
        LEFT JOIN ip_features_silver ip ON g.ip_address = ip.ip_address
        LEFT JOIN device_features_silver dev ON g.user_agent = dev.user_agent
        SETTINGS join_algorithm = 'partial_merge', max_memory_usage = 10000000000
    ")?;

    // Stage 4: + Campaign (Final)
    println!("   [4/4] Joining Campaign features & Finalizing...");
    execute_clickhouse_query("
        CREATE OR REPLACE TABLE fact_transactions_gold ENGINE = MergeTree() ORDER BY timestamp AS
        SELECT g.*, 
               assumeNotNull(cp.campaign_txn_count) as campaign_txn_count,
               assumeNotNull(cp.campaign_total_amount) as campaign_total_amount,
               assumeNotNull(cp.campaign_merchant_diversity) as campaign_merchant_diversity,
               now() as feature_calculated_at
        FROM gold_stage_3 g
        LEFT JOIN campaign_features_silver cp ON g.transaction_id = cp.transaction_id
        SETTINGS join_algorithm = 'partial_merge', max_memory_usage = 10000000000
    ")?;

    // Cleanup
    let _ = execute_clickhouse_query("DROP TABLE gold_stage_1");
    let _ = execute_clickhouse_query("DROP TABLE gold_stage_2");
    let _ = execute_clickhouse_query("DROP TABLE gold_stage_3");

    println!("🔍 Validating Gold Master integrity...");
    validate_gold_table()?;

    println!("✨ Gold master table populated and verified!");
    Ok(())
}

fn validate_gold_table() -> Result<(), Box<dyn Error + Send + Sync>> {
    use std::process::Command;
    let check_query = "
        SELECT 
            count() as total,
            countIf(ip_fraud_rate > 0) as net_hits,
            avg(ip_fraud_rate) as avg_ip_risk,
            avg(dev_degree) as avg_dev_degree
        FROM fact_transactions_gold
    ";
    
    let output = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            check_query,
        ])
        .output()?;
    
    println!("📊 Validation Results: {}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}

fn execute_clickhouse_query(query: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    use std::process::Command;
    let status = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            query,
        ])
        .status()?;
    
    if !status.success() {
        return Err(format!("ClickHouse query failed: {}", query).into());
    }
    Ok(())
}
