use polars::prelude::*;

pub fn transform_fraud_campaign_features(tx_lf: LazyFrame) -> LazyFrame {
    let tx = tx_lf.filter(col("is_fraud").eq(lit(1u32))); // Adjusted for is_fraud as UInt32

    let fraud_with_gaps = tx.with_column(
        (col("timestamp").str().to_datetime(None, None, StrptimeOptions::default(), lit("null")).cast(DataType::Int64) - 
         col("timestamp").str().to_datetime(None, None, StrptimeOptions::default(), lit("null")).shift(lit(1)).over([col("customer_id")]).cast(DataType::Int64))
        .alias("gap")
    );

    let campaigns = fraud_with_gaps.with_column(
        col("gap").is_null().or(col("gap").gt(lit(172800))).cast(DataType::UInt32).cum_sum(false).over([col("customer_id")])
            .alias("campaign_seq")
    ).with_column(
        (col("customer_id") + lit("-") + col("campaign_seq").cast(DataType::String)).alias("campaign_id")
    );

    let campaign_agg = campaigns.clone().group_by([col("campaign_id")])
        .agg([
            len().alias("campaign_txn_count"),
            col("amount").sum().alias("campaign_total_amount"),
            col("merchant_id").n_unique().alias("campaign_merchant_diversity"),
        ]);

    campaigns.join(campaign_agg, [col("campaign_id")], [col("campaign_id")], JoinType::Left.into())
        .select([
            col("transaction_id"),
            col("campaign_id"),
            col("campaign_txn_count"),
            col("campaign_total_amount"),
            col("campaign_merchant_diversity"),
        ])
}
