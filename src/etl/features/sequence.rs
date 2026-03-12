use polars::prelude::*;

pub fn transform_sequence_features(lf: LazyFrame, fraud_meta_lf: LazyFrame) -> LazyFrame {
    // 1. Prepare timestamp and initial sort
    let lf = lf
        .with_column(col("timestamp").cast(DataType::Int64).alias("ts_physical"))
        .sort(
            ["customer_id", "ts_physical"],
            SortMultipleOptions {
                maintain_order: true,
                ..Default::default()
            },
        );

    // 2. Join with Fraud Metadata
    let lf = lf.join(
        fraud_meta_lf,
        [col("transaction_id")],
        [col("transaction_id")],
        JoinType::Left.into(),
    );

    // 3. Block 1: Independent Window and Temporal Features
    let lf = lf.with_columns([
        // time_since_last_transaction
        (col("ts_physical") - col("ts_physical").shift(lit(1)).over([col("customer_id")]))
            .alias("time_since_last_transaction"),
        // transaction_sequence_number
        col("ts_physical")
            .cum_count(false)
            .over([col("customer_id")])
            .alias("transaction_sequence_number"),
        // hours_since_midnight
        (col("timestamp").dt().hour().cast(DataType::Float64)
            + col("timestamp").dt().minute().cast(DataType::Float64) / lit(60.0)
            + col("timestamp").dt().second().cast(DataType::Float64) / lit(3600.0))
        .alias("hours_since_midnight"),
        // transaction_hour (integer for deviation calc)
        col("timestamp").dt().hour().cast(DataType::Float64).alias("txn_hour"),
        // is_weekend
        col("timestamp")
            .dt()
            .weekday()
            .cast(DataType::Int32)
            .is_in(lit(Series::new("wknd".into(), &[6i32, 7i32])), false)
            .cast(DataType::UInt32)
            .alias("is_weekend"),
        // Prev Lat/Lon for Velocity
        col("location_lat").shift(lit(1)).over([col("customer_id")]).alias("prev_lat"),
        col("location_long").shift(lit(1)).over([col("customer_id")]).alias("prev_lon"),
        // Shifted Merchant Category
        col("merchant_category")
            .shift(lit(1))
            .over([col("customer_id")])
            .alias("prev_merchant_category"),
        // Shifted Amounts for Escalation
        col("amount").shift(lit(1)).over([col("customer_id")]).alias("prev_amount"),
        col("amount").shift(lit(2)).over([col("customer_id")]).alias("prev2_amount"),
    ]);

    // 4. Block 2: Dependent Features (Spatial Velocity, Deviations, Patterns)
    let lf = lf.with_columns([
        // Spatial Distance
        (((col("location_lat") - col("prev_lat")).pow(2.0) + (col("location_long") - col("prev_lon")).pow(2.0)).sqrt() * lit(111.0))
            .alias("distance_km"),
        // Hour Deviation
        ((col("txn_hour") - col("txn_hour").mean().over([col("customer_id")])).pow(2.0).sqrt())
            .alias("hour_deviation_from_norm"),
        // Amount Round Number
        ((col("amount") % lit(1.0)).eq(lit(0.0))
            .or((col("amount") % lit(5.0)).eq(lit(0.0)))
            .or((col("amount") % lit(10.0)).eq(lit(0.0))))
            .cast(DataType::UInt32)
            .alias("amount_round_number_flag"),
        // Z-score
        ((col("amount") - col("amount").mean().over([col("customer_id")]))
            / col("amount").std(1).over([col("customer_id")]))
            .fill_nan(lit(0.0))
            .alias("amount_deviation_z_score"),
        // Rapid Fire
        col("time_since_last_transaction").is_not_null()
            .and(col("time_since_last_transaction").lt_eq(lit(300_000)))
            .cast(DataType::UInt32)
            .alias("rapid_fire_transaction_flag"),
        // Escalation
        (col("prev2_amount").lt(col("prev_amount")).and(col("prev_amount").lt(col("amount"))))
            .cast(DataType::UInt32)
            .alias("escalating_amounts_flag"),
        // Category Switch
        col("prev_merchant_category").is_not_null()
            .and(col("prev_merchant_category").neq(col("merchant_category")))
            .cast(DataType::UInt32)
            .alias("merchant_category_switch_flag"),
    ]);

    // 5. Block 3: Capping and Cleanup
    let lf = lf.with_columns([
        // Final capped spatial velocity
        when((col("distance_km") / (col("time_since_last_transaction").cast(DataType::Float64) / lit(3_600_000.0)))
            .is_infinite().or((col("distance_km") / (col("time_since_last_transaction").cast(DataType::Float64) / lit(3_600_000.0))).gt(lit(10000.0))))
            .then(lit(10000.0))
            .otherwise((col("distance_km") / (col("time_since_last_transaction").cast(DataType::Float64) / lit(3_600_000.0))).fill_null(lit(0.0)))
            .fill_nan(lit(0.0))
            .alias("spatial_velocity")
    ]);

    // 6. Final Select
    lf.select([
        col("transaction_id"),
        col("customer_id"),
        col("timestamp"),
        (col("time_since_last_transaction").cast(DataType::Float64) / lit(1000.0))
            .alias("time_since_last_transaction"),
        col("transaction_sequence_number").cast(DataType::UInt64),
        lit(0u64).alias("same_day_transaction_count"),
        col("hours_since_midnight"),
        col("is_weekend"),
        lit(0u32).alias("is_holiday"),
        col("spatial_velocity"),
        col("hour_deviation_from_norm"),
        col("amount_round_number_flag"),
        col("amount_deviation_z_score"),
        col("rapid_fire_transaction_flag"),
        col("escalating_amounts_flag"),
        col("merchant_category_switch_flag"),
        lit(0u32).alias("is_foreign"),
        lit(0u32).alias("is_cross_border"),
        lit(0u32).alias("is_ip_mismatch"),
        col("fraud_target").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("fraud_type").fill_null(lit("none")),
        col("geo_anomaly").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("device_anomaly").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("ip_anomaly").fill_null(lit(0u32)).cast(DataType::UInt32),
        col("campaign_id").fill_null(lit(NULL).cast(DataType::String)),
    ])
}
