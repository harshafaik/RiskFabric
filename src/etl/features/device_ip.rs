use polars::prelude::*;

pub fn transform_device_ip_intelligence(tx_lf: LazyFrame) -> LazyFrame {
    // 1. Ensure device_id exists
    let tx = tx_lf.with_columns([
        col("is_fraud").cast(DataType::UInt32),
    ]);

    // 2. Device-level Aggregates
    let device_agg = tx
        .group_by([col("user_agent")]) // Using user_agent as proxy for device
        .agg([
            len().alias("transaction_count"),
            col("is_fraud").sum().alias("fraud_tx_count"),
            col("customer_id").n_unique().alias("unique_customers_per_device"),
            col("merchant_country").n_unique().alias("countries_accessed_from"),
        ])
        .with_columns([
            (col("fraud_tx_count").cast(DataType::Float64) / col("transaction_count").cast(DataType::Float64))
                .alias("device_fraud_rate"),
        ]);

    device_agg
}
