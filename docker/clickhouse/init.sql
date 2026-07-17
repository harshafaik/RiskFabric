CREATE TABLE IF NOT EXISTS fraud_scores (
    transaction_id String,
    card_id String,
    customer_id String,
    amount Float64,
    timestamp DateTime,
    kafka_received_at DateTime64(3),
    fraud_probability Float64,
    flagged UInt8,
    scored_at DateTime64(3)
) ENGINE = MergeTree()
ORDER BY (timestamp, card_id);
