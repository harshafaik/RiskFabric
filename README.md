# RiskFabric

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Polars](https://img.shields.io/badge/engine-Polars%200.51.0-blue.svg)](https://pola.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

RiskFabric is a fraud intelligence platform that generates synthetic Indian payment transaction data, processes it through a Medallion ETL pipeline, and produces trained fraud detection models.

## 🚀 The "North Star" Objective
RiskFabric demonstrates that a modern, vertically-scaled Rust stack can generate, store, and process **100 Million high-fidelity transactions** on a single workstation, outperforming traditional distributed clusters for large-scale financial simulation and ML training.

## ✨ Key Features
- **Extreme Throughput**: Achieves **~182,000 Transactions Per Second (TPS)** using a parallelized "One-Pass" architecture.
- **Agent-Based Realism**: Simulates the full lifecycle of `Customers`, `Accounts`, and `Cards`, with behavioral spend profiles driven by real-world heuristics.
- **Geographic Fidelity**: Integrates **OpenStreetMap (OSM)** India data and **Uber H3** hexagonal indexing for hyper-realistic spatial spend patterns and location anomalies.
- **Sophisticated Fraud Injection**: Includes signatures for UPI Scams, Account Takeover (ATO), Card Not Present (CNP) fraud, and coordinated campaigns.
- **Medallion Data Architecture**: A full pipeline taking data from **Bronze** (Raw) to **Silver** (Feature Engineered) to **Gold** (ML-Ready).
- **ML Mastery**: Built-in leakage prevention and simulated label noise (False Positives/Negatives) to ensure models are robust and production-ready.

## 🛠️ Tech Stack
- **Core Engine**: Rust (Rayon for parallelization, Rand for deterministic seeding).
- **Data Processing**: Polars 0.51.0 (Lazy API & Streaming).
- **Data Warehouse**: PostgreSQL (Spatial Reference) & dbt (Geographic Enrichment).
- **Storage**: Snappy-compressed Parquet & ClickHouse.
- **Machine Learning**: Python (XGBoost) with sanitized feature vectors.

## 📁 Project Structure
- `src/generators/`: Core ABM logic and fraud mutation engine.
- `src/etl/`: Polars-based feature engineering (Velocities, Reputations, Sequences).
- `src/bin/`: CLI utilities for OSM extraction, data landing, and pipeline orchestration.
- `warehouse/`: dbt project for geographic reference modeling.
- `documentation/`: Detailed technical documentation (mdBook).

## 🚀 Quick Start

### 1. Prerequisites
- Rust (Latest Stable)
- PostgreSQL (Local instance for geographic references)
- Python 3.10+ (For ML and dbt)

### 2. Generate Data
```bash
# Generate the population and transaction stream
cargo run --release --bin generate
```

### 3. Run ETL
```bash
# Process raw data into Silver and Gold layers
cargo run --release --bin etl_gold_master
```

### 4. View Documentation
Technical details, schemas, and theory of operation are available in the local mdBook:
```bash
mdbook serve --open
```

## 📈 Benchmarks (150k Txns)
| Architecture | Throughput | Total Time | Speedup |
| :--- | :--- | :--- | :--- |
| Sequential Port | 3,400 TPS | 48.7s | 1x |
| **Optimized One-Pass** | **182,000 TPS** | **4.4s** | **53x** |

---
*Developed by harshafaik*
