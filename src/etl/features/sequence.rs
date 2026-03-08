use polars::prelude::*;

pub fn transform_sequence_features(lf: LazyFrame, fraud_meta_lf: LazyFrame) -> LazyFrame {
    // 1. Prepare timestamp (already Datetime from ClickHouse)
    let lf = lf.with_column(
        col("timestamp").alias("ts_parsed")
    );

    // 2. Sort by customer and timestamp to ensure correct windowing
    let lf = lf.sort(["customer_id", "ts_parsed"], SortMultipleOptions::default());
    
    // 3. Join with Fraud Metadata early to have access to ground truth flags
    let lf = lf.join(
        fraud_meta_lf,
        [col("transaction_id")],
        [col("transaction_id")],
        JoinType::Left.into(),
    );

    // 4. Temporal Features
    let lf = lf.with_columns([
        // time_since_last_transaction
        // ClickHouse DateTime64(3) is in milliseconds
        (col("ts_parsed").cast(DataType::Int64) - col("ts_parsed").shift(lit(1)).over([col("customer_id")])
            .cast(DataType::Int64))
            .alias("time_since_last_transaction"),
        
        // transaction_sequence_number
        col("ts_parsed").cum_count(false).over([col("customer_id")]).alias("transaction_sequence_number"),
        
        // hours_since_midnight
        (col("ts_parsed").dt().hour().cast(DataType::Float64) + 
         col("ts_parsed").dt().minute().cast(DataType::Float64) / lit(60.0) + 
         col("ts_parsed").dt().second().cast(DataType::Float64) / lit(3600.0))
         .alias("hours_since_midnight"),
        
        // is_weekend (Polars ISO: 6=Sat, 7=Sun)
        col("ts_parsed").dt().weekday().cast(DataType::Int32)
            .is_in(lit(Series::new("wknd".into(), &[6i32, 7i32])), false)
            .cast(DataType::UInt32)
            .alias("is_weekend"),
    ]);

    // 5. Amount Patterns
    let lf = lf.with_columns([
        // amount_round_number_flag
        ((col("amount") % lit(1.0)).eq(lit(0.0))
            .or((col("amount") % lit(5.0)).eq(lit(0.0)))
            .or((col("amount") % lit(10.0)).eq(lit(0.0))))
            .cast(DataType::UInt32)
            .alias("amount_round_number_flag"),
        
        // Z-score per customer
        ((col("amount") - col("amount").mean().over([col("customer_id")])) / 
         col("amount").std(1).over([col("customer_id")]))
         .fill_nan(lit(0.0))
         .alias("amount_deviation_z_score"),
    ]);

    // 6. Sequential Risk (Rapid Fire & Escalation)
    let lf = lf.with_columns([
        // rapid_fire_transaction_flag (<= 300 seconds = 300,000 ms)
        col("time_since_last_transaction").is_not_null()
            .and(col("time_since_last_transaction").lt_eq(lit(300000)))
            .cast(DataType::UInt32)
            .alias("rapid_fire_transaction_flag"),
            
        // escalating_amounts_flag: prev2 < prev < current
        (col("amount").shift(lit(2)).over([col("customer_id")]).lt(col("amount").shift(lit(1)).over([col("customer_id")]))
            .and(col("amount").shift(lit(1)).over([col("customer_id")]).lt(col("amount"))))
            .cast(DataType::UInt32)
            .alias("escalating_amounts_flag"),
        
        // merchant_category_switch_flag
        col("merchant_category").shift(lit(1)).over([col("customer_id")]).is_not_null()
            .and(col("merchant_category").shift(lit(1)).over([col("customer_id")]).neq(col("merchant_category")))
            .cast(DataType::UInt32)
            .alias("merchant_category_switch_flag"),
    ]);

    lf.select([
        col("transaction_id"),
        col("customer_id"),
        col("ts_parsed").alias("timestamp"),
        (col("time_since_last_transaction").cast(DataType::Float64) / lit(1000.0)).alias("time_since_last_transaction"), // Convert ms to seconds
        col("transaction_sequence_number").cast(DataType::UInt64),
        lit(0u64).alias("same_day_transaction_count"), // Placeholder for now
        col("hours_since_midnight"),
        col("is_weekend"),
        lit(0u32).alias("is_holiday"), // Placeholder
        col("amount_round_number_flag"),
        col("rapid_fire_transaction_flag"),
        col("escalating_amounts_flag"),
        col("merchant_category_switch_flag"),
        lit(0u32).alias("is_foreign"), // Placeholder
        lit(0u32).alias("is_cross_border"), // Placeholder
        lit(0u32).alias("is_ip_mismatch"), // Placeholder
        col("fraud_target").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("fraud_type").fill_null(lit("none")),
        col("geo_anomaly").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("device_anomaly").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("ip_anomaly").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("campaign_id").fill_null(lit(NULL).cast(DataType::String)),
    ])
}
