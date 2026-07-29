# Defunct Implementations

Historical documentation retained for audit and migration reference. The pages here describe superseded designs that have been replaced or removed by decision; they do not reflect the current codebase.

## Modules

- [ETL Pipeline System (Shell-Pivot Variant)](components/defunct/etl_system_shell_pivot.md) — the pre-migration ETL design that routed all ClickHouse I/O through `podman exec clickhouse-client` shell pipelines. Superseded by the native `clickhouse` Rust crate implementation documented in [ETL Pipeline System](components/etl_system.md).

- [ClickHouse Batch ETL Architecture](components/defunct/clickhouse_batch_etl.md) — the previous architecture where ClickHouse served as the batch training data store (Bronze → Silver → Gold tables via native Rust ClickHouse client). Superseded by the Parquet-only pipeline with DuckDB for training queries, per the [Storage Architecture Split](decisions/storage_architecture_split.md) decision. ClickHouse remains only for the live streaming fraud_scores path.
