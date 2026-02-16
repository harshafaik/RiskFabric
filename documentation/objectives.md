# Project Objectives

This document outlines the strategic goals and technical benchmarks for the `riskfabric` synthetic fraud generator and ETL pipeline.

## 1. The 1-Billion-Row Benchmark (The "North Star")
The primary technical objective is to generate, store, and process **1,000,000,000 (one billion)** high-fidelity transactions on a single workstation.

### Performance Requirements
*   **Generation Speed**: Achieve a generation rate of >100,000 transactions per second (TPS) using multi-threaded Rust.
*   **Optimization Goals (Active)**:
    *   Parallelize Fraud & Campaign injection logic using `rayon` to remove the 44s bottleneck for 10k users.
    *   Optimize hashing-based PRNG logic to minimize cryptographic overhead.
    *   Reduce string allocations in high-frequency generation loops.
*   **Storage Efficiency**: Utilize ClickHouse (Columnar storage) and Partitioned Parquet to keep the 1B dataset within 150GB–250GB of disk space.
*   **ETL Latency**: Calculate sequence and behavioral features (Silver layer) across the entire 1B dataset in under 30 minutes using Polars Streaming or ClickHouse Materialized Views.

### Strategic Goals
*   **Scale Demonstration**: Show that Rust and Polars can outperform traditional distributed Spark clusters for billion-row workloads on vertical hardware.
*   **ML Readiness**: Provide a 10M-row balanced training subset sampled from the 1B-row population.
*   **XGBoost Baseline (v2)**: Successfully trained the first XGBoost model on a 4.3M row high-fidelity dataset, achieving a 0.97 AUC while using noisy labels and sanitized features.
*   **Inference Testing**: Use the remaining 990M rows to benchmark the latency and accuracy of the fraud detection model.

## 2. Medallion Architecture Implementation
Maintain a strict data quality pipeline to mirror professional fintech environments:
*   **Bronze**: Raw transaction and entity streams.
*   **Silver**: Feature-engineered tables (velocities, ratios, consistency flags).
*   **Gold**: Denormalized, ML-ready tables with joined dimensions.

## 3. High-Fidelity Fraud Simulation
*   Port and enhance complex fraud patterns (UPI scams, SIM swaps, ATO) from existing ETL logic.
*   Introduce "ground truth" labels vs. "noisy" labels (FN/FP) to simulate real-world label delay and error.
*   **International Fraud System (Future)**: Design and propagate systems to ensure realistic international fraud transactions (cross-border flows, currency conversion friction, and high-risk country profiles).
