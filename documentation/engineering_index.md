# Data Engineering & Warehouse

This section documents the ETL pipelines, warehouse ingestion utilities, and geographic reference preparation tools used to build the RiskFabric environment.

## Rust Core Engine (`src/`)

```
config.rs (YAML Config Loader)
          │
          ▼
┌─────────────────────────────────┐
│  📦 Library Modules             │
│                                 │
│  generators/                    │
│    Customer, Account, Card,     │
│    Transaction + Fraud          │
│                                 │
│  models/                        │
│    Data Structures              │
│    + FraudMetadata              │
│                                 │
│  etl/                           │
│    Bronze → Silver →            │
│    Gold (7 stages)              │
│                                 │
│  pipeline/                      │
│    Runner, Events,              │
│    Stream Handle                │
│                                 │
│  summary/                       │
│    Parquet + CH Stats           │
└───────────────┬─────────────────┘
                │  (uses)
                ▼
┌─────────────────────────────────┐
│  ⚡ CLI Binaries                 │
│                                 │
│  generate.rs    Batch → parquet │
│  stream.rs      Kafka → Redpanda│
│  etl.rs         ETL subcommands │
│  prepare_refs   OSM → PostGIS   │
│  export_refs    dbt → Parquet   │
└─────────────────────────────────┘
```

## Modules

- [Feature Engineering Pipeline](components/etl_system.md)
- [Geospatial Reference Pipeline](components/dbt_models.md)
- [OSM Reference Extractor](components/reference_preparator.md)

