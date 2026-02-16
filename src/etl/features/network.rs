use polars::prelude::*;

pub fn transform_network_features(tx_lf: LazyFrame) -> LazyFrame {
    // Strategy: Instead of full N:N customer linkage (which OOMs), 
    // we calculate the risk reputation of the IP and Device entities themselves.
    
    // 1. IP Reputation
    let ip_risk = tx_lf.clone()
        .group_by([col("ip_address")])
        .agg([
            col("is_fraud").cast(DataType::UInt32).mean().alias("ip_fraud_rate"),
            col("customer_id").n_unique().alias("ip_customer_count"),
        ]);

    // 2. Device (User Agent) Reputation
    let dev_risk = tx_lf.clone()
        .group_by([col("user_agent")])
        .agg([
            col("is_fraud").cast(DataType::UInt32).mean().alias("dev_fraud_rate"),
            col("customer_id").n_unique().alias("dev_customer_count"),
        ]);

    // 3. Join reputations back to transactions to flag high-risk "Network Clusters"
    tx_lf
        .join(ip_risk, [col("ip_address")], [col("ip_address")], JoinType::Left.into())
        .join(dev_risk, [col("user_agent")], [col("user_agent")], JoinType::Left.into())
        .with_columns([
            // Flag as suspicious if the IP or Device has a high fraud rate (> 20%) 
            // and is shared by multiple customers
            (col("ip_fraud_rate").gt(lit(0.2)).and(col("ip_customer_count").gt(lit(1))))
                .or(col("dev_fraud_rate").gt(lit(0.2)).and(col("dev_customer_count").gt(lit(1))))
                .cast(DataType::UInt32)
                .alias("suspicious_cluster_member")
        ])
        .select([
            col("transaction_id"),
            col("customer_id"),
            col("ip_fraud_rate").alias("shared_entity_fraud_rate"), // Use IP rate as proxy
            col("ip_customer_count").alias("degree"), // Use IP sharing as proxy for degree
            col("suspicious_cluster_member"),
        ])
}
