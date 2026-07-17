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

RiskFabric is a fraud intelligence platform designed for building fraud detection models using synthetic financial records. 

## ✨ Features
- **Agent-Based Realism**: Simulates the full lifecycle of `Customers`, `Accounts`, and `Cards`, with behavioral spend profiles driven by real-world heuristics.
- **Geographic Fidelity**: Integrates **OpenStreetMap (OSM)** and **H3** hexagonal indexing for realistic spatial spend patterns and location anomalies.
- **Sophisticated Fraud Injection**: Includes signatures for UPI Scams, Account Takeover (ATO), Card Not Present (CNP) fraud, and coordinated campaigns(yet to be implemented).

## 🛠️ Tech Stack
- **Core Engine**: Rust 
- **Real-time Streaming**: Redpanda (Kafka-compatible), `rdkafka`, and Tokio async runtime.
- **Data Processing**: Polars
- **Data Warehouse**: PostgreSQL (Spatial/OSM staging), Parquet (batch ETL via Polars), DuckDB (embedded training query engine), and dbt (Analytical enrichment).
- **Feature Store**: Redis
- **Data Ingestion**: `dlt` (Data Load Tool) for MDS integration.
- **Machine Learning**: XGBoost
- **Infrastructure**: Docker/Podman

## 🗄️ Case Management OLTP Store

### Architecture (OLTP vs. OLAP)
RiskFabric separates operational case management workloads (OLTP) from analytical workloads (OLAP) using distinct engines:
- **ClickHouse (OLAP)**: Optimizes bulk-analytical, historical, and batch operations for feature engineering and XGBoost training.
- **PostgreSQL (OLTP)**: Optimizes operational, read-write, single-record transactions. Handles live per-transaction case states, status modifications, and free-text investigator notes.

We run a dedicated Postgres service (`oltp-postgres`) separate from ClickHouse and the static PostGIS geo database to isolate analytical queries from transactional updates.

### Running Locally
To start the database and the case management admin panel:
```bash
podman-compose up --build -d
```

Once running:
- **Django Admin Interface**: Accessible at [http://localhost:8001/admin](http://localhost:8001/admin).
- **Default Credentials**: Username: `admin`, Password: `admin`.

### Ingestion Path Simulation
To simulate operational ingestion of flagged transactions into the case store, run:
```bash
podman-compose exec scorer python src/ml/ingest_cases.py
```
This script attempts to pull flagged transactions from ClickHouse's `fraud_scores` table, falling back to reading ground-truth fraud from `data/output/transactions.parquet` if ClickHouse has not yet been seeded. It then upserts the records into the Postgres `cases` table.

---
*Developed by harshafaik*
