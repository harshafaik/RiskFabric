# Conceptual Explanations

High-level documentation explaining the underlying philosophy, architectural strategies, and simulation logic of RiskFabric.

## System Architecture Overview

```mermaid
flowchart TB
    classDef script fill:#22252a,stroke:#4d535b,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9
    classDef store fill:#1b2a3a,stroke:#378ADD,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9
    classDef config fill:#182d24,stroke:#1D9E75,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9
    classDef stream fill:#2e1f26,stroke:#D4537E,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9
    classDef ui fill:#251e36,stroke:#7F77DD,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9
 
    subgraph WB["World Building"]
        OSM["OSM PBF"]:::store --> PREP["prepare_refs.rs"]:::script --> PG[("Postgres / PostGIS")]:::store --> DBT["dbt models<br/>(ST_Intersects spatial joins)"]:::script --> REF[("Reference Parquet")]:::store
    end
 
    subgraph SIM["Simulation Engine"]
        YAML["YAML Configs"]:::config --> CUST["customer_gen.rs"]:::script --> ACC["account_gen / card_gen"]:::script --> TXN["transaction_gen.rs"]:::script
        TXN --> BATCH["Batch Output"]:::script
        TXN --> STREAM["Stream Generator"]:::script
    end
 
    subgraph ETL["Data Pipeline"]
        BRONZE[("Bronze Parquet")]:::store --> ET["etl.rs"]:::script --> SILVER[("Silver Parquet")]:::store
    end
 
    subgraph ML["ML Training & Scoring"]
        GOLD[("Gold Parquet")]:::store --> TRAIN["train_xgboost.py"]:::script --> MODEL[("fraud_model_v4.json")]:::store
        STREAM --> KAFKA[("Redpanda")]:::stream --> SCORER["scorer.py"]:::script
        SEED["redis_seeder.py"]:::script --> REDIS[("Redis")]:::store --> SCORER
        MODEL --> SCORER
        SCORER --> SCORES[("ClickHouse Scores")]:::store
    end
 
    subgraph CM["Case Management"]
        INGEST["ingest_cases.py"]:::script --> OLTP[("Postgres OLTP")]:::store --> DJANGO["Django Admin"]:::ui
    end
 
    REF --> CUST
    BATCH --> BRONZE
    SILVER --> GOLD
    SCORES --> INGEST
 
    style WB fill:#26231b,stroke:#EF9F27,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
    style SIM fill:#1c241e,stroke:#1D9E75,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
    style ML fill:#28201b,stroke:#7F77DD,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
    style ETL fill:#231e2d,stroke:#D85A30,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
    style CM fill:#1c2423,stroke:#7F77DD,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
```

**<a id="fig-12"></a>Figure 12:** System Architecture Overview

## Modules

- [Theory of Operation](theory_of_operation.md)
- [Fraud Signatures & Attack Patterns](fraud_signatures.md)
