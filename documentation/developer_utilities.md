# Developer Utilities CLI

## Summary
The `developer_utilities.md` document details specialized binaries and tools designed to support the RiskFabric development lifecycle. These utilities automate auxiliary tasks surrounding synthetic data generation, such as geographic preprocessing, reference data export, and model metadata inspection.

## Design Intent
These utilities function as a **Developer's Toolkit** for the simulation. By decomposing complex tasks—such as OSM node extraction and Parquet serialization—into dedicated CLI binaries, the core generation engine remains focused. This modular approach allows the synthetic environment to be rebuilt independently of the transaction simulation, enabling iteration on geographic density and merchant risk profiles.

A critical design choice was the use of **Strongly-Typed Subcommands** via the `clap` library. This provides a consistent, self-documenting interface for every utility, reducing cognitive load and ensuring operational errors are caught during argument parsing.

---

## 🔧 Core Utilities

### `riskfabric-prepare-refs`
The primary utility for extracting and normalizing OSM data.
- **`extract-nodes`**: Parallel parsing of PBF files into a Postgres staging layer.
- **`map-city-state`**: Rules-based geographic normalization.

### `riskfabric-export-references`
The serializer bridging the staging database and the generation layer.
- **Function**: Converts Postgres tables into H3-indexed Parquet files.

### `riskfabric-ingest`
The automated loader for the ClickHouse data warehouse.
- **Function**: Handles schema creation and bulk loading of generated transactions.

---

## Known Issues
Two separate binaries are currently maintained for reference handling (`prepare-refs` and `export-references`), which introduces friction in the developer workflow. Consolidation into a **Unified "Refs" Command** with subcommands for extraction, normalization, and export is planned. 

Furthermore, **Duplicate Connection Logic** exists across several utilities, with database URLs and file paths hardcoded in multiple binaries. Refactoring common CLI logic into a shared `riskfabric-cli-core` crate is required to ensure consistent handling of parameters like `--db-url` and `--output-dir`.
