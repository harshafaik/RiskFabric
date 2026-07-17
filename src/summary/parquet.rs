use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use polars::prelude::*;

#[derive(Debug, Clone)]
pub struct DatasetSummary {
    pub total_transactions: u64,
    pub total_customers: u64,
    pub total_accounts: u64,
    pub total_cards: u64,
    pub fraud_transactions: u64,
    pub fraud_rate: f64,
    pub legitimate_transactions: u64,
    pub has_ground_truth: bool,
    pub account_type_dist: HashMap<String, u64>,
    pub channel_dist: HashMap<String, u64>,
    pub category_dist: Vec<(String, u64)>,
    pub amount_p25: f64,
    pub amount_p50: f64,
    pub amount_p75: f64,
    pub amount_p99: f64,
}

pub fn summarize_parquet_dir(dir: &str) -> anyhow::Result<DatasetSummary> {
    let base = PathBuf::from(dir);
    let gt_path = base.join("fraud_metadata.parquet");
    let has_ground_truth = gt_path.exists();

    let tx_path = base.join("transactions.parquet");
    if !tx_path.exists() {
        return Ok(empty_summary(has_ground_truth));
    }

    let mut file = File::open(&tx_path)?;
    let df = ParquetReader::new(&mut file).finish()?;

    let height = df.height() as u64;
    if height == 0 {
        return Ok(empty_summary(has_ground_truth));
    }

    let fraud_col = df.column("is_fraud")?.bool()?;
    let fraud_transactions = (0..df.height()).filter(|&i| fraud_col.get(i) == Some(true)).count() as u64;

    let legitimate_transactions = height.saturating_sub(fraud_transactions);
    let fraud_rate = if height > 0 { fraud_transactions as f64 / height as f64 } else { 0.0 };

    let channel_map = if let Ok(col) = df.column("transaction_channel") {
        count_string_column(col.str()?)
    } else { HashMap::new() };

    let category_dist = if let Ok(col) = df.column("merchant_category") {
        let mut map = count_string_column(col.str()?);
        let mut vec: Vec<(String, u64)> = map.drain().collect();
        vec.sort_by(|a, b| b.1.cmp(&a.1));
        vec.truncate(20);
        vec
    } else { Vec::new() };

    // Amount quartiles
    let amount_col = df.column("amount")?.f64()?;
    let mut amounts: Vec<f64> = (0..df.height()).filter_map(|i| amount_col.get(i)).collect();
    amounts.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = amounts.len();
    let p25 = if n > 0 { amounts[n / 4] } else { 0.0 };
    let p50 = if n > 0 { amounts[n / 2] } else { 0.0 };
    let p75 = if n > 0 { amounts[n * 3 / 4] } else { 0.0 };
    let p99 = if n > 0 { amounts[n * 99 / 100] } else { 0.0 };

    let total_customers = count_df_rows(&base.join("customers.parquet"))?;
    let total_accounts = count_df_rows(&base.join("accounts.parquet"))?;
    let total_cards = count_df_rows(&base.join("cards.parquet"))?;

    let account_type_dist = if let Ok(mut f) = File::open(base.join("accounts.parquet")) {
        let acct_df = ParquetReader::new(&mut f).finish()?;
        if let Ok(col) = acct_df.column("account_type") {
            count_string_column(col.str()?)
        } else { HashMap::new() }
    } else { HashMap::new() };

    Ok(DatasetSummary {
        total_transactions: height,
        total_customers,
        total_accounts,
        total_cards,
        fraud_transactions,
        fraud_rate,
        legitimate_transactions,
        has_ground_truth,
        account_type_dist,
        channel_dist: channel_map,
        category_dist,
        amount_p25: p25,
        amount_p50: p50,
        amount_p75: p75,
        amount_p99: p99,
    })
}

fn count_string_column(col: &polars::prelude::StringChunked) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for s in col.into_no_null_iter() {
        *map.entry(s.to_string()).or_default() += 1;
    }
    map
}

fn count_df_rows(path: &PathBuf) -> anyhow::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut file = File::open(path)?;
    let df = ParquetReader::new(&mut file).finish()?;
    Ok(df.height() as u64)
}

fn empty_summary(has_ground_truth: bool) -> DatasetSummary {
    DatasetSummary {
        total_transactions: 0,
        total_customers: 0,
        total_accounts: 0,
        total_cards: 0,
        fraud_transactions: 0,
        fraud_rate: 0.0,
        legitimate_transactions: 0,
        has_ground_truth,
        account_type_dist: HashMap::new(),
        channel_dist: HashMap::new(),
        category_dist: Vec::new(),
        amount_p25: 0.0,
        amount_p50: 0.0,
        amount_p75: 0.0,
        amount_p99: 0.0,
    }
}
