# Knowledge Base: Issues & Resolutions

This document tracks technical hurdles encountered during the development of RiskFabric. It serves as a troubleshooting guide for engineers working with the Rust/Polars/ClickHouse stack.

## Quick Reference

| Component | Symptom | Resolution |
| :--- | :--- | :--- |
| **Polars** | `cannot create series from UInt8` | Migrate flags to `UInt32` |
| **Polars** | Kernel panic on `.is_in()` | Cast weekday to `Int32` |
| **ClickHouse**| Timestamp parsing failed | Store as String, parse in Silver ETL |
| **System** | Exit Code 137 (OOM) | Use Chunked Generation |
| **ML Model** | Perfect AUC (0.999) | Sanitize features (Leakage prevention) |
| **DLT** | ClickHouse Driver/SSL Woes | Deprecated in favor of Rust Native Ingestor |

---

## ⚙️ Data Engine & Type Safety

### Polars `UInt8` Series Creation Error
- **Issue**: Polars threw a `ComputeError` when materializing a DataFrame containing 8-bit unsigned integers.
- **Impact**: Blocked Silver ETL features like `is_weekend` and boolean flags.
- **Resolution**: Migrated all flag and counter columns to `DataType::UInt32` for native Polars support and better ML compatibility.

### Polars `is_in` Panic on `Int8`
- **Issue**: `.dt().weekday()` returns `Int8`, which caused a kernel-level panic during `.is_in()` checks.
- **Impact**: ETL runner crashes during temporal feature calculation.
- **Resolution**: Explicitly cast the output of `.weekday()` to `Int32` and ensure the comparison set matches exactly (e.g., `&[6i32, 7i32]`).

### ClickHouse Timestamp Precision
- **Issue**: Standard `DateTime64` ingestion failed for ISO 8601 strings with nanosecond precision.
- **Resolution**: Land raw timestamps as `String` in the Bronze layer. Use Polars' `.str().to_datetime()` during Silver ETL for flexible, high-precision parsing.

---

## 🚀 Performance & Scaling

### Out Of Memory (OOM) in Network Linkage
- **Issue**: Multi-million row many-to-many joins on IP/UA caused combinatorial explosion.
- **Impact**: OS terminated the process (Exit Code 137).
- **Resolution**: Shifted from an **Edge-List Graph approach** to an **Entity Reputation approach**. Calculate risk at the entity level and join back to transactions. Complexity reduced from $O(N^2)$ to $O(N)$.

### OOM in Large-Scale Generation
- **Issue**: Single-pass generation of 17M+ transactions exhausted RAM.
- **Resolution**: Refactored to a **Chunked One-Pass Architecture**. The generator processes the population in batches of 5,000 entities and flushes transactions to Parquet incrementally.

---

## 🧠 Machine Learning & Data Science

### Label Leakage (Near-Perfect AUC)
- **Issue**: Initial training yielded a near-perfect 0.9993 AUC.
- **Impact**: The model used hidden synthetic signals rather than behavioral patterns.
- **Resolution**: 
    1. **Feature sanitization**: Excluded `geo_anomaly`, `device_anomaly`, etc.
    2. **Target Shift**: Switched from `fraud_target` (Ground Truth) to `is_fraud` (Noisy Label).

### Observed vs. Configured Fraud Rate Discrepancy
- **Issue**: Observed fraud (~13.6%) appeared higher than the 12% configuration.
- **Resolution**: Verified that `is_fraud` deliberately incorporates simulated label noise (3% FP, 10% FN), resulting in the higher observed ratio.

### High Fraud Prevalence in Initial Runs
- **Issue**: ~86% of customers experienced fraud due to high default config values.
- **Resolution**: Tuned `target_share` to 0.005 (0.5% txn rate) to align with industry sparse-data benchmarks.

---

## 🔄 Data Ingestion & Orchestration

### DLT ClickHouse Ingestion Complexity
- **Issue**: Attempting to use `dlt` for Parquet-to-ClickHouse ingestion in a containerized environment led to a cascade of driver, network, and permission failures.
- **Root Causes**:
    1. **Driver Ambiguity**: DLT switches between `clickhouse-driver` (Native, port 9000) and `clickhouse-connect` (HTTP, port 8123). Misconfiguration often triggered automatic SSL upgrades to port 8443, causing `[SSL] record layer failure`.
    2. **Strict Type Casting**: Connection parameters in DLT (like `secure` and `http`) must be passed as integers (`0` or `1`) rather than booleans when using certain string formats, or they trigger `ValueError`.
    3. **Permission Wall**: DLT requires `SELECT` access to `INFORMATION_SCHEMA.COLUMNS` for schema synchronization. The default ClickHouse user is restricted to `localhost`, necessitating the creation of a remote user with broad grants.
- **Resolution**: Deprecated DLT for the ClickHouse sink to avoid orchestration bloat. Migrated to a **Native Rust Ingestor** (`src/bin/ingest.rs`) that utilizes `podman exec` and `clickhouse-client` pipes. This approach is faster, requires zero additional Python dependencies, and uses the database's native high-performance Parquet parser.
