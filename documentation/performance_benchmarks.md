# Performance Benchmarks

This document tracks the evolution of `riskfabric` generation performance, focusing on the journey to the 100k+ Transactions Per Second (TPS) milestone.

## Test Environment
- **Workload**: 10,000 Customers (~15,000 Accounts, ~150,000 Transactions)
- **Format**: Parquet (Snappy compression)
- **Hardware**: Single Workstation (Multi-threaded Rust)

## Milestone Log

### 1. Initial Port (Sequential Multi-Pass)
*Date: February 2026*
*   **Architecture**: Sequential loops for generation, fraud injection, and campaign mutations. Cryptographic `Sha256` hashing for reproducibility.
*   **Transaction Gen Time**: 44.11 seconds
*   **Total Runtime**: 48.76 seconds
*   **Throughput**: ~3,400 TPS
*   **Bottleneck**: Cryptographic hashing overhead and high memory access in multiple sequential passes.

### 2. Parallel Injection & Hash Optimization
*   **Architecture**: Parallelized the `inject` pass using `rayon`. Optimized `hash01` to reduce string allocations.
*   **Transaction Gen Time**: 35.86 seconds
*   **Total Runtime**: 40.35 seconds
*   **Throughput**: ~4,100 TPS
*   **Gain**: +20% improvement.

### 3. The "One-Pass" Unified Architecture (Current)
*Date: February 2026*
*   **Architecture**: 
    *   **Unified Loop**: All logic (Selection, Generation, Fraud, Campaigns) handled in a single parallel pass.
    *   **Fast PRNG**: Swapped `Sha256` for `StdRng` (seeded per card for stability).
    *   **Reduced Allocations**: Replaced UUIDs with synthetic IDs and pre-formatted timestamps.
*   **Transaction Gen Time**: 0.82 seconds
*   **Total Runtime**: 4.40 seconds (Includes all file I/O)
*   **Throughput**: **~182,000 TPS**
*   **Gain**: **53x improvement** from baseline.

## Summary of Optimization Impact

| Stage | Baseline (s) | Optimized (s) | Speedup |
| :--- | :--- | :--- | :--- |
| Customer Gen | 0.147 | 0.155 | 1x |
| Transaction Gen | 44.110 | 0.823 | **53.6x** |
| Parquet Write (Txn) | 3.696 | 2.640 | 1.4x |
| **Total Pipeline** | **48.763** | **4.402** | **11x** |

### 4. High-Fidelity One-Pass (Tuned)
*Date: February 2026*
*   **Architecture**: Added profile-specific geo-anomalies, campaign-coordinated spatial signals, and dynamic failure reasons.
*   **Performance**: Maintained throughput at **~180,000 TPS** despite increased logic complexity.
*   **Result**: High-quality training data with sharp spatial/temporal signals generated in < 4 seconds for 150k+ transactions.

---

### 5. Real-Time Streaming Throughput (Kafka)
*Date: March 15, 2026*
*   **Architecture**: 
    *   **Async I/O**: Leverages `tokio` and `rdkafka` for non-blocking Kafka publication.
    *   **Self-Correcting Limiter**: Measures per-message latency to adjust micro-sleep intervals.
    *   **Verification Mode Overhead**: Minimal (local CSV writes are buffered).
*   **Target Throughput**: 100 tx/s (Configurable)
*   **Actual Throughput**: 99.85 tx/s (Average over 1 hour)
*   **Publication Latency (P99)**: 4.2ms to local Kafka broker.

## Throughput Comparison

| Mode | Engine | Transport | Peak Throughput (TPS) |
| :--- | :--- | :--- | :--- |
| **Batch** | `generate.rs` | Local Parquet | ~180,000 |
| **Streaming**| `stream.rs` | Kafka Topic | ~1,200 (Unbound)* |

*\*Note: Streaming throughput is artificially limited to 100 tx/s for realism, but peak unbound performance is ~1,200 tx/s on a single thread.*

---

## Summary of Optimization Impact

*   **Scale Testing**: Achieve similar linear scaling for 100,000,000 transactions (targeting completion in under 15 minutes).
*   **I/O Optimization**: Investigate partitioned Parquet writes to further reduce the 2.6s write bottleneck.
