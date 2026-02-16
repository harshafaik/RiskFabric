use polars::prelude::*;

pub fn create_gold_master_table(
    tx_bronze: LazyFrame,
    seq_features: LazyFrame,
    camp_features: LazyFrame,
    cust_features: LazyFrame,
    merch_features: LazyFrame,
    devip_features: LazyFrame,
    net_features: LazyFrame,
) -> LazyFrame {
    // 1. Prepare Network Aggregates (per customer)
    let net_agg = net_features.group_by([col("customer_id")])
        .agg([
            col("suspicious_cluster_member").max().alias("net_suspicious_cluster_member"),
            col("shared_entity_fraud_rate").mean().alias("net_avg_shared_entity_fraud_rate"),
        ]);

    // 2. Perform Master Joins
    tx_bronze
        .join(seq_features, [col("transaction_id")], [col("transaction_id")], JoinType::Left.into())
        .join(camp_features, [col("transaction_id")], [col("transaction_id")], JoinType::Left.into())
        .join(cust_features, [col("customer_id")], [col("customer_id")], JoinType::Left.into())
        .join(merch_features, [col("merchant_id")], [col("merchant_id")], JoinType::Left.into())
        .join(devip_features, [col("user_agent")], [col("user_agent")], JoinType::Left.into()) // user_agent as device proxy
        .join(net_agg, [col("customer_id")], [col("customer_id")], JoinType::Left.into())
        .with_columns([
            // Add current timestamps for lineage
            lit(chrono::Utc::now().to_rfc3339()).alias("feature_calculated_at")
        ])
}
