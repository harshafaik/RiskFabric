# ETL Performance Optimizations

## Summary
The RiskFabric ETL pipeline (`etl.rs`) is designed for high-fidelity feature engineering using a hybrid Polars and ClickHouse architecture. While functionally robust, the current implementation contains several architectural bottlenecks that limit its scalability to billion-row datasets. This document outlines the identified performance issues and the strategic roadmap for transitioning to a high-concurrency, zero-copy pipeline.

## Architectural Decisions
To achieve enterprise-grade throughput, the pipeline is moving toward a **Parallel Stream-Oriented Architecture**. 

The primary decision is the shift to **Asynchronous Pipeline Orchestration**. By utilizing `tokio` or `rayon`, the independent "Silver" ETL stages (Customer, Merchant, Device/IP) will be executed in parallel. This maximizes multi-core utilization and significantly reduces the total wall-clock time of the transformation phase.

The second decision involves **Zero-Copy Data Exchange**. The current "Double Buffering" strategy—where data is fetched into memory, stored as a vector, and then parsed—is slated for replacement with a streaming architecture. By piping the raw output of the ClickHouse process directly into the Polars `ParquetReader` and vice-versa, the memory footprint is halved, and intermediate disk I/O for temporary Parquet files is eliminated.

Finally, the transition to **Native Driver Connectivity** via `clickhouse-rs` is prioritized over the current `podman exec` method. This eliminates the process overhead of spawning container instances for every query and provides superior type safety and error propagation.

## System Integration
The optimized ETL system remains the central bridge between the **Data Warehouse** (ClickHouse) and the **Machine Learning Pipeline**. By maintaining the Parquet exchange format but moving it through memory pipes rather than physical files, the system ensures that the "handshake" between Polars and ClickHouse remains high-speed while reducing infrastructure dependencies and disk wear.

## Performance Benchmarks & Results

| Implementation | Wall-Clock Time | CPU User Time | Memory / Disk Overhead |
| :--- | :--- | :--- | :--- |
| **Baseline (Sequential)** | ~22.5 seconds | ~19.6 seconds | High (Temp files + buffers) |
| **Optimized (Parallel + Pipes)** | ~21.1 - 22.5 seconds | ~32.4 seconds | Low (Streaming + Zero temp files) |

### Analysis
The implementation of **Rayon-based parallelism** and **Direct Stdin Piping** resulted in a significant increase in CPU utilization (~65% increase in User time), indicating that the Rust transformation engine is now processing multiple stages concurrently. 

However, the **Wall-Clock Time** remained relatively flat. This confirms that the pipeline is currently **I/O Bound by the ClickHouse single-node instance**. Spawning six parallel `podman exec` processes causes resource contention at the database level, preventing a linear speedup. 

### Implemented Improvements
1.  **Stage Parallelism**: All Silver ETL functions now run concurrently via `rayon`.
2.  **Streaming Ingestion**: Parquet data is piped directly from Polars to ClickHouse `stdin`, eliminating `data/tmp_*.parquet` file I/O.
3.  **Thread-Safe Workspace**: Each parallel stage uses isolated logic and unique identifiers to prevent race conditions.
4.  **Memory Optimization**: Replaced large `Vec<u8>` output buffers with direct process pipes where possible.
