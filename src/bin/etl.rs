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
            run_silver_customer()?;
            run_silver_merchant()?;
            run_silver_sequence()?;
            run_silver_campaign()?;
            run_silver_device_ip()?;
            run_silver_network()?;
        }
        Commands::SilverCustomer => run_silver_customer()?,
        Commands::SilverMerchant => run_silver_merchant()?,
        Commands::SilverSequence => run_silver_sequence()?,
        Commands::SilverCampaign => run_silver_campaign()?,
        Commands::SilverDeviceIp => run_silver_device_ip()?,
        Commands::SilverNetwork => run_silver_network()?,
        Commands::GoldMaster => run_gold_master()?,
    }

    Ok(())
}

// TODO: Move these into library modules later
fn run_silver_customer() -> Result<(), Box<dyn Error>> {
    println!("🚀 Running Silver Customer ETL...");
    // Logic from src/bin/etl_silver_customer.rs
    use polars::prelude::*;
    use riskfabric::etl::features::customer::transform_customer_features;
    use std::process::Command;

    let tx_query = "SELECT transaction_id, customer_id, merchant_id, merchant_category, merchant_country, amount, timestamp, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let tx_out = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            tx_query,
        ])
        .output()?;
    let tx_df = ParquetReader::new(std::io::Cursor::new(tx_out.stdout)).finish()?;

    let cust_query = "SELECT customer_id, name, toUInt32(age) as age, email, location, state, location_type, home_latitude, home_longitude, home_h3r5, home_h3r7, toUInt32(credit_score) as credit_score, monthly_spend, customer_risk_score, toUInt32(is_fraud) as is_fraud, registration_date, registration_year, toUInt32(registration_month) as registration_month, toUInt32(registration_day) as registration_day FROM dim_customers FORMAT Parquet";
    let cust_out = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            cust_query,
        ])
        .output()?;
    let cust_df = ParquetReader::new(std::io::Cursor::new(cust_out.stdout)).finish()?;

    let acc_query = "SELECT * FROM dim_accounts FORMAT Parquet";
    let acc_out = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            acc_query,
        ])
        .output()?;
    let acc_df = ParquetReader::new(std::io::Cursor::new(acc_out.stdout)).finish()?;

    let enriched_lf = transform_customer_features(tx_df.lazy(), cust_df.lazy(), acc_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            "
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
    ",
        ])
        .status()?;

    let temp_path = "data/tmp_silver_customer.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO customer_features_silver FORMAT Parquet\"", temp_path))
        .status()?;

    std::fs::remove_file(temp_path)?;
    println!("✨ Customer features populated!");
    Ok(())
}

fn run_silver_merchant() -> Result<(), Box<dyn Error>> {
    println!("🚀 Running Silver Merchant ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::merchant::transform_merchant_features;
    use std::process::Command;

    let tx_query = "SELECT transaction_id, merchant_id, merchant_name, merchant_category, amount, timestamp, toUInt32(is_fraud) as is_fraud FROM fact_transactions_bronze FORMAT Parquet";
    let tx_out = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            tx_query,
        ])
        .output()?;
    let tx_df = ParquetReader::new(std::io::Cursor::new(tx_out.stdout)).finish()?;

    let enriched_lf = transform_merchant_features(tx_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            "
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
    ",
        ])
        .status()?;

    let temp_path = "data/tmp_silver_merchant.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO merchant_features_silver FORMAT Parquet\"", temp_path))
        .status()?;

    std::fs::remove_file(temp_path)?;
    println!("✨ Merchant features populated!");
    Ok(())
}

fn run_silver_sequence() -> Result<(), Box<dyn Error>> {
    println!("🚀 Running Silver Sequence ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::sequence::transform_sequence_features;
    use std::process::Command;

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
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            query,
        ])
        .output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

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
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            meta_query,
        ])
        .output()?;
    let meta_df = ParquetReader::new(std::io::Cursor::new(meta_out.stdout)).finish()?;

    let enriched_lf = transform_sequence_features(df.lazy(), meta_df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            "
        CREATE TABLE IF NOT EXISTS fact_transactions_silver (
            transaction_id String,
            time_since_last_transaction Float64,
            transaction_sequence_number UInt32,
            hours_since_midnight Float64,
            is_weekend UInt32,
            amount_round_number_flag UInt32,
            rapid_fire_transaction_flag UInt32,
            escalating_amounts_flag UInt32,
            merchant_category_switch_flag UInt32,
            fraud_target UInt32,
            geo_anomaly UInt32,
            device_anomaly UInt32,
            ip_anomaly UInt32
        ) ENGINE = MergeTree() ORDER BY transaction_id
    ",
        ])
        .status()?;

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            "TRUNCATE TABLE fact_transactions_silver",
        ])
        .status()?;

    let temp_path = "data/tmp_silver_sequence.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO fact_transactions_silver FORMAT Parquet\"", temp_path))
        .status()?;

    std::fs::remove_file(temp_path)?;
    println!("✨ Sequence features populated!");
    Ok(())
}

