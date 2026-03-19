use polars::prelude::*;

pub fn transform_customer_features(
    tx_lf: LazyFrame, 
    cust_lf: LazyFrame,
    acc_lf: LazyFrame
) -> LazyFrame {
    // 1. Prepare Transactions
    let tx = tx_lf.with_columns([
        col("timestamp").alias("ts"),
        col("is_fraud").cast(DataType::UInt32),
    ]);

    // 2. Calculate Behavioral Flags
    let tx_enriched = tx.with_columns([
        (col("ts").dt().hour().gt_eq(lit(22)).or(col("ts").dt().hour().lt(lit(6))))
            .cast(DataType::UInt32).alias("is_night"),
        
        // Detect weekend (Saturday=6, Sunday=7)
        col("ts").dt().weekday().gt_eq(lit(6))
            .cast(DataType::UInt32).alias("is_weekend"),
    ]);

    // 3. Aggregate Features per Customer
    let tx_agg = tx_enriched.group_by([col("customer_id")])
        .agg([
            len().alias("total_transactions"),
            col("is_fraud").sum().alias("total_fraud_transactions"),
            col("amount").mean().alias("avg_transaction_amount"),
            col("amount").std(1).alias("std_transaction_amount"),
            col("merchant_id").n_unique().alias("unique_merchants_count"),
            col("merchant_category").n_unique().alias("unique_merchant_categories_count"),
            col("merchant_country").n_unique().alias("unique_countries_count"),
            col("is_night").sum().alias("night_tx_count"),
            col("is_weekend").sum().alias("weekend_tx_count"),
            col("ts").min().alias("first_transaction_ts"),
            col("ts").max().alias("last_transaction_ts"),
        ])
        .with_columns([
            (col("total_fraud_transactions").cast(DataType::Float64) / col("total_transactions").cast(DataType::Float64))
                .alias("fraud_rate"),
            (col("night_tx_count").cast(DataType::Float64) / col("total_transactions").cast(DataType::Float64))
                .alias("night_transaction_ratio"),
            (col("weekend_tx_count").cast(DataType::Float64) / col("total_transactions").cast(DataType::Float64))
                .alias("weekend_transaction_ratio"),
        ]);

    let acc_count = acc_lf.group_by([col("customer_id")])
        .agg([len().alias("account_count")]);

    cust_lf
        .join(acc_count, [col("customer_id")], [col("customer_id")], JoinType::Left.into())
        .join(tx_agg, [col("customer_id")], [col("customer_id")], JoinType::Left.into())
        .with_columns([
            col("fraud_rate").fill_null(lit(0.0)),
            col("total_transactions").fill_null(lit(0)),
            col("account_count").fill_null(lit(0)),
        ])
}
