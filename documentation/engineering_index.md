# Data Engineering & Warehouse

This section documents the ETL pipelines, warehouse ingestion utilities, and geographic reference preparation tools used to build the RiskFabric environment.

## Rust Core Engine (`src/`)

```d2
direction: down

classes: {
  head: {
    style.fill: "#1e232e"
    style.stroke: "#333e54"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
  mod: {
    style.fill: "#182d24"
    style.stroke: "#2b5443"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
  bin: {
    style.fill: "#1b2a3a"
    style.stroke: "#304e70"
    style.stroke-width: 1
    style.border-radius: 5
    style.font-color: "#cfd2d9"
  }
  container_box: {
    style.stroke-dash: 3
    style.stroke-width: 1
    style.font-color: "#cfd2d9"
  }
}

CFG: "config.rs\nYAML Config Loader" {
  class: head
}

M: "📦 Library Modules" {
  class: container_box
  style.fill: "#1c241e"
  style.stroke: "#304033"

  GEN: "generators/\nCustomer, Account, Card,\nTransaction + Fraud" {
    class: mod
  }
  MOD: "models/\nData Structures\n+ FraudMetadata" {
    class: mod
  }
  ETL: "etl/\nBronze → Silver →\nGold (7 stages)" {
    class: mod
  }
  PIP: "pipeline/\nRunner, Events,\nStream Handle" {
    class: mod
  }
  SUM: "summary/\nParquet + CH Stats" {
    class: mod
  }
}

B: "⚡ CLI Binaries" {
  class: container_box
  style.fill: "#1c241e"
  style.stroke: "#304033"

  B1: "generate.rs\nBatch → parquet" {
    class: bin
  }
  B2: "stream.rs\nKafka → Redpanda" {
    class: bin
  }
  B3: "etl.rs\nETL subcommands" {
    class: bin
  }
  B4: "prepare_refs.rs\nOSM → PostGIS" {
    class: bin
  }
  B5: "export_references.rs\ndbt → Parquet" {
    class: bin
  }
}

CFG -> M
M -> B: {style.stroke-dash: 3}
```

## Modules

- [Feature Engineering Pipeline](components/etl_system.md)
- [Geospatial Reference Pipeline](components/dbt_models.md)
- [OSM Reference Extractor](components/reference_preparator.md)

