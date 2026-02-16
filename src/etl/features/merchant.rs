use polars::prelude::*;

pub fn transform_merchant_features(tx_lf: LazyFrame) -> LazyFrame {
    // 1. Basic Merchant Aggregations
    tx_lf.group_by([col("merchant_id")])
        .agg([
            len().alias("total_transactions"),
            col("is_fraud").cast(DataType::UInt32).sum().alias("total_fraud_transactions"),
            col("customer_id").n_unique().alias("unique_customers_count"),
            col("amount").mean().alias("avg_transaction_amount"),
            col("amount").std(1).alias("std_transaction_amount"),
        ])
        .with_columns([
            // Merchant Fraud Rate
            (col("total_fraud_transactions").cast(DataType::Float64) / col("total_transactions").cast(DataType::Float64))
                .alias("merchant_fraud_rate"),
        ])
}

pub fn calculate_category_baselines(tx_lf: LazyFrame) -> LazyFrame {
    // 2. Global Baseline per Category for anomaly detection
    tx_lf.group_by([col("merchant_category")])
        .agg([
            col("amount").mean().alias("category_avg_amount"),
            col("is_fraud").cast(DataType::UInt32).mean().alias("category_fraud_baseline"),
        ])
}
