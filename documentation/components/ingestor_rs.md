# Data Warehouse Ingestor (`ingest.rs`)

## Summary
The `ingest.rs` binary is the primary data loading utility that populates the RiskFabric data warehouse (ClickHouse). It consumes the raw Parquet output from the batch generator and transforms it into structured "Bronze" tables, providing the necessary foundation for downstream ETL and machine learning operations.

## Architectural Decisions
The ingestor handles the **initial schema enforcement** for the warehouse. A key architectural decision is the use of a two-stage ingestion process for transactions. First, raw data is loaded into `fact_transactions_bronze_raw` with all fields preserved as strings or basic types. Then, ClickHouse's `parseDateTime64BestEffort` performs a high-performance conversion into a typed `DateTime64` column for the final `fact_transactions_bronze` table. This approach ensures that data is not lost because of formatting mismatches during the initial bulk load.

The utility is **idempotent**, automatically dropping and recreating tables on every run. This simplifies the development lifecycle by ensuring the warehouse reflects the latest state of the synthetic generation configuration.

## System Integration
`ingest.rs` acts as the bridge between the **File System layer** and the **Warehouse layer**. It interacts directly with the `podman` container runtime to execute commands against the `riskfabric_clickhouse` instance. It is the prerequisite for the `etl.rs` pipeline, which expects the tables defined here to be present and populated.

## Known Issues
Data is currently piped into the warehouse using shell-based `cat` and `podman exec` commands. This is inefficient for large datasets and introduces a dependency on the host's shell environment. Refactoring this to use the ClickHouse HTTP interface or a native Rust client will allow for more reliable bulk inserts. 

Furthermore, the warehouse schema in `ingest.rs` has drifted from the Rust model definitions in `src/models/`. For example:
- The `dim_accounts` table in the warehouse is missing the `bank_id` and `account_no` fields present in `account.rs`.
- The `dim_cards` table is missing over 10 fields, including `issue_date`, `activation_date`, and all usage limit fields defined in `card.rs`.
- The `dim_customers` schema is more aligned but still represents a manual duplication of the `Customer` struct.

Unifying these schemas, ideally by deriving the ClickHouse DDL directly from the Rust structs, will ensure the warehouse remains a high-fidelity representation of the synthetic population.
