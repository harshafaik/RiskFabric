# Physical World Transformation (`warehouse/`)

## Summary
The `warehouse/` directory contains the SQL-based transformation logic for RiskFabric's physical environment. Using **dbt (data build tool)** and Postgres/PostGIS, this layer transforms raw OpenStreetMap (OSM) nodes into the "Physical World" reference data (Merchants and Residential points) used by the simulation engine.

## Architectural Decisions
This layer prioritizes **Geographic High-Fidelity**. Instead of relying on the often inconsistent "state" and "district" tags in OSM, a **Spatial Join Strategy** is implemented. By performing `ST_Intersects` operations against official geographic boundaries (provided by DataMeet), the transformation layer provides a verified ground truth for every coordinate in the simulation. This ensures that a customer living in "Mumbai" is programmatically anchored to the correct state and district boundaries, which is critical for realistic spatial velocity calculations.

For **Merchant Risk Profiles**, a categorical mapping strategy is implemented in the `stg_merchants` model. By mapping raw OSM sub-categories (like `jewelry` or `electronics`) to standardized RiskFabric categories and risk levels (LOW, MEDIUM, HIGH), the "Adversarial Ground Truth" is established for the simulation. This architectural decision allows the fraud engine to select high-risk merchants for specific attack profiles without needing to embed merchant-level risk logic into the Rust binaries.

## System Integration
The dbt layer acts as the "Level 0" enrichment engine. It consumes the raw tables populated by `prepare_refs.rs` and produces the `mart_residential` and `mart_merchants` models. These models are then exported to Parquet via `export_references.rs` or `dlt/pipelines.py` to be used as the primary lookup data for the simulation generators.

## Known Issues
**Spatial Joins are performed on every run** for the mart models. While this ensures data quality, it is computationally expensive and slow when processing millions of Indian OSM nodes. A "Spatial Indexing" strategy should be implemented or the boundary results materialized into a lookup table to reduce the processing time. 

Furthermore, the **City Normalization** logic is currently based on a simple regex-based macro. This fails to handle the wide variety of spelling variations and transliteration errors found in raw Indian OSM data. A fuzzy-matching strategy or integration of a dedicated geographic gazetteer is needed to ensure more robust city-level clustering in the simulation.
