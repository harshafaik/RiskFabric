use polars::prelude::*;

pub fn transform_network_features(tx_lf: LazyFrame) -> (LazyFrame, LazyFrame) {
    // 1. IP Reputation (Entity Level)
    let ip_risk = tx_lf.clone()
        .group_by([col("ip_address")])
        .agg([
            col("is_fraud").cast(DataType::Float64).mean().alias("ip_fraud_rate"),
            col("customer_id").n_unique().alias("ip_degree"),
        ]);

    // 2. Device (User Agent) Reputation (Entity Level)
    let dev_risk = tx_lf
        .group_by([col("user_agent")])
        .agg([
            col("is_fraud").cast(DataType::Float64).mean().alias("dev_fraud_rate"),
            col("customer_id").n_unique().alias("dev_degree"),
        ]);

    (ip_risk, dev_risk)
}
