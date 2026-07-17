use clap::{Parser, Subcommand};
use polars::prelude::*;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

const BRONZE_DIR: &str = "data/bronze";
const SILVER_DIR: &str = "data/silver";
const GOLD_DIR: &str = "data/gold";

type EResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(name = "riskfabric-etl")]
#[command(about = "Parquet-native ETL pipeline for RiskFabric", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    SilverAll,
    SilverCustomer,
    SilverMerchant,
    SilverSequence,
    SilverCampaign,
    GoldMaster,
}

fn main() -> EResult {
    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> EResult {
    match cli.command {
        Commands::SilverAll => {
            rayon::scope(|s| {
                s.spawn(|_| run_silver_customer().unwrap());
                s.spawn(|_| run_silver_merchant().unwrap());
                s.spawn(|_| run_silver_sequence().unwrap());
            });
            println!("✅ Silver stages completed in parallel.");
            Ok(())
        }
        Commands::SilverCustomer => run_silver_customer(),
        Commands::SilverMerchant => run_silver_merchant(),
        Commands::SilverSequence => run_silver_sequence(),
        Commands::SilverCampaign => {
            println!("⚠️ Campaign features have unresolved signal reliability issues and are disabled.");
            Ok(())
        },
        Commands::GoldMaster => run_gold_master(),
    }
}

fn ensure_dir(path: &str) -> EResult {
    fs::create_dir_all(path)?;
    Ok(())
}

fn bronze_path(file: &str) -> String {
    format!("{}/{}", BRONZE_DIR, file)
}

fn silver_path(file: &str) -> String {
    format!("{}/{}", SILVER_DIR, file)
}

fn local_pl_path(s: &str) -> PlPath {
    PlPath::Local(Arc::from(Path::new(s)))
}

fn scan_bronze(file: &str) -> EResult<LazyFrame> {
    let path = bronze_path(file);
    if !Path::new(&path).exists() {
        return Err(format!("Bronze file not found: {}. Run generate.rs first and copy data/output/*.parquet to data/bronze/.", path).into());
    }
    let args = ScanArgsParquet::default();
    Ok(LazyFrame::scan_parquet(local_pl_path(&path), args).map_err(|e| format!("Polars scan error: {}", e))?)
}

fn scan_silver(file: &str) -> EResult<LazyFrame> {
    let path = silver_path(file);
    if !Path::new(&path).exists() {
        return Err(format!("Silver file not found: {}. Run silver-all first.", path).into());
    }
    let args = ScanArgsParquet::default();
    Ok(LazyFrame::scan_parquet(local_pl_path(&path), args).map_err(|e| format!("Polars scan error: {}", e))?)
}

fn write_parquet(df: &mut DataFrame, path: &str) -> EResult {
    let mut file = fs::File::create(path)?;
    ParquetWriter::new(&mut file).finish(df)?;
    Ok(())
}

fn run_silver_customer() -> EResult {
    println!("🚀 Running Silver Customer ETL...");
    use riskfabric::etl::features::customer::transform_customer_features;

    let tx_lf = scan_bronze("transactions.parquet")?;
    let cust_lf = scan_bronze("customers.parquet")?;
    let acc_lf = scan_bronze("accounts.parquet")?;

    let mut result = transform_customer_features(tx_lf, cust_lf, acc_lf).collect()?;

    ensure_dir(SILVER_DIR)?;
    let out_path = silver_path("customer_features.parquet");
    write_parquet(&mut result, &out_path)?;

    println!("✨ Customer features written to {}", out_path);
    println!("   {} rows, {} columns", result.height(), result.width());
    Ok(())
}

fn run_silver_merchant() -> EResult {
    println!("🚀 Running Silver Merchant ETL...");
    use riskfabric::etl::features::merchant::transform_merchant_features;

    let tx_lf = scan_bronze("transactions.parquet")?;

    let mut result = transform_merchant_features(tx_lf).collect()?;

    ensure_dir(SILVER_DIR)?;
    let out_path = silver_path("merchant_features.parquet");
    write_parquet(&mut result, &out_path)?;

    println!("✨ Merchant features written to {}", out_path);
    println!("   {} rows, {} columns", result.height(), result.width());
    Ok(())
}

fn run_silver_sequence() -> EResult {
    println!("🚀 Running Silver Sequence ETL...");
    use riskfabric::etl::features::sequence::transform_sequence_features;

    let tx_lf = scan_bronze("transactions.parquet")?;
    let meta_lf = scan_bronze("fraud_metadata.parquet")?;

    let mut result = transform_sequence_features(tx_lf, meta_lf).collect()?;

    ensure_dir(SILVER_DIR)?;
    let out_path = silver_path("transaction_features.parquet");
    write_parquet(&mut result, &out_path)?;

    println!("✨ Transaction features written to {}", out_path);
    println!("   {} rows, {} columns", result.height(), result.width());
    Ok(())
}

fn run_gold_master() -> EResult {
    println!("🚀 Running Gold Master ETL...");

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let gold_dir = format!("{}/{}", GOLD_DIR, ts);
    ensure_dir(&gold_dir)?;

    let tx_lf = scan_silver("transaction_features.parquet")?;

    let cust_exists = Path::new(&silver_path("customer_features.parquet")).exists();
    let merch_exists = Path::new(&silver_path("merchant_features.parquet")).exists();

    let gold = if cust_exists && merch_exists {
        let cust_lf = scan_silver("customer_features.parquet")?
            .select([col("customer_id"), col("fraud_rate").alias("cf_fraud_rate"), col("night_transaction_ratio").alias("cf_night_tx_ratio")]);
        let merch_lf = scan_silver("merchant_features.parquet")?
            .select([col("merchant_id"), col("merchant_fraud_rate").alias("mf_fraud_rate")]);

        tx_lf
            .join(cust_lf, [col("customer_id")], [col("customer_id")], JoinType::Left.into())
            .join(merch_lf, [col("merchant_id")], [col("merchant_id")], JoinType::Left.into())
            .with_columns([
                lit(0u32).alias("campaign_txn_count"),
                lit(0.0f64).alias("campaign_total_amount"),
                lit(0u32).alias("campaign_merchant_diversity"),
                col("cf_fraud_rate").fill_null(lit(0.0)),
                col("cf_night_tx_ratio").fill_null(lit(0.0)),
                col("mf_fraud_rate").fill_null(lit(0.0)),
            ])
            .collect()?
    } else {
        if !cust_exists {
            println!("   ⚠️ customer_features.parquet not found, zeroing entity features");
        }
        if !merch_exists {
            println!("   ⚠️ merchant_features.parquet not found, zeroing entity features");
        }
        tx_lf
            .with_columns([
                lit(0.0f64).alias("cf_fraud_rate"),
                lit(0.0f64).alias("cf_night_tx_ratio"),
                lit(0.0f64).alias("mf_fraud_rate"),
                lit(0u32).alias("campaign_txn_count"),
                lit(0.0f64).alias("campaign_total_amount"),
                lit(0u32).alias("campaign_merchant_diversity"),
            ])
            .collect()?
    };

    let mut gold_mut = gold.clone();
    let out_path = format!("{}/fact_transactions_gold.parquet", gold_dir);
    write_parquet(&mut gold_mut, &out_path)?;

    let fraud_count = gold.column("is_fraud")?
        .u32()?
        .into_no_null_iter()
        .filter(|&v| v > 0)
        .count();

    println!("✨ Gold master written to {}", out_path);
    println!("   {} rows, {} columns", gold.height(), gold.width());
    println!("   Fraud rows: {}", fraud_count);
    Ok(())
}
