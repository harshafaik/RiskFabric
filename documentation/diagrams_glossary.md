# Diagram Glossary
Comprehensive index of all 17 figures in the RiskFabric documentation, grouped by domain. Each figure title links directly to the diagram in its source file.
## Summary
| Figure | Title | File | Type |
|--------|-------|------|------|
| 1 | [System Pipeline](index.md#fig-1) | `index.md` | Flowchart |
| 2 | [Customer Entity Schema](components/customer_generator.md#fig-2) | `components/customer_generator.md` | ER Diagram |
| 3 | [Streaming Generator Data Flow](components/streaming_generator.md#fig-3) | `components/streaming_generator.md` | Flowchart |
| 4 | [Rust Core Engine Module Map](engineering_index.md#fig-4) | `engineering_index.md` | Flowchart |
| 5 | [Feature Engineering ETL Schema](components/etl_system.md#fig-5) | `components/etl_system.md` | ER Diagram |
| 6 | [OSM Reference Extraction Pipeline](components/reference_preparator.md#fig-6) | `components/reference_preparator.md` | Flowchart |
| 7 | [Python ML Pipeline](ml_systems_index.md#fig-7) | `ml_systems_index.md` | Flowchart |
| 8 | [Infrastructure Service Stack](components/infrastructure.md#fig-8) | `components/infrastructure.md` | Flowchart |
| 9 | [Case Admin Interface](components/case_admin.md#fig-9) | `components/case_admin.md` | Screenshot |
| 10 | [Case Management Entity Schema](components/case_admin.md#fig-10) | `components/case_admin.md` | ER Diagram |
| 11 | [Redis Feature Seeder Flow](components/redis_seeder.md#fig-11) | `components/redis_seeder.md` | Flowchart |
| 12 | [System Architecture Overview](concepts_index.md#fig-12) | `concepts_index.md` | Flowchart |
| 13 | [Entity Lifecycle Schema](theory_of_operation.md#fig-13) | `theory_of_operation.md` | ER Diagram |
| 14 | [Grafana Fraud Monitoring Dashboard](results_index.md#fig-14) | `results_index.md` | Screenshot |
| 15 | [ClickHouse Batch ETL Architecture (Defunct)](components/defunct/clickhouse_batch_etl.md#fig-15) | `components/defunct/clickhouse_batch_etl.md` | Flowchart |
| 16 | [Storage Architecture Split — Parquet Pipeline](decisions/storage_architecture_split.md#fig-16) | `decisions/storage_architecture_split.md` | Flowchart |
| 17 | [Component Dependency Map](components/index.md#fig-17) | `components/index.md` | Flowchart |
## Architecture & System Overview
**[Figure 1 — System Pipeline](index.md#fig-1)**
- **File:** `documentation/index.md`
- **Type:** Flowchart (LR)
- **Description:** Compact left-to-right overview of the five major subsystems: World Building, Simulation Engine, Data Pipeline, ML Training & Scoring, and Case Management.
- **Key nodes:** `World Building`, `Simulation Engine`, `Data Pipeline`, `ML Training & Scoring`, `Case Management`
- **Purpose:** Landing-page diagram — gives first-time readers a one-glance understanding of the end-to-end data flow before diving into subsystem details.
**[Figure 12 — System Architecture Overview](concepts_index.md#fig-12)**
- **File:** `documentation/concepts_index.md`
- **Type:** Flowchart (TB)
- **Description:** High-level system architecture showing how the five subsystems connect: World Building, Simulation Engine, ML Training & Scoring (top row), and Data Pipeline, Case Management (bottom row). Cross-row edges show data flow from batch output to ETL and from reference data to customer generation.
- **Key nodes:** `prepare_refs.rs`, `customer_gen.rs`, `train_xgboost.py`, `scorer.py`, `etl.rs`, `ingest_cases.py`, `Django Admin`
- **Purpose:** Onboarding overview of the full pipeline from OSM data to case review.
**[Figure 4 — Rust Core Engine Module Map](engineering_index.md#fig-4)**
- **File:** `documentation/engineering_index.md`
- **Type:** Flowchart (LR)
- **Description:** Module dependency graph of the Rust core engine: `config.rs` feeds `generators`, `models`, `etl`, and `pipeline` modules, which in turn drive `generate.rs`, `stream.rs`, `etl.rs`, and `prepare_refs.rs`.
- **Key nodes:** `config.rs`, `generators`, `models`, `etl`, `pipeline`, `generate.rs`, `stream.rs`, `etl.rs`
- **Purpose:** Entry point for the Data Engineering & Warehouse documentation section.
**[Figure 8 — Infrastructure Service Stack](components/infrastructure.md#fig-8)**
- **File:** `documentation/components/infrastructure.md`
- **Type:** Flowchart (TB)
- **Description:** Docker Compose service topology showing host-to-container and container-to-container communication. Redpanda, Redis, ClickHouse, Postgres, scorer, Grafana, and Django case admin with port mappings and data flow labels.
- **Key nodes:** `stream.rs` (host), `Redpanda`, `Redis`, `ClickHouse`, `scorer.py`, `Grafana`, `case_admin`
- **Purpose:** Explains how the 7 Docker services interconnect for local development.
**[Figure 9 — Case Admin Interface](components/case_admin.md#fig-9)**
- **File:** `documentation/components/case_admin.md`
- **Type:** Screenshot
- **Description:** Django admin list view of the Case model showing case IDs, fraud scores, status badges, reviewer assignments, and flag reason indicators.
- **Purpose:** Shows the investigator-facing UI for the case management workflow.
**[Figure 16 — Storage Architecture Split — Parquet Pipeline](decisions/storage_architecture_split.md#fig-16)**
- **File:** `documentation/decisions/storage_architecture_split.md`
- **Type:** Flowchart (TB)
- **Description:** Side-by-side comparison of the batch Parquet pipeline (Bronze → Polars → Silver → DuckDB → Gold → training) and the streaming pipeline (stream.rs → Redpanda → scorer.py → ClickHouse → Grafana). Documents the architectural decision to remove ClickHouse from the batch path.
- **Key nodes:** `generate.rs`, `Polars feature engineering`, `DuckDB`, `train_xgboost.py`, `Redpanda`, `scorer.py`, `ClickHouse`, `Grafana`
- **Purpose:** Architecture decision record showing the storage split between batch (Parquet) and streaming (ClickHouse).
**[Figure 17 — Component Dependency Map](components/index.md#fig-17)**
- **File:** `documentation/components/index.md`
- **Type:** Flowchart (TD)
- **Description:** Dependency graph of every codebase component organized into six subsystems: World Building, Configuration, Generators, Data Engineering, ML, and Infrastructure. Shows how configuration drives generators, how generators feed ETL, and how training outputs feed the realtime scorer.
- **Key nodes:** `reference_preparator`, `customer_generator`, `transaction_generator`, `adversary_logic_engine`, `etl_system`, `ml_training`, `realtime_scorer`, `infrastructure`
- **Purpose:** Navigation map for the components section — shows which docs cover which modules.
## Data Schemas & Entity Relationships
**[Figure 2 — Customer Entity Schema](components/customer_generator.md#fig-2)**
- **File:** `documentation/components/customer_generator.md`
- **Type:** ER Diagram
- **Description:** Entity schema for the synthetic customer with three embedded sub-profiles: `GeoLocation` (H3-indexed coordinates), `FinancialProfile` (credit score, spend limits), and `DeviceProfile` (user agents, ISP, IP subnet).
- **Key entities:** `Customer`, `GeoLocation`, `FinancialProfile`, `DeviceProfile`
- **Purpose:** Documents the customer data model generated by `customer_gen.rs` from OSM reference data and YAML configs.
**[Figure 5 — Feature Engineering ETL Schema](components/etl_system.md#fig-5)**
- **File:** `documentation/components/etl_system.md`
- **Type:** ER Diagram
- **Description:** Medallion architecture schema: `customer_features_silver` and `merchant_features_silver` join against `fact_transactions_silver`, producing the training-ready `fact_transactions_gold` snapshot.
- **Key entities:** `customer_features_silver`, `merchant_features_silver`, `fact_transactions_silver`, `fact_transactions_gold`
- **Purpose:** Documents the Bronze → Silver → Gold Parquet pipeline and the join keys used for feature union.
**[Figure 10 — Case Management Entity Schema](components/case_admin.md#fig-10)**
- **File:** `documentation/components/case_admin.md`
- **Type:** ER Diagram
- **Description:** Django ORM entity schema: `auth_User` (reviewer) has many `Case` records, each with transaction ID, fraud score, status, reviewer assignment, notes, and SHAP flag reasons.
- **Key entities:** `auth_User`, `Case`
- **Purpose:** Documents the OLTP case management data model consumed by the Django admin interface.
**[Figure 13 — Entity Lifecycle Schema](theory_of_operation.md#fig-13)**
- **File:** `documentation/theory_of_operation.md`
- **Type:** ER Diagram
- **Description:** Entity-relationship diagram showing the deterministic creation lifecycle: Customer → Account → Card → Transaction → Merchant, with FraudMetadata attached to transactions containing fraud labels.
- **Key entities:** `Customer`, `Account`, `Card`, `Transaction`, `Merchant`, `FraudMetadata`
- **Purpose:** Illustrates the agent-based simulation philosophy — one-pass architecture that mirrors real banking hierarchies.
## Data Flow & Pipelines
**[Figure 3 — Streaming Generator Data Flow](components/streaming_generator.md#fig-3)**
- **File:** `documentation/components/streaming_generator.md`
- **Type:** Flowchart (LR)
- **Description:** Shows how `Transaction` structs are stripped of labels at the type level into `UnlabeledTransaction`, published to the `raw_transactions` Kafka topic, with ground truth captured to `ground_truth.csv` during verification mode.
- **Key nodes:** `Transaction`, `UnlabeledTransaction`, `raw_transactions` (Kafka), `ground_truth.csv`
- **Purpose:** Documents the label-leakage prevention design — the Kafka payload is guaranteed label-free by the type system.
**[Figure 6 — OSM Reference Extraction Pipeline](components/reference_preparator.md#fig-6)**
- **File:** `documentation/components/reference_preparator.md`
- **Type:** Flowchart (LR)
- **Description:** Three-stage pipeline: `prepare_refs.rs` extracts OSM PBF into Postgres raw tables → dbt models transform into staging/mart layers → `export_references.rs` writes reference Parquet files for downstream generators.
- **Key nodes:** `India OSM PBF`, `raw_residential / raw_merchants / raw_financial`, `stg_residential / stg_merchants`, `mart_residential / mart_merchants`, `ref_residential.parquet / ref_merchants.parquet`
- **Purpose:** Documents the full extract-transform-export chain for geographic reference data.
**[Figure 7 — Python ML Pipeline](ml_systems_index.md#fig-7)**
- **File:** `documentation/ml_systems_index.md`
- **Type:** Flowchart (TB)
- **Description:** End-to-end ML pipeline organized into Training (XGBoost → calibration → threshold optimization → SHAP analysis → model serialization), Serving (Redis seeding → Kafka scorer → case ingestion), and Batch utilities (large-scale generation, combined generation+scoring).
- **Key nodes:** `train_xgboost.py`, `calibrate_model.py`, `compute_thresholds.py`, `shap_analysis.py`, `dump_model.py`, `seed_redis.py`, `scorer.py`, `ingest_cases.py`
- **Purpose:** Index map for the Machine Learning Systems documentation section.
**[Figure 11 — Redis Feature Seeder Flow](components/redis_seeder.md#fig-11)**
- **File:** `documentation/components/redis_seeder.md`
- **Type:** Flowchart (LR)
- **Description:** Gold Parquet snapshot feeds `seed_redis.py` via DuckDB queries, which writes per-card, per-customer, and per-merchant keys to Redis. The scorer reads these keys for warm-start feature computation.
- **Key nodes:** `fact_transactions_gold` (Parquet), `seed_redis.py`, `Redis`, `scorer.py`
- **Purpose:** Documents the warm-start mechanism — without seeding, the first streaming transaction per entity produces degenerate zero-valued features.
## Screenshots
**[Figure 9 — Case Admin Interface](components/case_admin.md#fig-9)**
- **File:** `documentation/components/case_admin.md`
- **Type:** Screenshot
- **Description:** Django admin list view of the Case model showing case IDs, fraud scores, status badges, reviewer assignments, and flag reason indicators.
- **Purpose:** Shows the investigator-facing UI for the case management workflow.
**[Figure 14 — Grafana Fraud Monitoring Dashboard](results_index.md#fig-14)**
- **File:** `documentation/results_index.md`
- **Type:** Screenshot
- **Description:** Grafana dashboard displaying real-time fraud score distributions, transaction volume trends, and alert summaries from ClickHouse data sources.
- **Purpose:** Illustrates the operational monitoring view for fraud ops teams.
## Defunct / Historical
**[Figure 15 — ClickHouse Batch ETL Architecture (Defunct)](components/defunct/clickhouse_batch_etl.md#fig-15)**
- **File:** `documentation/components/defunct/clickhouse_batch_etl.md`
- **Type:** Flowchart (TD)
- **Description:** The superseded architecture where ClickHouse served as the central analytical store for both batch ETL and streaming. Shows the wasteful Parquet → ClickHouse → Parquet round-trip and the merged Bronze/Silver/Gold tables within ClickHouse.
- **Key nodes:** `generate.rs`, `ingest.rs`, `ClickHouse`, `etl.rs`, `Python ML`
- **Purpose:** Historical reference — documents why the architecture was split per the Storage Architecture Split decision. Retained for context when reading old code or discussions.
