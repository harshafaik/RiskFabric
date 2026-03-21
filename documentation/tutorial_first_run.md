# Your First Generation

## Summary
The `tutorial_first_run.md` provides a step-by-step operational guide for initializing the RiskFabric environment and executing a full synthetic data lifecycle—from world-building to real-time scoring.

## Design Intent
This tutorial provides an end-to-end workflow for the simulation. By following this path, the integration between the Rust generators, dbt models, ClickHouse ingestion, and Python inference can be verified. The workflow includes the use of Verification Mode in the streaming section to evaluate simulation fidelity by joining real-time model scores against the simulated ground truth.

## Prerequisites
The following components must be installed and available:
- **Rust** (Latest Stable)
- **Docker** or **Podman** (with Docker Compose support)
- **Python 3.10+**
- **Git**

## Step 0: Infrastructure Setup
The simulation requires several backing services (Postgres, ClickHouse, Redpanda, Redis). These are orchestrated via Docker Compose and must be running before the generation binaries are executed.

```bash
# Start the local service stack
docker-compose up -d
```

## Step 1: World Building (Level 0)
Before generating transactions, the physical reference data must be prepared by extracting OpenStreetMap nodes and exporting them to Parquet.

```bash
# Extract OSM nodes to Postgres
cargo run --bin prepare_refs -- extract-nodes

# Export to Parquet for the generator
cargo run --bin export_references
```

## Step 2: Batch Generation and ETL
The historical dataset used for model training must be generated, ingested into the warehouse, and processed through the feature engineering pipeline.

```bash
# Generate the initial population and history
cargo run --release --bin generate

# Ingest into ClickHouse and run ETL layers
cargo run --bin ingest
cargo run --bin etl -- silver-all
cargo run --bin etl -- gold-master
```

## Step 3: Model Training and Streaming
The final phase involves training the XGBoost classifier, seeding the real-time feature store, and starting the streaming simulation.

```bash
# Train the fraud detection model
python src/ml/train_xgboost.py

# Seed the Redis feature store with warehouse state
python src/ml/seed_redis.py

# Start the real-time scorer and the streaming generator
python src/ml/scorer.py
cargo run --bin stream
```

## Known Issues
The documentation assumes a local container environment. Running without containers may result in database connection failures. Explicit validation of service availability (Kafka, Redis, ClickHouse, Postgres) is required before beginning the tutorial. 

Furthermore, the tutorial follows a linear path. Instructions for incremental updates, such as appending new transactions to an existing warehouse, are currently omitted. Implementation of stateful resumption guidance is required for large-scale simulation runs.
