# Your First Generation

## Summary
This tutorial provides a step-by-step operational guide for initializing the RiskFabric environment and executing a full synthetic data lifecycle—from world-building to model training.

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
Before generating transactions, the physical reference data must be prepared by extracting OpenStreetMap nodes, enriching them via dbt, and exporting them to Parquet.

```bash
# 1. Extract raw OSM nodes to Postgres
cargo run --bin prepare_refs -- extract-nodes

# 2. Enrich & Transform (Spatial Joins and Risk Categorization)
dbt run --project-dir warehouse

# 3. Export to Parquet for the generator
# Option A: Rust-based export
cargo run --bin export_references
# Option B: DLT-based export (Recommended)
python dlt/pipelines.py export
```

### The Database Transformation Process
During this step, the Postgres database performs three critical operations to build the "Physical World":
1.  **Ingestion**: Millions of raw coordinates are copied from OSM PBF files into the staging area.
2.  **Spatial Anchoring**: `dbt` uses **PostGIS** to perform spatial intersections against official Indian boundaries, ensuring every coordinate is anchored to a verified State and District for realistic travel-velocity calculations.
3.  **Adversarial DNA**: Raw merchant tags are mapped to standardized categories (e.g., `LUXURY`, `GAMBLING`) and assigned baseline risk levels, establishing the ground truth for fraud injection.

## Step 2: Batch Generation and ETL
The historical dataset used for model training must be generated, ingested into the warehouse, and processed through the feature engineering pipeline.

### Configuring the Simulation
Before running the generation, you can tune the scale and behavior of the synthetic population in the `data/config/` directory:

*   **Population Scale (`customer_config.yaml`)**:
    *   `control.customer_count`: Total number of unique agents (Default: `3334`).
    *   `control.transactions_per_customer`: Min/Max transaction volume per agent (Default: `400-800`).
    *   `registration.lookback_years`: How far back the customer history goes (Default: `5 years`).
*   **Transaction Patterns (`transaction_config.yaml`)**:
    *   `transactions.lookback_days`: Duration of the generated transaction history (Default: `365 days`).
    *   `transactions.amount_range`: The global min/max for transaction values (Default: `10 - 50,000 INR`).
    *   `temporal_patterns`: Hourly and daily weights that drive circadian rhythms.
*   **Fraud Injection (`fraud_rules.yaml`)**:
    *   `fraud_injector.target_share`: The percentage of transactions that are intentionally fraudulent (Default: `0.01` or 1%).
    *   `fraud_injector.default_fp_rate`: Baseline "noise" (False Positives) injected into the labels (Default: `0.005`).
    *   `fraud_injector.profiles`: Tune the frequency and behavior of specific attack types (UPI Scams, ATO, Velocity Abuse).

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

### Model Configuration established
The training script (`train_xgboost.py`) uses a configration optimal for high-imbalance datasets:

*   **Class Imbalance Handling**: The `scale_pos_weight` is calculated dynamically (Legitimate / Fraud ratio) to ensure the model doesn't ignore the minority fraud class.
*   **Hyperparameters**:
    *   `n_estimators`: 100
    *   `max_depth`: 6
    *   `learning_rate`: 0.1
    *   `eval_metric`: `aucpr`
*   **Operational Feature Set**: The model trains on 12 behavioral features (e.g., `spatial_velocity`, `amount_deviation_z_score`), explicitly excluding synthetic IDs (`customer_id`, etc.) to prevent label leakage.

```bash
# Train the fraud detection model
python src/ml/train_xgboost.py
```

### Model Validation and Interpretability
Before moving to production scoring, you should validate the model's performance and interpret its decision drivers:

*   **Performance Testing (`test_model.py`)**: Runs the trained model against a test dataset to generate classification reports and conduct threshold analysis (identifying the optimal Precision/Recall trade-off).
*   **Explainability (`shap_analysis.py`)**: Uses SHAP (SHapley Additive exPlanations) to create visual reports in `reports/shap/`. This identifies which features (e.g., `spatial_velocity`) drove the model's flags globally and for each specific fraud profile.
*   **Model Metadata (`dump_model.py`)**: A developer utility used to inspect the internals of the saved JSON model, verifying feature names, types, and categorical encodings.

```bash
# Run performance and threshold analysis
python src/ml/test_model.py

# Generate SHAP interpretability reports
python src/ml/shap_analysis.py

# (Optional) Inspect model metadata
python src/ml/dump_model.py
```

### Starting the Real-time Pipeline
Once the model is validated, seed the feature store and start the inference engine:

```bash
# Seed the Redis feature store with warehouse state
python src/ml/seed_redis.py

# Start the real-time scorer and the streaming generator
python src/ml/scorer.py
cargo run --bin stream
```


## Known Issues
The documentation assumes a local container environment. Running without containers may result in database connection failures. Explicit validation of service availability (Kafka, Redis, ClickHouse, Postgres) is required before beginning the tutorial. 

Furthermore, the tutorial follows a linear path. Instructions for incremental updates, such as appending new transactions to an existing warehouse, are currently omitted. Implementation of stateful resumption guidance is required for large-scale simulation runs.
