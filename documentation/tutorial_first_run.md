# Your First Generation

## Summary
The `tutorial_first_run.md` provides a step-by-step operational guide for initializing the RiskFabric environment and executing a full synthetic data lifecycle—from world-building to real-time scoring.

## Design Intent
This tutorial serves as an **End-to-End Orchestration Blueprint**. Rather than solely demonstrating CSV generation, the guide outlines the Modern Data Stack (MDS) integration. By following this path, researchers can verify that every component—Rust generators, dbt models, ClickHouse ingestion, and Python inference—is correctly configured and communicating. 

A critical part of this design is the inclusion of the **Verification Mode** in the streaming section. This enables the evaluation of simulation fidelity by joining real-time model scores against the simulated ground truth.

---

## 🛠️ Step 1: World Building (Level 0)
Before generating transactions, the physical reference data must be prepared.
```bash
# Extract OSM nodes to Postgres
cargo run --bin riskfabric-prepare-refs -- extract-nodes

# Export to Parquet for the generator
cargo run --bin riskfabric-export-references
```

## 🚀 Step 2: Batch Generation & ETL
Create the historical dataset used for model training.
```bash
# Generate 10k customers and their history
cargo run --release --bin riskfabric-generate

# Ingest into ClickHouse and run ETL
cargo run --bin riskfabric-ingest
cargo run --bin riskfabric-etl -- silver-all
cargo run --bin riskfabric-etl -- gold-master
```

## 🧠 Step 3: Train & Stream
Train the fraud model and score live transactions.
```bash
# Train XGBoost model
python src/ml/train_xgboost.py

# Seed the Redis feature store
python src/ml/seed_redis.py

# Start the real-time scorer and the generator
python src/ml/scorer.py
cargo run --bin riskfabric-stream
```

---

## Known Issues
The tutorial documentation assumes a **local Podman/Docker environment**. Running without containers may cause database command failures. Adding a "Prerequisites" section to check for the availability of required services (Kafka, Redis, ClickHouse, Postgres) is necessary. 

Furthermore, the tutorial is **linear**. Instructions for handling "Incremental Updates" (e.g., appending transactions to an existing warehouse) are not yet included. Implementing a section on "Stateful Resumption" is required to assist with large-scale, multi-day simulation runs.
