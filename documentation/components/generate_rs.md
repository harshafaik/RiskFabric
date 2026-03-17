# Batch Data Generator (`generate.rs`)

## Summary
The `generate.rs` binary serves as the primary orchestration engine for creating large-scale, labeled synthetic datasets. It generates a complete ecosystem of customers, accounts, cards, and historical transactions, providing the "ground truth" required for training fraud detection models.

## Architectural Decisions
The generator uses a **chunked execution strategy** to handle datasets that exceed available system memory. By processing cards in batches of 5,000, the generator maintains a stable memory profile regardless of the total population size. For spatial lookups, the system implements a multi-tier H3 index (resolutions 4 and 6) and a state-level index. This allows for rapid, localized merchant selection during transaction generation without exhaustive searching of the merchant reference dataset.

The choice of **Apache Parquet** as the output format ensures that multi-million row datasets remain compressed and performant for the downstream Python-based ML pipeline and Polars-based ETL.

## System Integration
`generate.rs` sits at the start of the RiskFabric lifecycle. It consumes reference Parquet files for merchants and residential locations and produces the four core tables: `customers.parquet`, `accounts.parquet`, `cards.parquet`, and `transactions.parquet` (including its accompanying `fraud_metadata.parquet`).

## Known Issues
The final merge phase is implemented by writing temporary Parquet chunks to disk and then re-scanning them with the Polars lazy API. While this prevents memory exhaustion during the final join, it introduces disk I/O overhead that affects the "cleanup" phase of generation. Additionally, the 5,000-card chunk size is currently hardcoded; moving this to `customer_config.yaml` would allow performance tuning based on available RAM capacity.
