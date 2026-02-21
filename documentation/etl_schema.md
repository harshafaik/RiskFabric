# ETL & Feature Engineering Schema

RiskFabric follows a **Medallion Architecture** to transform raw synthetic data into ML-ready features. This document outlines the schema changes and features engineered at each layer.

---

## 🥈 Silver Layer (Feature Engineering)
The Silver layer consists of modular transformations that calculate behavioral, temporal, and entity-based risk signals.

### 1. Sequence Features (`sequence.rs`)
Calculates the relationship between a transaction and the customer's immediate history.

| Feature | Type | Description |
| :--- | :--- | :--- |
| `time_since_last_transaction` | Float64 | Seconds elapsed since previous txn (Windowed). |
| `transaction_sequence_number` | UInt64 | Monotonic counter of txns per customer. |
| `hours_since_midnight` | Float64 | Time of day decimal (0.0 to 23.99). |
| `is_weekend` | UInt32 | Boolean flag (1 = Sat/Sun). |
| `amount_round_number_flag` | UInt32 | Flag for amounts divisible by 1, 5, or 10. |
| `amount_deviation_z_score` | Float64 | Distance from user's mean amount in Std Devs. |
| `rapid_fire_transaction_flag` | UInt32 | Flag for txns occurring within 300s of each other. |
| `escalating_amounts_flag` | UInt32 | Flag if Amount(N) > Amount(N-1) > Amount(N-2). |
| `merchant_category_switch` | UInt32 | Flag if current category differs from the last. |

### 2. Entity Reputation (`merchant.rs` / `network.rs`)
Calculates risk scores based on shared entities (IPs, Devices, Merchants).

| Feature | Type | Description |
| :--- | :--- | :--- |
| `merchant_fraud_rate` | Float64 | Historical % of fraud at this specific merchant. |
| `ip_fraud_rate` | Float64 | % of fraud transactions originating from this IP. |
| `dev_fraud_rate` | Float64 | % of fraud transactions from this User Agent. |
| `ip_customer_count` | UInt32 | Count of distinct customers sharing this IP. |
| `suspicious_cluster_member`| UInt32 | Flag if IP/Dev has >20% fraud and >1 customer. |

### 3. Customer Baselines (`customer.rs`)
Aggregates long-term behavior for each customer.

| Feature | Type | Description |
| :--- | :--- | :--- |
| `total_transactions` | UInt32 | Total historical txn count. |
| `avg_transaction_amount` | Float64 | Mean amount across all txns. |
| `unique_merchants_count` | UInt32 | Count of distinct merchants visited. |
| `night_transaction_ratio` | Float64 | % of txns occurring between 10PM - 6AM. |
| `account_count` | UInt32 | Total number of accounts owned by the user. |

---

## 🥇 Gold Layer (The Master Table)
The Gold Master Table is a fully denormalized view created by joining the Bronze transactions with all Silver feature sets. This is the **Final Training Set**.

### Master Join Key Logic
1. **Transaction ID**: Joins Sequence and Campaign features.
2. **Customer ID**: Joins Customer Baselines and Network Aggregates.
3. **Merchant ID**: Joins Merchant Reputation.
4. **User Agent**: Joins Device/IP Reputation.

### Derived Metadata
| Field | Type | Description |
| :--- | :--- | :--- |
| `feature_calculated_at` | String | ISO 8601 timestamp of the ETL run. |
| `fraud_target` | UInt32 | **Label**: The ground-truth (hidden) fraud flag. |
| `is_fraud` | UInt32 | **Label**: The noisy (observed) fraud flag. |

---

## Data Flow Summary
1. **Bronze**: `transactions.parquet` (Raw).
2. **Transform**: Polars Lazy API performs windowing, group-bys, and joins.
3. **Silver**: Modular feature tables (parquet or in-memory).
4. **Gold**: `gold_master.parquet` (ML-ready).
