# Reference Data Pipeline (`dlt/pipelines.py`)

## Summary
The `dlt/pipelines.py` script is the Modern Data Stack (MDS) integration for RiskFabric. It uses the `dlt` (Data Load Tool) library to manage the extraction and movement of cleaned, enriched geographic data from the staging database (Postgres) into the optimized Parquet reference files used by the generators.

## Architectural Decisions
This pipeline is designed to facilitate **Declarative Reference Data Export**. Instead of custom SQL-to-Parquet conversion logic (as seen in `export_references.rs`), this script leverages the `dlt` library’s built-in support for the "filesystem" destination. This allows for automated schema handling and standardized Parquet formatting, which is critical for maintaining consistency between the OSM-derived reference data and the Rust-based simulation.

A key architectural choice was the use of **`write_disposition="replace"`**. Since the reference data (merchants and residential nodes) represents a "static" world that is fully rebuilt after every OSM extraction, this strategy ensures that the `data/references/` directory always contains a clean snapshot of the environment without manual cleanup.

## System Integration
`dlt/pipelines.py` acts as an alternative or supplementary utility to `export_references.rs`. It bridges the **Staging layer** (Postgres) and the **Local File System layer**. It is typically run as part of the "Level 0" world-building phase, specifically after dbt has transformed the raw OSM nodes into the `mart_residential` and `mart_merchants` models.

## Known Issues
Environment variables (e.g., `DESTINATION__FILESYSTEM__BUCKET_URL`) are currently used to configure the DLT pipeline directly within the Python script. This approach is fragile and makes it difficult to change the reference directory without modifying the code. These should be moved into a dedicated `dlt_config.toml` file to align with the library’s best practices. Additionally, the pipeline currently lacks **Data Validation tests**; dlt "checks" should be implemented to ensure that the exported Parquet files contain the expected number of rows and non-null H3 indices before they are handed off to the generation engine.
