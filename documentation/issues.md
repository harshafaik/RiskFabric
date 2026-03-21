# Technical Issues & Resolutions

## Summary
The `issues.md` document acts as the primary engineering log for RiskFabric. It captures architectural hurdles, environment-specific bugs, and performance bottlenecks encountered during development, along with their implemented or proposed resolutions.

## Design Intent
This document serves as **Institutional Knowledge** for the project. In complex simulations, the most difficult bugs often arise from the interaction between system layers (e.g., Rust → Kafka → Python). Documenting these issues provides a roadmap for future optimizations and prevents the repetition of architectural errors. Every entry is paired with a specific technical fix validated through benchmarking or regression testing.

---

## 🛠️ Data Engine & Type Safety

### 1. Polars UInt8 Series Creation Error
- **Problem**: Polars returned a `ComputeError` when materializing DataFrames containing 8-bit unsigned integers. This blocked features like `is_weekend` and other boolean-adjacent flags.
- **Resolution**: All flag and counter columns were migrated to `DataType::UInt32` to ensure native Polars support and broader ML library compatibility.

### 2. Polars `is_in` Panic on Int8
- **Problem**: The `.dt().weekday()` function returns `Int8`, which caused kernel-level panics during `.is_in()` membership checks.
- **Resolution**: Output from `.weekday()` is now explicitly cast to `Int32`, ensuring the comparison set (e.g., `&[6i32, 7i32]`) matches the target type exactly.

### 3. ClickHouse Timestamp Precision
- **Problem**: Standard `DateTime64` ingestion in ClickHouse failed when processing ISO 8601 strings with nanosecond precision.
- **Resolution**: Timestamps are landed as `String` in the Bronze layer. High-precision parsing is deferred to the Silver ETL stage using Polars' `.str().to_datetime()` for increased flexibility.

---

## 🚀 Performance & Scaling

### 4. Out of Memory (OOM) in Network Linkage
- **Problem**: Multi-million row many-to-many joins on IP and User Agent entities caused combinatorial explosions, leading to process termination.
- **Resolution**: The architecture shifted from an Edge-List Graph approach to an Entity Reputation model. Risk is now calculated at the entity level and joined back to transactions, reducing complexity from $O(N^2)$ to $O(N)$.

### 5. OOM in Large-Scale Generation
- **Problem**: Single-pass generation of 17M+ transactions exceeded available system RAM.
- **Resolution**: The generator was refactored to use a **Chunked One-Pass Architecture**. The population is processed in batches of 5,000 entities, with transactions flushed to Parquet incrementally to maintain a constant memory profile.

### 6. Parquet Serialization Bottleneck
- **Problem**: Transaction generation required 44 seconds, with 90% of the time spent in disk I/O and Parquet encoding.
- **Resolution**: A **One-Pass Parallel Architecture** was implemented and the Polars chunk size was optimized. This reduced the total runtime to 4.4 seconds, an 11x improvement.

---

## 🤖 Machine Learning & Data Science

### 7. Label Leakage (Near-Perfect AUC)
- **Problem**: Early models achieved 0.9993 AUC by learning internal generator flags (e.g., `geo_anomaly`) instead of behavioral patterns.
- **Resolution**: A strict **"Operational Feature" Sanitization** step was implemented to drop all internal metadata. The training target was also shifted from the perfect `fraud_target` to the noisy `is_fraud` label.

### 8. Observed vs. Configured Fraud Rate Discrepancy
- **Problem**: The observed fraud rate (~13.6%) appeared higher than the 12% defined in the configuration.
- **Resolution**: Validation confirmed that `is_fraud` deliberately incorporates simulated label noise (3% FP, 10% FN), resulting in a higher observed ratio than the latent ground truth.

### 9. High Fraud Prevalence in Initial Runs
- **Problem**: Approximately 86% of customers experienced fraud due to high default configuration values.
- **Resolution**: The `target_share` parameter was tuned to 0.005 (0.5% transaction rate) to align with industry benchmarks for sparse fraud data.

---

## Known Issues
There is ongoing difficulty with **Container Runtime Variability**. The `podman exec` calls used in ingestion and ETL pipelines behave inconsistently across Linux and macOS environments, causing failures in the data warehouse loading process. Transitioning to native database drivers is required to eliminate dependency on the host's container CLI.

Furthermore, **Memory Management during Reference Extraction** is currently insufficient. When processing large OSM PBF files, the `prepare_refs.rs` binary can consume significant RAM. Implementing a "Spill-to-Disk" strategy for the parallel map-reduce operation is necessary to maintain a memory footprint below 4GB.
