# Reference Data Preparator (`prepare_refs.rs`)

## Summary
The `prepare_refs.rs` binary is the "world-building" utility of RiskFabric. It is responsible for ingesting, filtering, and normalizing raw OpenStreetMap (OSM) data and other geographic datasets to create the high-performance reference files used by the simulation generators. It handles the task of mapping physical coordinates to behavioral entities like merchants, residential points, and financial institutions.

## Architectural Decisions
This utility is designed to handle **Parallel OSM Parsing** using the `osmpbf` library and `rayon`. Since the raw India PBF file is several gigabytes in size, the preparator uses a map-reduce strategy to extract relevant nodes (residential buildings, shops, and amenities) across all available CPU cores. This allows for the processing of a country's entire geographic dataset in minutes rather than hours.

A key architectural choice is the implementation of **Fuzzy State Normalization**. OSM data is often inconsistent, with the same state appearing in multiple formats (e.g., "AP," "Andhra Pradesh," or "Andra Pradesh"). A rule-based normalization engine standardizes these variations, ensuring that downstream generators can reliably perform state-level joins and spatial indexing without data gaps.

A **Postgres-Based Staging Layer** is also integrated for the extraction process. By using the `BinaryCopyInWriter` for bulk insertion, the preparator moves millions of extracted nodes into a structured database with minimal overhead. This staging layer allows for complex SQL-based cleaning and verification before the final reference Parquet files are exported.

## System Integration
`prepare_refs.rs` is a standalone "Level 0" utility that must be run before synthetic data generation. It populates the `data/references/` directory with `ref_merchants.parquet`, `ref_residential.parquet`, and other critical lookup tables. These files are then consumed by `generate.rs`, `stream.rs`, and `customer_gen.rs`.

## Known Issues
A hardcoded Postgres connection string (`postgres://harshafaik:123@localhost:5432/riskfabric`) is currently used within the CLI defaults. This is a security and portability issue; it should be moved to an environment variable or a configuration file. Additionally, the utility lacks a unified "Export to Parquet" command—it populates Postgres, but the final conversion to Parquet is often handled by separate, manual scripts. Consolidating the end-to-end pipeline (OSM → Postgres → Parquet) into this single binary would improve the developer experience.
