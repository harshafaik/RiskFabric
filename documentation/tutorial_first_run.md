# Tutorial: Your First Generation

Welcome to RiskFabric! This tutorial guides you through the "Golden Path" to generate your first synthetic dataset of 150,000 transactions in under 5 minutes.

## 1. Prerequisites
Before starting, ensure you have the following running:
- **PostgreSQL**: A local instance running on port `5432` with a database named `riskfabric`.
- **Rust**: Latest stable version installed.
- **Python**: A virtual environment set up in `env/`.

## 2. Preparing Geographic Reference Data
RiskFabric uses real-world data from OpenStreetMap (OSM) to place customers in realistic locations.

### Step 1: Extract OSM Nodes
Run the extractor to pull residential and merchant points from the India PBF file into your PostgreSQL database.
```bash
cargo run --release --bin extract_references
```
*Wait for "All done! Data loaded into PostgreSQL" to appear.*

### Step 2: Enrich with dbt
Now, we use SQL to perform spatial joins and assign H3 indices to these points.
```bash
# Move to warehouse and run dbt
cd warehouse
source ../env/bin/activate
dbt run
cd ..
```

### Step 3: Export to Parquet
The generator requires these references in high-performance Parquet format. Run the extractor one more time to export the clean marts.
```bash
cargo run --release --bin extract_references
```

---

## 3. Running the Generator
With the "Ground Truth" ready in `data/references/`, we can start the simulation.

### Step 4: Execute the Generation
```bash
cargo run --release --bin generate
```
The engine processes 10,000 cards in parallel and flushes them to disk. You should see a throughput of ~180,000 TPS.

---

## 4. Verifying the Results
Your synthetic data awaits in `data/output/`.

### Check Output Files
List the directory to ensure the generator created all tables:
```bash
ls -lh data/output/
```
You should see:
- `customers.parquet`
- `accounts.parquet`
- `cards.parquet`
- `transactions.parquet`
- `fraud_metadata.parquet`

### Quick Inspection
Use a tool like `parquet-cli` or a simple Python script to check the fraud distribution:
```python
import polars as pl
df = pl.read_parquet("data/output/transactions.parquet")
print(df["is_fraud"].value_counts())
```

**Congratulations!** You have successfully completed your first generation. Next, you can learn [How to add a new Fraud Signature](how_to_add_fraud.md) or dive into the [ETL & Feature Schema](etl_schema.md).
