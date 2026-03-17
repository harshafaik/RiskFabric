# Reference Data Exporter (`export_references.rs`)

## Summary
The `export_references.rs` binary is the final stage of the reference data preparation pipeline. It extracts cleaned and processed geographic data from the staging database (Postgres) and serializes it into the high-performance Parquet format required by the simulation generators. This utility ensures that the "synthetic world" is correctly typed, indexed, and portable across different environments.

## Architectural Decisions
This utility is designed to act as the **Final Schema Validator** for the reference data. While the `prepare_refs.rs` utility handles raw extraction and normalization, the exporter ensures that the data is structured exactly as expected by the generators. By using Polars to build the final DataFrames, high-performance memory management and efficient Parquet serialization are leveraged, which is critical when dealing with millions of reference nodes.

A key architectural choice is the **Database-to-Parquet decoupling**. By exporting processed staging tables into standalone Parquet files, the simulation environment becomes portable. This allows the core RiskFabric generators to run without a live Postgres connection, simplifying the deployment and execution of the simulation on local workstations or in CI/CD pipelines.

## System Integration
`export_references.rs` is a "Level 0" utility that bridges the **Staging layer** (Postgres) and the **Generation layer** (Parquet). It is typically run after `prepare_refs.rs` and any subsequent SQL-based cleaning has been performed on the staging tables. The resulting Parquet files in `data/references/` are the direct input for `generate.rs`, `stream.rs`, and the various generator modules.

## Known Issues
A hardcoded Postgres connection string (`postgres://harshafaik:123@localhost:5432/riskfabric`) is currently used directly in the source code. This is a duplicate of the issue in `prepare_refs.rs` and should be unified into a shared configuration or environment variable. Additionally, the exporter manually maps Postgres rows into local vectors before creating the Polars DataFrame. This is inefficient for extremely large datasets; refactoring to use a streaming connector or a more direct Polars-Postgres integration is needed to reduce the memory overhead of the export process.
