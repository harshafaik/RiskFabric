# Summary

- [Welcome](index.md)
- [Documentation Style Guide](style_guide.md)

- [Tutorials](tutorial_first_run.md)

- [How-to Guides](guides_index.md)
    - [Project Roadmap](to-do.md)
    - [Add a New Fraud Signature](how_to_add_fraud.md)

- [Simulation & Generation](simulation_index.md)
    - [Batch Generator (generate.rs)](components/generate_rs.md)
    - [Streaming Generator (stream.rs)](components/stream_rs.md)
    - [Population Generator (customer_gen.rs)](components/population_generator.md)
    - [Financial Entity Linking (account_gen.rs & card_gen.rs)](components/financial_entities.md)
    - [Simulation Engine (transaction_gen.rs)](components/transaction_engine.md)
    - [Adversary Logic Engine (fraud.rs)](components/adversary_logic.md)
    - [Central Configuration Engine (config.rs)](components/config_engine.md)

- [Data & Engineering](engineering_index.md)
    - [ETL Pipeline System (etl.rs)](components/etl_system.md)
    - [Behavioral Feature Engineering (src/etl/features/)](components/behavioral_features.md)
    - [Physical World Transformation (warehouse/)](components/dbt_models.md)
    - [Data Warehouse Ingestor (ingest.rs)](components/ingestor_rs.md)
    - [Reference Data Preparator (prepare_refs.rs)](components/reference_preparator.md)
    - [Reference Data Exporter (export_references.rs)](components/reference_exporter.md)
    - [Reference Data Pipeline (dlt/pipelines.py)](components/dlt_pipeline.md)

- [Machine Learning Systems](ml_systems_index.md)
    - [Training Pipeline (train_xgboost.py)](components/ml_training.md)
    - [Real-Time Scoring Service (scorer.py)](components/realtime_scorer.md)
    - [Model Metadata Utility (dump_model.py)](components/model_metadata.md)

- [Infrastructure & Operations](infrastructure_index.md)
    - [Infrastructure & Local Stack (docker-compose.yml)](components/infrastructure.md)
    - [Redis Feature Seeder (seed_redis.py)](components/redis_seeder.md)

- [Technical Reference](reference_index.md)
    - [Synthetic Data Schema](data_schema.md)
    - [ETL & Feature Schema](etl_schema.md)
    - [Configuration Reference](config_reference.md)
    - [Developer Utilities CLI](developer_utilities.md)
    - [Machine Learning Pipeline](machine_learning.md)

- [Conceptual Explanations](concepts_index.md)
    - [Theory of Operation](theory_of_operation.md)
    - [Fraud Signatures & Attack Patterns](fraud_signatures.md)
    - [Synthetic Fraud Profiles](fraud_profiles.md)
    - [Data Warehouse & dbt Strategy](data_warehouse.md)
    - [Project Goals](objectives.md)

- [Results & Monitoring](results_index.md)
    - [Machine Learning Metrics](ml_metrics.md)
    - [Generation Performance](performance_benchmarks.md)
    - [ETL Performance Optimizations](optimizations.md)

- [Knowledge Base](knowledge_index.md)
    - [Technical Issues & Resolutions](issues.md)
