# RiskFabric

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/language-Python-blue.svg)](https://www.python.org/)
[![Polars](https://img.shields.io/badge/engine-Polars%200.51.0-blue.svg)](https://pola.rs/)
[![ClickHouse](https://img.shields.io/badge/warehouse-ClickHouse-yellow.svg)](https://clickhouse.com/)
[![Redpanda](https://img.shields.io/badge/streaming-Redpanda-red.svg)](https://redpanda.com/)
[![Redis](https://img.shields.io/badge/cache-Redis-red.svg)](https://redis.io/)
[![Docker](https://img.shields.io/badge/orchestration-Docker-blue.svg)](https://www.docker.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Deploy mdBook](https://github.com/harshafaik/riskfabric/actions/workflows/deploy_book.yml/badge.svg)](https://github.com/harshafaik/riskfabric/actions/workflows/deploy_book.yml)

RiskFabric is a fraud intelligence platform that generates synthetic Indian payment transaction data, processes it through a Medallion ETL pipeline, and produces trained fraud detection models.

## ✨ Key Features
- **Extreme Throughput**: Achieves **~182,000 Transactions Per Second (TPS)** using a parallelized "One-Pass" architecture.
- **Agent-Based Realism**: Simulates the full lifecycle of `Customers`, `Accounts`, and `Cards`, with behavioral spend profiles driven by real-world heuristics.
- **Geographic Fidelity**: Integrates **OpenStreetMap (OSM)** India data and **Uber H3** hexagonal indexing for hyper-realistic spatial spend patterns and location anomalies.
- **Sophisticated Fraud Injection**: Includes signatures for UPI Scams, Account Takeover (ATO), Card Not Present (CNP) fraud, and coordinated campaigns.
- **Medallion Data Architecture**: A full pipeline taking data from **Bronze** (Raw) to **Silver** (Feature Engineered) to **Gold** (ML-Ready).
- **ML Mastery**: Built-in leakage prevention and simulated label noise (False Positives/Negatives) to ensure models are robust and production-ready.

## 🛠️ Tech Stack
- **Core Engine**: Rust (Rayon for parallelization, Rand for deterministic simulation).
- **Real-time Streaming**: Redpanda (Kafka-compatible), `rdkafka`, and Tokio async runtime.
- **Data Processing**: Polars 0.51.0 (Lazy API & high-performance transformation).
- **Data Warehouse**: PostgreSQL (Spatial/OSM staging), ClickHouse (High-volume transactions), and dbt (Analytical enrichment).
- **Feature Store**: Redis (Low-latency state for real-time Z-scores and behavior).
- **Data Ingestion**: `dlt` (Data Load Tool) for MDS integration.
- **Machine Learning**: Python (XGBoost) with real-time inference via `scorer.py`.
- **Infrastructure**: Docker/Podman orchestration with Prometheus and Grafana for observability.

## 📁 Project Structure

### 🧠 Core Simulation (`src/`)
- `generators/`: Agent-Based Modeling (ABM) logic, entity creation, and fraud mutation engines.
- `models/`: Rust structures for Customers, Accounts, Cards, and Transactions.
- `bin/`: CLI binaries for data generation (`generate.rs`), streaming (`stream.rs`), and preparation.
- `config.rs`: Centralized, type-safe configuration engine for simulation parameters.

### 🥈 ETL & Data Warehouse (`src/etl/` & `warehouse/`)
- `etl/`: Multi-stage Polars transformation pipeline (Silver/Gold feature engineering).
- `warehouse/`: dbt project for geographic enrichment and merchant risk profiling using PostGIS.
- `dlt/`: MDS integration for automated data lake ingestion.

### 🤖 Machine Learning (`src/ml/`)
- `train_xgboost.py`: Training pipeline with Feature sanitization and OOT validation.
- `scorer.py`: Real-time inference service consuming from Kafka and stateful Redis features.
- `seed_redis.py`: Point-in-time state synchronization between the warehouse and feature store.

### 🛠️ Infrastructure & Docs
- `docker-compose.yml`: Orchestrated local stack (ClickHouse, Postgres, Redpanda, Redis, Grafana).
- `documentation/`: Arichitectural docs and theory of operation (mdBook).
- `data/config/`: Behavioral rules and system tuning YAML configurations.

## 📈 Benchmarks (150k Txns)
| Architecture | Throughput | Total Time | Speedup |
| :--- | :--- | :--- | :--- |
| Sequential Port | 3,400 TPS | 48.7s | 1x |
| **Optimized One-Pass** | **182,000 TPS** | **4.4s** | **53x** |

---
*Developed by harshafaik*
