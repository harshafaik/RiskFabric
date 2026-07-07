# Data Warehouse Ingestor

## Overview

The ingestor (`ingest.rs`) is the data loading binary that populates the RiskFabric ClickHouse warehouse with raw synthetic output. It reads the Parquet files produced by `generate.rs` from the `data/output/` directory and creates the Bronze-layer fact and dimension tables required by the downstream `etl.rs` pipeline. The ingestor is fully idempotent — it drops and recreates all six warehouse tables on every run.

## Schema

### `fact_transactions_bronze_raw`

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `transaction_id` | `String` | UUID v4 identifying the transaction. |
| `card_id` | `String` | UUID v4 link to the associated card entity. |
| `account_id` | `String` | UUID v4 link to the associated account entity. |
| `customer_id` | `String` | UUID v4 link to the associated customer profile. |
| `merchant_id` | `String` | Unique identifier of the merchant. |
| `merchant_name` | `String` | Name of the merchant. |
| `merchant_category` | `String` | Category classification of the merchant. |
| `merchant_country` | `String` | Country where the merchant is registered. |
| `amount` | `Float64` | Transaction monetary value. |
| `currency` | `String` | Currency code (e.g., `"INR"`, `"USD"`). |
| `timestamp` | `String` | Raw ISO 8601 timestamp string as emitted by the generator. |
| `transaction_channel` | `String` | Channel used (e.g., `"POS"`, `"Online"`, `"ATM"`). |
| `card_present` | `UInt8` | Flag (0 or 1) indicating if the physical card was present. |
| `user_agent` | `String` | User Agent string recorded for the transaction. |
| `ip_address` | `String` | IP address of the client device. |
| `status` | `String` | Final state of the transaction (e.g., `"Approved"`, `"Declined"`). |
| `auth_status` | `String` | Authorization response code. |
| `failure_reason` | `Nullable(String)` | Reason for failure if the transaction was declined; null otherwise. |
| `is_fraud` | `UInt8` | Ground truth fraud label (0 or 1). |
| `chargeback` | `UInt8` | Flag (0 or 1) indicating if a chargeback was initiated. |
| `chargeback_days` | `Nullable(Int32)` | Days elapsed before a chargeback was filed; null if no chargeback. |
| `location_lat` | `Float64` | Latitude coordinate of the transaction. |
| `location_long` | `Float64` | Longitude coordinate of the transaction. |
| `h3_r7` | `String` | H3 index at Resolution 7 representing the transaction location. |

### `fact_transactions_bronze`

Identical column set to `fact_transactions_bronze_raw`, with one exception:

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `timestamp` | `DateTime64(3, 'UTC')` | Parsed and typed timestamp, converted from the raw string using `parseDateTime64BestEffort` with millisecond precision. All other fields are inherited without transformation. |

### `dim_customers`

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `customer_id` | `String` | UUID v4 identifying the customer. |
| `name` | `String` | Customer name. |
| `age` | `UInt32` | Customer age. |
| `email` | `String` | Synthetic email address. |
| `location` | `String` | Full residential address string. |
| `state` | `String` | State name. |
| `location_type` | `String` | Proximity classification (`"Metro"`, `"Urban"`, `"Rural"`). |
| `home_latitude` | `Float64` | Latitude of the customer's home coordinate. |
| `home_longitude` | `Float64` | Longitude of the customer's home coordinate. |
| `home_h3r5` | `String` | H3 index at Resolution 5 for the home location. |
| `home_h3r7` | `String` | H3 index at Resolution 7 for the home location. |
| `credit_score` | `UInt32` | Normalized credit score. |
| `monthly_spend` | `Float64` | Baseline monthly spend limit. |
| `customer_risk_score` | `Float64` | Probability score representing the customer's default risk level. |
| `is_fraud` | `UInt8` | Flag (0 or 1) indicating if this customer is simulated as compromised. |
| `registration_date` | `Date` | ISO 8601 date the customer registered. |
| `registration_year` | `UInt32` | Year portion of the registration date. |
| `registration_month` | `UInt32` | Month portion of the registration date. |
| `registration_day` | `UInt32` | Day portion of the registration date. |

### `dim_accounts`

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `account_id` | `String` | UUID v4 identifying the account. |
| `customer_id` | `String` | UUID v4 link to the parent customer. |
| `account_type` | `String` | Type of account (`"Savings"`, `"Current"`, or `"Credit"`). |
| `open_date` | `Date` | Date the account was opened. |
| `balance` | `Float64` | Starting account balance. |
| `status` | `String` | Current account status (e.g., `"Active"`). |

