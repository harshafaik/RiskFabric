# Transaction Generator

## Overview
The transaction generator module `transaction_gen.rs` is responsible for simulating the financial lifecycle of every card in the system over a specified lookback period (default 365 days). 

## Schema

Each transaction execution pass generates a comprehensive transaction record and matching fraud diagnostics metadata:

### `Transaction`
| Field Name | Type | Description |
| :--- | :--- | :--- |
| `transaction_id` | `String` | Unique UUID v4 identifying the transaction. |
| `card_id` | `String` | UUID v4 link to the associated card entity. |
| `account_id` | `String` | UUID v4 link to the associated account entity. |
| `customer_id` | `String` | UUID v4 link to the associated customer profile. |
| `merchant_id` | `String` | Unique identifier of the merchant. |
| `merchant_name` | `String` | Name of the merchant. |
| `merchant_category` | `String` | Category classification (e.g., `"Retail"`, `"Travel"`, `"Food"`). |
| `mcc` | `String` | Merchant Category Code (4-digit string). |
| `merchant_country` | `String` | Default country where the merchant is registered (derived from configuration rules). |
| `amount` | `f64` | Transaction monetary value. |
| `currency` | `String` | Base currency code (e.g., `"INR"`, `"USD"`). |
| `timestamp` | `String` | ISO 8601 UTC date-time string (`"YYYY-MM-DDTHH:MM:SSZ"`). |
| `transaction_channel` | `String` | Channel used for the transaction (e.g., `"POS"`, `"Online"`, `"ATM"`). |
| `card_present` | `bool` | Flag designating if the physical card was present at the transaction. |
| `user_agent` | `String` | User Agent string recorded for online/app transactions. |
| `ip_address` | `String` | IP address of the client device. |
| `status` | `String` | Final state of the transaction (e.g., `"Approved"`, `"Declined"`). |
| `auth_status` | `String` | Authorization response code or message. |
| `failure_reason` | `Option<String>` | Reason for failure if the transaction was declined. |
| `is_fraud` | `bool` | Ground truth label indicating whether this transaction is simulated as fraudulent. |
| `chargeback` | `bool` | Flag designating if a chargeback was initiated for this transaction. |
| `chargeback_days` | `Option<i32>` | Days elapsed before a chargeback was filed. |
| `location_lat` | `f64` | Latitude coordinate of the transaction. |
| `location_long` | `f64` | Longitude coordinate of the transaction. |
| `h3_r7` | `String` | H3 Index at Resolution 7 representing the transaction location. |

### `FraudMetadata`
| Field Name | Type | Description |
| :--- | :--- | :--- |
| `transaction_id` | `String` | Unique UUID v4 linking back to the transaction. |
| `fraud_target` | `bool` | Flag designating if the transaction was targeted for fraud injection. |
| `fraud_type` | `String` | Type classification of the injected fraud (e.g., `"Simulated Compromise"`, `"Carding"`, etc.). |
| `label_noise` | `String` | Noise indicator injected to simulate labelling errors. |
| `injector_version` | `String` | Version identifier of the fraud injection engine. |
| `geo_anomaly` | `bool` | True if the transaction location deviates stochastically from customer behavior patterns. |
| `device_anomaly` | `bool` | True if the user agent or device parameters deviate from profile norms. |
| `ip_anomaly` | `bool` | True if the transaction IP address is anomalous (e.g., proxy/VPN). |
| `flags` | `Option<Vec<String>>` | Warning list flags generated during mutation. |
| `burst_session` | `bool` | True if the transaction belongs to a high-velocity burst session. |
| `burst_seq` | `Option<i32>` | Sequence index of the transaction within a burst session. |
| `campaign_id` | `Option<String>` | Unique ID of the fraudulent campaign. |
| `campaign_type` | `Option<String>` | Type/Strategy of the campaign. |
| `campaign_phase` | `Option<String>` | Phase of the multi-step fraud campaign (e.g., testing, extraction). |
| `campaign_day_number` | `Option<i32>` | Day sequence number within the campaign duration. |

This module uses `rayon` to iterate over cards, all logic—including merchant selection, timestamp generation, amount calculation, and fraud injection, occuring within a single parallelized loop. This eliminates the need for multi-pass joins, thereby improving performance.

For spatial realism, merchants are selected based on a probabilistic proximity model: 80% are "super-local" (Res 6), 15% are "district-level" (Res 4), 3% are "state-level," and 2% are "global." This creates realistic spending clusters around a customer's home while allowing for occasional travel or remote spending.

**Deterministic Seeding** is used at the card level to ensure reproducibility.

`transaction_gen.rs` is the central module consumed by both the Batch Generator (`generate.rs`) and the Streaming Generator (`stream.rs`),  consuming configuration, spatial reference structures, and entity mappings as inputs, producing vectors of `Transaction` and `FraudMetadata` structures. These outputs are serialized to parquet datasets or piped directly into streams.

## Known Issues
Timestamp generation is implemented by sorting a local vector of dates for each card. While this ensures that transactions are chronologically ordered *per card*, it does not guarantee a global chronological order across the entire dataset during batch generation. ClickHouse is currently used to perform the final global sort. 

Additionally, the spatial distribution weights (80/15/3/2) are hardcoded directly into the logic. Moving these to `transaction_config.yaml` would allow users to simulate different mobility profiles—for example, a "commuter" population would require a higher Res 4 weight compared to a "rural" population.
