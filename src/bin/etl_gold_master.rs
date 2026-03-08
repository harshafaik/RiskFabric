use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Gold ETL (SQL Mode): Master ML Table...");

    let db = "riskfabric";

    // 1. Create the Gold table via SQL Join in ClickHouse
    println!("   ... executing master join in ClickHouse");
    let join_query = "
        CREATE OR REPLACE TABLE fact_transactions_gold ENGINE = MergeTree() ORDER BY timestamp AS
        SELECT 
            t.transaction_id,
            t.timestamp,
            t.amount,
            t.merchant_category,
            t.transaction_channel,
            toUInt32(t.card_present) as card_present,
            toUInt32(t.is_fraud) as is_fraud,
            s.fraud_target,
            s.time_since_last_transaction,
            s.transaction_sequence_number,
            s.rapid_fire_transaction_flag,
            s.escalating_amounts_flag,
            s.merchant_category_switch_flag,
            s.geo_anomaly,
            s.device_anomaly,
            s.ip_anomaly,
            c.fraud_rate as cf_fraud_rate,
            c.night_transaction_ratio as cf_night_tx_ratio,
            m.merchant_fraud_rate as mf_fraud_rate,
            now() as feature_calculated_at
        FROM fact_transactions_bronze t
        LEFT JOIN fact_transactions_silver s ON t.transaction_id = s.transaction_id
        LEFT JOIN customer_features_silver c ON t.customer_id = c.customer_id
        LEFT JOIN merchant_features_silver m ON t.merchant_id = m.merchant_id
    ";

    let status = Command::new("podman")
        .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", join_query])
        .status()?;

    if status.success() {
        let count = Command::new("podman")
            .args(["exec", "riskfabric_clickhouse", "clickhouse-client", "--database", db, "--query", "SELECT count() FROM fact_transactions_gold"])
            .output()?;
        println!("✨ Successfully populated fact_transactions_gold! Rows: {}", String::from_utf8_lossy(&count.stdout).trim());
    } else {
        println!("❌ Failed to create Gold table.");
    }

    Ok(())
}
