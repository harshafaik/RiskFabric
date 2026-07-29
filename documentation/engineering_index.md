# Data Engineering & Warehouse

This section documents the ETL pipelines, warehouse ingestion utilities, and geographic reference preparation tools used to build the RiskFabric environment.

## Rust Core Engine (`src/`)

```mermaid
flowchart LR
    classDef script fill:#22252a,stroke:#4d535b,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    classDef config fill:#182d24,stroke:#2b5443,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    CONFIG["config.rs"]:::config
    CONFIG --> GEN["generators"]:::script
    CONFIG --> MOD["models"]:::script
    CONFIG --> ETL["etl"]:::script
    CONFIG --> PIPE["pipeline"]:::script
    GEN --> BATCH["generate.rs"]:::script
    PIPE --> STREAM["stream.rs"]:::script
    ETL --> ETL_BIN["etl.rs"]:::script
    BATCH --> PREP["prepare_refs / export_refs"]:::script
```

**<a id="fig-4"></a>Figure 4:** Rust Core Engine Module Map

## Modules

- [Feature Engineering Pipeline](components/etl_system.md)
- [Geospatial Reference Pipeline](components/dbt_models.md)
- [OSM Reference Extractor](components/reference_preparator.md)

