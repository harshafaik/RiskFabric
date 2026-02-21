# Project Goals & North Star

RiskFabric aims to redefine the performance boundaries of synthetic data generation. This document outlines our strategic goals, the depth of our implementation, and the current progress toward the "North Star."

## 📊 Goal Status Summary

| Goal | Status | Metric / Evidence |
| :--- | :--- | :--- |
| **100k+ TPS Throughput** | ✅ **Complete** | Achieved **182,000 TPS** in One-Pass mode. |
| **Medallion Architecture** | ✅ **Complete** | Full Bronze -> Silver -> Gold pipeline in Rust. |
| **Realistic Fraud Signals** | 🟡 **In-Progress** | 5 patterns implemented; Jittering pending. |
| **XGBoost Training Flow** | ✅ **Complete** | Pipeline ready; 0.97 AUC achieved on noisy data. |
| **100M-Row Milestone** | 🔵 **Planned** | Validated up to 17M; Streaming ETL pending. |

---

## 1. The 100-Million-Row Benchmark
The "North Star" involves generating, storing, and processing **100,000,000** high-fidelity transactions on a single workstation. This benchmark proves that vertical scaling with Rust/Polars can outperform traditional horizontal scaling (Spark) for many financial use cases.

### Performance Needs
*   **Generation Speed**: Sustain >100,000 TPS using multi-threaded Rust.
    *   **Implementation**: A "One-Pass" architecture that minimizes string allocations and context switching by handling fraud, campaigns, and generation in a single loop, processing the population in segments of 5,000 entities.
*   **Storage Efficiency**: Keep 100M rows within optimized disk footprints using ClickHouse and Snappy-compressed Parquet.
    *   **Target**: ~15GB-25GB for the full raw dataset.
*   **ETL Latency**: Calculate complex behavioral features across 100M rows in under 15 minutes.
    *   **Need**: Migration from Polars eager API to the **Streaming Lazy API** to handle datasets larger than available RAM.

---

## 2. Medallion Architecture Implementation
RiskFabric mirrors professional financial technology environments with a strict three-tier data quality pipeline.

*   **Bronze (Raw)**: Captures the raw output of the generator. It stores transactions exactly as the synthetic banking system "observed" them, including simulated transmission delays.
*   **Silver (Features)**: The "Engine Room" where the pipeline calculates 22+ features. This layer performs windowed calculations (Time Since Last txn), entity reputations (Merchant Fraud Rate), and network linkage (IP Sharing).
*   **Gold (Master)**: A flat table ready for Machine Learning. It joins the Bronze stream with Silver features and Customer/Card dimensions to provide a single source of truth for model training.

---

## 3. High-Fidelity Fraud Simulation
We generate data that mirrors real fraud through multi-layered mutation logic.

### Behavioral Signatures
The engine supports five core fraud profiles, each with distinct distributions for amount, time, and frequency:
- **UPI Scam**: High-frequency, medium-amount transfers with high geographic anomalies.
- **Account Takeover (ATO)**: Sudden shifts in Device ID and User Agent coupled with escalating amounts.
- **Velocity Abuse**: "Testing" transactions characterized by round numbers or rapid-fire sequences.
- **Card Not Present (CNP)**: Online-only channel bias using standard e-commerce amount distributions.
- **Friendly Fraud**: Legitimate device and location signatures but with later chargeback events.

### Spatial & Network Realism
- **OSM Integration**: All transactions occur at real-world coordinates extracted from OpenStreetMap India nodes.
- **H3 Grid**: We use Uber’s H3 hexagonal grid (Resolution 7) to simulate neighborhood clusters and calculate "Distance from Home" anomalies.
- **Entity Sharing**: We simulate coordinated campaigns by forcing many synthetic customers to share the exact same IP Address and Device ID for a burst period.

---

## 4. Machine Learning & Validation
A primary goal ensures that the synthetic data remains "trainable" and representative of real-world challenges.

*   **Leakage Prevention**: We strictly enforce a "Sanitized Feature Vector." The pipeline prevents models from seeing ground-truth metadata (e.g., `fraud_type`), forcing them to learn from raw behavioral signals.
*   **Simulated Label Noise**: We inject 3% False Positives and 10% False Negatives into the `is_fraud` label to mirror the reality of imperfect human-labeled data in banking.
*   **Validation Metrics**: An XGBoost baseline validates every generation pass to ensure the AUC remains within a realistic range (0.95 - 0.98).
