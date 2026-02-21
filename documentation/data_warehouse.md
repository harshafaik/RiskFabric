# Data Warehouse & Analytics

RiskFabric uses a **Dual-Engine Architecture** to balance extreme performance with analytical flexibility. 

## The Dual-Engine Strategy

| Task | Engine | Reason |
| :--- | :--- | :--- |
| **High-Volume Generation** | Rust (Polars) | Multi-threaded performance for 100M+ rows. |
| **Feature Engineering** | Rust (Polars) | Low-latency sequence and graph calculations. |
| **Reference Data Prep** | dbt (SQL) | Declarative modeling for geographic enrichment. |
| **Analytical Reporting** | dbt (SQL) | Standardization and data quality audits. |

## dbt Project Structure (`warehouse/`)

The dbt project is responsible for transforming raw OpenStreetMap (OSM) data and geographic boundaries into the "Ground Truth" Parquet files used by the Rust generator.

### 1. Staging Layer
- `stg_osm__residential`: Cleans and deduplicates residential node data extracted from the India OSM PBF.
- `stg_merchants`: Normalizes merchant categories and geographic coordinates.

### 2. Marts Layer
- `geo_enriched_residential`: The "Master Geographic Reference." It joins residential points with state boundaries and assigns H3 indices. This table is exported to `data/references/ref_residential.parquet`.

### 3. Seeds
- `state_map.csv`: Standardized mapping of Indian state names and codes.
- `merchant_category_map.csv`: Maps OSM tags to financial Industry Codes (MCC).

## Running the Warehouse

To run the dbt models, you must use the Python environment provided in the project:

```bash
# Activate environment
source env/bin/activate

# Navigate to warehouse
cd warehouse

# Run models
dbt run
```

## The Feedback Loop
1. **Rust (`extract_references`)**: Extracts raw points from the India OSM PBF -> Lands in Postgres using high-performance binary copy.
2. **dbt**: Cleans, joins, and enriches data.
3. **Rust (`bin/extract_references` - second pass)**: Reads the dbt Marts -> Saves as optimized Parquet in `data/references/`.
4. **Rust (`generate`)**: Uses the Parquet references to ensure synthetic customers live in realistic locations.

## OSM Extraction Process

The `extract_references` binary is responsible for the initial "landing" of geographic data. It parses the multi-gigabyte India OSM PBF file in parallel and extracts three specific categories of data into the Postgres database:

### 1. `raw_residential`
- **Criteria**: OSM nodes tagged with `building=residential`, `landuse=residential`, or containing specific address tags (`addr:street`).
- **Purpose**: Defines the "home" coordinates for synthetic customers.
- **Fields**: `osm_id`, `h3_index`, `lat`, `lon`, `city`, `postcode`, `state`.

### 2. `raw_merchants`
- **Criteria**: Nodes tagged as `shop=*`, specific amenities (`restaurant`, `fuel`, `pharmacy`), or tourism spots (`hotel`).
- **Purpose**: Defines where transactions physically occur.
- **Fields**: `osm_id`, `h3_index`, `name`, `category`, `sub_category`, `lat`, `lon`.

### 3. `raw_financial`
- **Criteria**: Nodes tagged as `amenity=atm` or `amenity=bank`.
- **Purpose**: Used to simulate cash-out points and banking locations.
- **Fields**: `osm_id`, `h3_index`, `kind`, `operator`, `lat`, `lon`.

## High-Performance Ingestion
To handle millions of OSM nodes efficiently, the Rust engine utilizes the **Postgres Binary Copy** protocol (`BinaryCopyInWriter`). This bypasses standard SQL parsing overhead, allowing for ingestion speeds that match the multi-threaded parsing of the PBF file.
