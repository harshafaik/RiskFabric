# Data Warehouse & dbt Strategy

## Summary
The `data_warehouse.md` document outlines the architectural strategy for RiskFabric's analytical layer. It explains how raw synthetic data is transformed into high-fidelity behavioral entities using a Modern Data Stack (MDS) approach, specifically leveraging ClickHouse for high-volume transactions and Postgres/dbt for geographic enrichment.

## Design Intent
The warehouse functions as a **Medallion Data Lakehouse**, intended to demonstrate how synthetic data can be used to test both machine learning models and the data engineering lifecycle. By using **dbt (data build tool)**, complex geographic filtering and merchant risk assignment are implemented in SQL, allowing for a clear separation between the simulation engine (Rust) and the analytical environment (SQL).

A critical architectural decision was the adoption of a **Dual-Warehouse Model**. ClickHouse serves as the primary engine for transaction data due to its performance with columnar storage and large-scale joins. Conversely, Postgres is used for "Level 0" geographic preparation (OSM extraction), as it provides mature support for spatial extensions like PostGIS. This approach ensures each part of the simulation utilizes the tool best suited for its specific data type.

---

## 🏗️ Warehouse Layers
1.  **Bronze (Raw)**: Direct ingest from Parquet files via `ingest.rs`.
2.  **Silver (Enriched)**: Entity-level behavioral features (e.g., `customer_features_silver`).
3.  **Gold (Master)**: The flattened, model-ready `fact_transactions_gold`.

---

## Known Issues
The system currently utilizes **Podman-based container execution** to interact with the warehouse from the Rust binaries. This introduces environment-level fragility and limits the simulation's scalability in distributed cloud environments. Transitioning to native ClickHouse and Postgres client libraries is necessary to improve the reliability of the ingestion and transformation stages.

Furthermore, **dbt models** are split between two different databases (Postgres for references, ClickHouse for transactions). This prevents cross-warehouse joins and requires moving data via Parquet files. Unifying the transformation layer—specifically by moving all "Level 0" geography data into ClickHouse—is required to eliminate manual data-movement steps and simplify dbt pipeline orchestration.
