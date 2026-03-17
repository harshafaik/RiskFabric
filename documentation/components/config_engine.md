# Central Configuration Engine (`config.rs`)

## Summary
The `config.rs` module is the architectural backbone of RiskFabric. It provides a strongly-typed, unified interface for all behavioral and operational parameters of the simulation. By mapping multiple YAML files into a hierarchical Rust structure, it ensures that every component—from the simulation engine to the machine learning pipeline—operates with a consistent and validated world-view.

## Architectural Decisions
This engine is designed to enforce **Type-Safe Behavioral Modeling**. Instead of using loose key-value pairs or dynamic JSON, a deep hierarchy of nested structs is implemented. This leverages Rust’s compiler to ensure that any change to the configuration schema in one part of the system is immediately reflected and validated in every other part. 

The use of **Atomic Multi-File Loading** is a critical architectural decision. The `AppConfig::load()` method reads five separate YAML files (`fraud_rules`, `fraud_tuning`, `customer_config`, `transaction_config`, and `product_catalog`) and synthesizes them into a single `AppConfig` object. This separation of concerns allows specific domains (like "Product Catalog" or "Fraud Rules") to be tuned in isolation without creating massive, unmanageable configuration files.

**Safety Defaults** are also implemented using `serde` macros. This ensures that the simulation remains resilient even if the underlying YAML files are missing non-essential keys, providing sensible fallbacks for parameters like the `streaming_rate`.

## System Integration
`config.rs` is widely consumed across the codebase. It is initialized at the entry point of every binary (`generate`, `stream`, `etl`, `ingest`) and is passed down into the generators as a shared reference. This ensures that the "rules of the world" are identical across the batch, streaming, and ETL layers.

## Known Issues
`fs::read_to_string` and `expect` calls are currently used in the `load()` method. This causes the application to panic immediately if a config file is missing or contains a syntax error. While acceptable for a CLI tool, refactoring to return a `Result` type is required to allow for more graceful error handling and reporting. Additionally, the file paths for the YAML configs are currently hardcoded relative to the project root; a more flexible path resolution strategy is needed to allow RiskFabric to be executed from different directories.