### `dim_cards`

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `card_id` | `String` | UUID v4 identifying the card. |
| `account_id` | `String` | UUID v4 link to the parent account. |
| `card_type` | `String` | Classification of the card (`"Debit"` or `"Credit"`). |
| `card_network` | `String` | Brand network (`"VISA"`, `"Mastercard"`, or `"RuPay"`). |
| `expiry_date` | `Date` | Date the card expires. |
| `status` | `String` | State of the card (`"Active"`, `"Expired"`, or `"Blocked"`). |

### `fact_fraud_metadata_bronze`

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `transaction_id` | `String` | UUID v4 linking back to the parent transaction in `fact_transactions_bronze`. |
| `fraud_target` | `UInt8` | Flag (0 or 1) indicating if the transaction was targeted for fraud injection. |
| `fraud_type` | `String` | Type classification of the injected fraud pattern. |
| `label_noise` | `String` | Noise indicator injected to simulate labelling errors. |
| `injector_version` | `String` | Version identifier of the fraud injection engine. |
| `geo_anomaly` | `UInt8` | Flag (0 or 1) indicating a geographic anomaly was injected. |
| `device_anomaly` | `UInt8` | Flag (0 or 1) indicating a device anomaly was injected. |
| `ip_anomaly` | `UInt8` | Flag (0 or 1) indicating an IP anomaly was injected. |
| `burst_session` | `UInt8` | Flag (0 or 1) indicating the transaction belongs to a high-velocity burst session. |
| `burst_seq` | `String` | Sequence index within the burst session. |
| `campaign_id` | `Nullable(String)` | UUID of the fraudulent campaign; null for non-campaign transactions. |
| `campaign_type` | `Nullable(String)` | Type of the fraud campaign; null for non-campaign transactions. |
| `campaign_phase` | `String` | Phase of the fraud campaign (e.g., `"testing"`, `"extraction"`). |
| `campaign_day_number` | `Int32` | Day sequence number within the campaign duration. |

**Two-Stage Timestamp Ingestion** is used for the transaction fact table. The raw Parquet file is first loaded into `fact_transactions_bronze_raw` with `timestamp` stored as a plain `String`, preserving whatever format the generator emitted without risking parse failures during bulk load. A second `CREATE TABLE AS SELECT` statement then applies `parseDateTime64BestEffort` to produce `fact_transactions_bronze` with a properly typed `DateTime64(3, 'UTC')` column. This staging approach ensures that no rows are silently dropped due to timestamp formatting mismatches.

**Idempotent Execution** is a core property of the ingestor. All six target tables are dropped unconditionally at startup before any data is loaded. This eliminates row duplication from re-runs and ensures the warehouse always reflects the current state of the `data/output/` Parquet files. There is no partial-update or upsert path — the ingestor is always a full rebuild.

**Shell-Piped Bulk Load** is used for all table inserts. Each Parquet file is piped via `cat {file} | podman exec -i riskfabric_clickhouse clickhouse-client ... INSERT INTO ... FORMAT Parquet`. This approach avoids staging the data in a temporary file inside the container, but ties the ingestor to the availability of both the host shell and the `podman` container runtime.

`ingest.rs` sits between the **File System layer** (Parquet output from `generate.rs`) and the **Warehouse layer** (ClickHouse). It is the mandatory prerequisite for `etl.rs`, which expects all six tables to be present and populated before any Silver or Gold stage is invoked.

## Known Issues

The warehouse schema defined in `ingest.rs` has drifted from the Rust model structs in `src/models/`. `dim_accounts` is missing `bank_id` and `account_no` relative to the `Account` struct. `dim_cards` retains only 6 of the 17 fields defined in `card.rs`, omitting `issue_date`, `activation_date`, `card_number`, `issuing_bank`, all usage limit fields, and `status_reason`. This means downstream ETL stages and ML features that attempt to join on card-level metadata operate on a truncated view of each instrument. Deriving ClickHouse DDL directly from the Rust structs, or adding a compile-time schema consistency check, is required to prevent continued drift.

All data is loaded via `sh -c "cat ... | podman exec -i ..."` shell pipelines rather than a native ClickHouse client. This creates a hard dependency on the host shell environment and the `podman` runtime being available and configured correctly. It also makes error handling coarse — any non-zero exit code from the shell pipeline is treated as a generic failure with no query-level context. Migrating to the ClickHouse HTTP interface or a native Rust client library would eliminate the shell dependency and enable row-level error reporting.
