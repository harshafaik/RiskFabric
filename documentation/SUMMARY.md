# Summary

- [Welcome](index.md)

- [Data Generation Modules](simulation_index.md)
    - [Customer Data Generator](components/customer_generator.md)
    - [Account and Card Data Generator](components/account_and_card_generator.md)
    - [Transactional Data Generator](components/transaction_generator.md)
    - [Batch Orchestrator](components/batch_generator.md)
    - [Streaming Orchestrator](components/streaming_generator.md)
    - [Fraud Injector](components/adversary_logic_engine.md)
    - [Configuration Module](components/configuration.md)

- [Data & Engineering](engineering_index.md)
    - [Feature Engineering Pipeline](components/etl_system.md)
    - [Geospatial Reference Pipeline](components/dbt_models.md)
    - [OSM Reference Extractor](components/reference_preparator.md)

- [Machine Learning Systems](ml_systems_index.md)
    - [Training Pipeline](components/ml_training.md)
    - [Model Calibration](components/ml_calibration.md)
    - [Real-Time Scoring Service](components/realtime_scorer.md)
    - [Model Explainability (SHAP)](components/ml_explainability.md)
    - [Model Robustness & Drift](components/ml_robustness.md)
    - [Analysis & Utility Scripts](ml_analysis_scripts.md)

- [Infrastructure & Operations](infrastructure_index.md)
    - [Infrastructure & Local Stack](components/infrastructure.md)
    - [Django Case Management UI](components/case_admin.md)
    - [Redis Feature Seeder](components/redis_seeder.md)

- [Conceptual Explanations](concepts_index.md)
    - [Theory of Operation](theory_of_operation.md)
    - [Fraud Signatures & Attack Patterns](fraud_signatures.md)
    - [Diagram Glossary](diagrams_glossary.md)

- [Results & Monitoring](results_index.md)
    - [Machine Learning Metrics](ml_metrics.md)
    - [Feature Leakage Case Study](feature_leakage_issues.md)
    - [Performance Benchmarks](performance.md)

- [Defunct Implementations](defunct_index.md)
    - [ETL Pipeline System (Shell-Pivot Variant)](components/defunct/etl_system_shell_pivot.md)
    - [ClickHouse Batch ETL Architecture](components/defunct/clickhouse_batch_etl.md)

- [Decisions](decisions_index.md)
    - [IP and Device Reputation Removal](decisions/ip_device_reputation_removal.md)
    - [Storage Architecture Split](decisions/storage_architecture_split.md)
    - [Deployment Architecture](decisions/deployment_architecture.md)
    - [Fraud Operations Cost Model](decisions/fraud_operations_cost_model.md)
    - [Calibration Gap](decisions/calibration_gap.md)