fn run_silver_campaign() -> Result<(), Box<dyn Error>> {
    println!("🚀 Running Silver Campaign ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::campaign::transform_fraud_campaign_features;
    use std::process::Command;

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
    let output = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            query,
        ])
        .output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

    let enriched_lf = transform_fraud_campaign_features(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            "
        CREATE TABLE IF NOT EXISTS campaign_features_silver (
            transaction_id String,
            campaign_id String,
            campaign_txn_count UInt32,
            campaign_total_amount Float64,
            campaign_merchant_diversity UInt32
        ) ENGINE = MergeTree() ORDER BY (campaign_id, transaction_id)
    ",
        ])
        .status()?;

    let temp_path = "data/tmp_silver_campaign.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO campaign_features_silver FORMAT Parquet\"", temp_path))
        .status()?;

    std::fs::remove_file(temp_path)?;
    println!("✨ Campaign features populated!");
    Ok(())
}

fn run_silver_device_ip() -> Result<(), Box<dyn Error>> {
    println!("🚀 Running Silver Device IP ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::device_ip::transform_device_ip_intelligence;
    use std::process::Command;

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
    let output = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            query,
        ])
        .output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

    let enriched_lf = transform_device_ip_intelligence(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            "
        CREATE TABLE IF NOT EXISTS device_features_silver (
            user_agent String,
            transaction_count UInt32,
            fraud_tx_count UInt32,
            unique_customers_per_device UInt32,
            countries_accessed_from UInt32,
            device_fraud_rate Float64
        ) ENGINE = ReplacingMergeTree() ORDER BY user_agent
    ",
        ])
        .status()?;

    let temp_path = "data/tmp_silver_device.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO device_features_silver FORMAT Parquet\"", temp_path))
        .status()?;

    std::fs::remove_file(temp_path)?;
    println!("✨ Device/IP features populated!");
    Ok(())
}

fn run_silver_network() -> Result<(), Box<dyn Error>> {
    println!("🚀 Running Silver Network ETL...");
    use polars::prelude::*;
    use riskfabric::etl::features::network::transform_network_features;
    use std::process::Command;

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
    let output = Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            query,
        ])
        .output()?;
    let df = ParquetReader::new(std::io::Cursor::new(output.stdout)).finish()?;

    let enriched_lf = transform_network_features(df.lazy());
    let mut enriched_df = enriched_lf.collect()?;

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            "
        CREATE TABLE IF NOT EXISTS network_features_silver (
            customer_id String,
            related_entity_id String,
            relationship_type String,
            shared_entity_fraud_rate Float64,
            degree UInt32,
            suspicious_cluster_member UInt32
        ) ENGINE = MergeTree() ORDER BY customer_id
    ",
        ])
        .status()?;

    let temp_path = "data/tmp_silver_network.parquet";
    let mut file = std::fs::File::create(temp_path)?;
    ParquetWriter::new(&mut file).finish(&mut enriched_df)?;

    Command::new("sh")
        .arg("-c")
        .arg(format!("cat {} | podman exec -i riskfabric_clickhouse clickhouse-client --database riskfabric --query \"INSERT INTO network_features_silver FORMAT Parquet\"", temp_path))
        .status()?;

    std::fs::remove_file(temp_path)?;
    println!("✨ Network features populated!");
    Ok(())
}

fn run_gold_master() -> Result<(), Box<dyn Error>> {
    println!("🚀 Running Gold Master ETL...");
    use std::process::Command;

    let join_query = "
        CREATE OR REPLACE TABLE fact_transactions_gold ENGINE = MergeTree() ORDER BY timestamp AS
        SELECT 
            t.transaction_id,
            t.timestamp,
            t.amount,
            t.merchant_category,
            t.transaction_channel,
            toUInt32(t.card_present) as card_present,
            toUInt32(t.is_fraud) as is_fraud,
            s.fraud_target,
            s.time_since_last_transaction,
            s.transaction_sequence_number,
            s.rapid_fire_transaction_flag,
            s.escalating_amounts_flag,
            s.merchant_category_switch_flag,
            s.geo_anomaly,
            s.device_anomaly,
            s.ip_anomaly,
            c.fraud_rate as cf_fraud_rate,
            c.night_transaction_ratio as cf_night_tx_ratio,
            m.merchant_fraud_rate as mf_fraud_rate,
            now() as feature_calculated_at
        FROM fact_transactions_bronze t
        LEFT JOIN fact_transactions_silver s ON t.transaction_id = s.transaction_id
        LEFT JOIN customer_features_silver c ON t.customer_id = c.customer_id
        LEFT JOIN merchant_features_silver m ON t.merchant_id = m.merchant_id
    ";

    Command::new("podman")
        .args([
            "exec",
            "riskfabric_clickhouse",
            "clickhouse-client",
            "--database",
            "riskfabric",
            "--query",
            join_query,
        ])
        .status()?;

    println!("✨ Gold master table populated!");
    Ok(())
}
