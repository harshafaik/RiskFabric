# Technical Issues & Resolutions

This document tracks significant technical hurdles encountered during the development of `riskfabric`, specifically regarding the Rust/Polars/ClickHouse stack.

## 1. Polars 0.51.0 `UInt8` Series Creation Error
### Issue
During the `lf.collect()` phase, Polars threw a `ComputeError(ErrString("cannot create series from UInt8"))`. This occurred when trying to materialize a DataFrame containing 8-bit unsigned integer columns, either imported from ClickHouse or cast within Rust.

### Impact
Blocked the Silver ETL pipeline from materializing features like `is_weekend`, `rapid_fire_flag`, and other boolean-style indicators.

### Resolution
Migrated all flag and counter columns to `DataType::UInt32`. 32-bit integers are natively supported by the Polars `Series` factory and offer better compatibility with downstream Machine Learning libraries (XGBoost, CatBoost).

## 2. Polars `is_in` Panic on `Int8` Types
### Issue
The `.dt().weekday()` function in Polars 0.51.0 returns an `Int8` series. Executing `.is_in()` on this series caused a kernel-level panic: `not implemented for dtype Int8`.

### Impact
Caused the ETL runner to crash when calculating temporal features.

### Resolution
Explicitly cast the output of `.weekday()` to `Int32` before calling `.is_in()`. Additionally, ensure the literal comparison set (e.g., `&[6i32, 7i32]`) matches the target type exactly.

## 3. ClickHouse Best-Effort Timestamp Parsing
### Issue
Standard `DateTime64` ingestion in ClickHouse failed for ISO 8601 strings with high precision (nanoseconds) and timezone offsets (e.g., `2025-03-13T09:46:20.960868686+00:00`).

### Resolution
Stored raw timestamps as `String` in the Bronze layer (`fact_transactions_bronze`). Utilized Polars' robust `.str().to_datetime()` during the Silver ETL phase to handle high-precision parsing, which proved more flexible than native ClickHouse casting for this specific synthetic format.

## 4. High Fraud Prevalence in Synthetic Population
### Issue
During dry runs, ~86% of the customer population experienced at least one fraud event. This was caused by high default values for `target_share` (0.12) and `target_campaign_share` (0.15) in `fraud_rules.yaml`.

### Impact
The dataset became unrealistic for ML training, as the "clean" customer baseline was too small (sparsity of fraud was lost).

### Resolution
Tuned the configuration to industry-standard benchmarks: `target_share` reduced to 0.005 (0.5% txn rate) and `target_campaign_share` reduced to 0.01 (1% customer attack rate).

## 5. Out Of Memory (OOM) in Network Linkage
### Issue
The initial implementation of `etl_silver_network` attempted a full many-to-many join on `ip_address` and `user_agent` to identify all pairs of customers sharing entities. High-cardinality entities (e.g., common Public IPs or User Agents) caused a combinatorial explosion, attempting to materialize millions of rows in RAM.

### Impact
The process was terminated by the OS (Exit Code 137) during the `.collect()` phase, even with a relatively small 180k row dataset.

### Resolution
    Shifted from an **Edge-List Graph approach** to an **Entity Reputation approach**. Instead of joining customers to customers, the logic now calculates fraud rates and customer counts for each IP/Device and joins these "Reputations" back to the transactions. This achieved the same risk signal with $O(N)$ memory complexity instead of $O(N^2)$.

## 6. Duplicate Records in Silver Layer
### Issue
The `fact_transactions_silver` table contained double the expected number of records (358,830 vs 179,415). This was caused by the `etl_silver_sequence` process appending data to a `MergeTree` table without truncating it first, leading to duplicates on subsequent runs.

### Impact
Skewed downstream analytics and ML training data, potentially doubling the weight of certain transactions and misrepresenting fraud ratios.

### Resolution
Added an explicit `TRUNCATE TABLE fact_transactions_silver` command to the `src/bin/etl_silver_sequence.rs` binary before the data insertion phase.

## 7. Observed vs. Configured Fraud Rate Discrepancy
### Issue
The observed fraud rate in the Bronze layer (~13.6%) appeared higher than the `target_share` configured in `fraud_rules.yaml` (0.12).

### Impact
Initial confusion regarding the accuracy of the fraud injection logic.

### Resolution
Investigation of `src/generators/fraud.rs` revealed that the `is_fraud` label includes deliberate noise. While the `fraud_target` (ground truth) aligns with the 12% `target_share`, the `is_fraud` label incorporates a 3% False Positive rate and a 10% False Negative rate, resulting in the ~13.6% observed label frequency. This is a desired feature to simulate "noisy" real-world labels.

