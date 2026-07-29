# Batch Orchestrator

## Overview
The batch generator module `generate.rs` serves as the primary orchestration engine for synthetic datasets.

## Schema

The entity schema follows the canonical [entity model](../theory_of_operation.md#2-the-deterministic-lifecycle) (Customer → Account → Card → Transaction → FraudMetadata). The orchestrator outputs five primary relational tables:
<details>
<summary>Output tables</summary>

| File Name | Primary Keys / Foreign Keys | Description |
| :--- | :--- | :--- |
| `customers.parquet` | `customer_id` | Labeled customer profile data, including demographic, geographical, and risk profile columns. |
| `accounts.parquet` | `account_id`, `customer_id` | Relational deposit and credit account profiles. |
| `cards.parquet` | `card_id`, `account_id`, `customer_id` | Physical and virtual card payment instruments linked to accounts. |
| `transactions.parquet` | `transaction_id`, `card_id` | Labeled transaction events stream. |
| `fraud_metadata.parquet` | `transaction_id` | Injected adversarial mutation metadata and diagnostics labels. |

</details>

The generator uses a **chunked execution strategy** to handle datasets that exceed available system memory. Measured RSS: 3.6 GB at 3,400 customers (1.54M txn), 6.5 GB at 10,000 customers (4.47M txn). The 5,000-card chunks keep per-chunk allocation bounded, but the final Parquet merge materializes the full dataset — memory grows with total volume. [[See benchmarks](performance.md)] For spatial lookups, the system implements a multi-tier H3 index (resolutions 4 and 6) and a state-level index. This allows for rapid, localized merchant selection during transaction generation without exhaustive searching of the merchant reference dataset.

The choice of **Apache Parquet** as the output format ensures that multi-million row datasets remain compressed and performant for the downstream Python-based ML pipeline and Polars-based ETL.

`generate.rs` sits at the start of the RiskFabric lifecycle. It consumes reference Parquet files for merchants and residential locations and produces the four core tables: `customers.parquet`, `accounts.parquet`, `cards.parquet`, and `transactions.parquet` (including its accompanying `fraud_metadata.parquet`).

## Current Limitations

The final merge writes temporary Parquet chunks to disk and re-scans them. While this prevents OOM, it adds disk I/O overhead. The 5,000-card chunk size is hardcoded; moving it to config would allow RAM-based tuning.
