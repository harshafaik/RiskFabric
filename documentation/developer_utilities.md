# Developer Utilities & CLI Tools

RiskFabric includes a suite of standalone Rust binaries in `src/bin/` to handle geographic data preparation, OSM extraction, and ETL orchestration. These tools must be run in a specific sequence to prepare the environment for generation.

---

## 🗺️ Phase 1: Geographic Discovery
Before generating customers, we must understand the geographic hierarchy of the target region (India).

### 1. `parse_osm`
Scans the India OSM PBF file to build a mapping of Districts to States using ISO-3166-2 codes.
```bash
cargo run --release --bin parse_osm
```
- **Output**: `data/state_district_map.txt`

### 2. `map_city_state`
Aggregates OSM nodes to count how many residential/merchant points exist in every city/state combination.
```bash
cargo run --release --bin map_city_state
```
- **Output**: `data/city_state_report.txt`

### 3. `normalize_state_names`
Messy OSM data often contains variations (e.g., "MH", "Maharastra", "Maharashtra"). This tool standardizes all discovered names against a master list.
```bash
cargo run --release --bin normalize_state_names
```
- **Output**: `data/city_state_report_clean.txt`

---

## 📥 Phase 2: Data Landing
Once the hierarchy is discovered, we extract raw points into the database.

### 4. `extract_references` (Lander)
Performs a high-performance parallel extraction of OSM nodes and writes them to Postgres using binary copy.
```bash
cargo run --release --bin extract_references
```
- **Writes to**: `raw_residential`, `raw_merchants`, `raw_financial` in Postgres.

---

## 🥈 Phase 3: Reference Export
After the dbt project has enriched the raw data (see [Data Warehouse & dbt](data_warehouse.md)), we must export the clean Marts back to Parquet for the Generator to use.

### 5. `extract_references` (Exporter)
*Note: This binary handles both landing and exporting depending on current implementation/flags.*
It reads the `geo_enriched_residential` Mart from Postgres and saves it as an optimized Parquet file.
```bash
cargo run --release --bin extract_references
```
- **Output**: `data/references/ref_residential.parquet`

---

## 🧪 Phase 4: Medallion ETL
These binaries run the feature engineering pipeline on previously generated data.

### 6. `etl_silver_*`
Individual binaries for each feature category (Sequence, Merchant, Network, etc.).
```bash
cargo run --release --bin etl_silver_sequence
cargo run --release --bin etl_silver_network
```

### 7. `etl_gold_master`
The final orchestrator that joins all Silver tables into the ML-ready Gold Master.
```bash
cargo run --release --bin etl_gold_master
```

---

## 🚀 Phase 5: The Generator
The main entry point for the simulation.

### 8. `generate`
Loads all reference Parquet files and YAML configs to produce the final synthetic dataset.
```bash
cargo run --release --bin generate
```
- **Output**: `data/output/*.parquet`
