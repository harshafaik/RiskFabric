# RiskFabric

RiskFabric is an end-to-end fraud intelligence platform that generates synthetic financial data, trains XGBoost detection models, and scores transactions in real time against live behavioral patterns. It simulates the full lifecycle of customers, accounts, cards, and transactions with geographic fidelity sourced from OpenStreetMap, injects realistic fraud signatures, and surfaces flagged cases to analysts through a Django case management interface.

## ✨ Features
- **Synthetic Data Generation**: Rust-based generators create realistic customer profiles, accounts, cards, and transactions at scale, with configurable behavioral spend profiles and geographic distributions.
- **Geographic Realism**: OpenStreetMap reference points and H3 hexagonal indexing drive residential and merchant placement, with location-type-aware clustering (Metro/Urban/Rural) and ~500m spatial jitter to prevent coordinate clumping.
- **Fraud Injection Engine**: Simulates UPI scams, Account Takeover (ATO), and Card Not Present (CNP) fraud using configurable signatures that corrupt transaction metadata, amounts, channels, and temporal patterns (coordinated campaigns yet to be implemented).
- **Real-Time Scoring Pipeline**: Kafka consumer reads transaction streams, enriches with Redis feature lookups, runs XGBoost inference, and writes fraud scores to ClickHouse — all visible on Grafana dashboards with sub-second latency.
- **ML Training & Evaluation**: DuckDB queries against Gold Parquet tables feed XGBoost training with Platt/Isotonic calibration, SHAP feature analysis, data drift simulation, and leakage audits.
- **Analyst Case Management**: Flagged transactions are ingested into a PostgreSQL-backed Django admin with a state-machine review workflow (pending → investigating → confirmed fraud / cleared / false positive), notes, and SHAP explanations.

## 🛠️ Tech Stack
- **Core Engine**: Rust (Tokio, Polars, rdkafka, h3o, osmpbf, Rayon, serde, clap)
- **Real-time Streaming**: Redpanda (Kafka-compatible)
- **Data Processing**: Polars (batch ETL), DuckDB (embedded training queries), dbt + PostGIS (OSM enrichment)
- **Data Stores**: PostgreSQL + PostGIS (spatial staging), ClickHouse (real-time fraud scores), Redis (feature store), Parquet (Bronze/Silver/Gold)
- **Data Ingestion**: dlt (reference data export from Postgres to Parquet)
- **Machine Learning**: XGBoost (classifier), scikit-learn (evaluation, calibration), SHAP (feature explanation), NumPy, pandas
- **Real-time Scoring**: Kafka consumer → Redis lookups → XGBoost inference → ClickHouse writes
- **Dashboards**: Grafana (ClickHouse-backed)
- **Case Management**: Django 4.2 + Jazzmin — analyst review UI for investigating flagged transactions, updating case statuses, and recording dispositions
- **Infrastructure**: Docker Compose (7 services), Podman, Terraform (AWS)
- **Documentation**: mdBook

## 📁 Project Structure

### Root Directory

```
📁 RiskFabric
 │
 ├─ ⚙️ Engine
 │   ├─ src/ — Rust Core (generators, ETL, pipeline)
 │   └─ Cargo.toml
 │
 ├─ 🧠 ML & Data
 │   ├─ src/ml/ — Python ML (training, scoring, explainability)
 │   ├─ models/ — XGBoost artifacts
 │   ├─ reports/ — SHAP analysis outputs
 │   ├─ warehouse/ — dbt models + PostGIS refs
 │   ├─ data/ — Configs, Parquet inputs/outputs
 │   └─ dlt/ — Data Load Tool pipelines
 │
 ├─ ☁️ Ops & Infra
 │   ├─ case_admin/ — Django admin (case management)
 │   ├─ deploy/ — Terraform (GCP deployment)
 │   ├─ docker/ — ClickHouse init, Grafana provisioning
 │   └─ docker-compose.yml
 │
 └─ 📖 Docs (documentation/ — mdBook)
```

