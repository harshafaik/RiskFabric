# Bronze Layer Ingestor (Defunct)

**Status:** Superseded as of the [Storage Architecture Split](../decisions/storage_architecture_split.md) decision. The ingestor binary (`src/bin/ingest.rs`) and the `src/clickhouse/` module have been removed.

**What it was:** A binary that read Parquet output from `generate.rs` and bulk-inserted it into ClickHouse Bronze-layer tables via the native `clickhouse` Rust client. It was the bridge between file-system generation and ClickHouse-based ETL.

**Why removed:** The batch ETL path was refactored to a Parquet-only pipeline. Generation writes to `data/output/`, and the ETL reads Parquet directly — no ClickHouse ingestion step needed. ClickHouse remains only for the live streaming `fraud_scores` path.

**Full documentation:** See [ClickHouse Batch ETL Architecture (Defunct)](defunct/clickhouse_batch_etl.md) for the complete architecture that this ingestor was part of.
