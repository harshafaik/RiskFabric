# Codebase Components

This section provides a detailed technical breakdown of the individual modules, engines, and utilities that make up the RiskFabric simulation environment. Each document explains the architectural intent and system integration of a specific file or directory.

## Component Dependency Map

```mermaid
%%{init: {
  'themeVariables': {
    'fontFamily': '"JetBrains Mono", monospace'
  }
}}%%
flowchart TD
    %% Node Class Definitions
    classDef component fill:#22252a,stroke:#4d535b,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    
    subgraph WB["🌍 World Building"]
        PR["reference_preparator"]:::component
        DBT["dbt_models"]:::component
        PR --> DBT
    end

    subgraph CFG_GRP["⚙️ Configuration"]
        CFG["central_configuration_engine"]:::component
    end

    subgraph GEN["⚡ Generators"]
        CG["customer_generator"]:::component
        AG["account_and_card_generator"]:::component
        TG["transaction_generator"]:::component
        ALE["adversary_logic_engine"]:::component
        BG["batch_generator"]:::component
        SG["streaming_generator"]:::component
    end

    subgraph DE["🏛️ Data Engineering"]
        ING["ingestor_rs"]:::component
        ETL["etl_system"]:::component
    end

    subgraph ML_GRP["🧠 ML"]
        MT["ml_training"]:::component
        MC["ml_calibration"]:::component
        MEX["ml_explainability"]:::component
        MRB["ml_robustness"]:::component
        RS["redis_seeder"]:::component
        RTS["realtime_scorer"]:::component
    end

    subgraph INFRA["☁️ Infrastructure"]
        INF["infrastructure"]:::component
    end

    DBT --> CG
    CFG --> CG & AG & TG & ALE & BG & SG
    CG --> AG --> TG
    ALE --> TG
    TG --> BG & SG
    BG --> ING --> ETL
    SG --> RTS
    ETL --> MT
    MT --> MC --> RTS
    MC --> MEX & MRB
    RS --> RTS

    %% Subgraph Styling
    style WB fill:#26231b,stroke:#474130,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9;
    style CFG_GRP fill:#1e232e,stroke:#333e54,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9;
    style GEN fill:#1c241e,stroke:#304033,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9;
    style DE fill:#231e2d,stroke:#3f3354,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9;
    style ML_GRP fill:#28201b,stroke:#4c392c,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9;
    style INFRA fill:#1c2423,stroke:#2e403d,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9;

    linkStyle default stroke-width:1px,stroke:#5c687a
```

**<a id="fig-17"></a>Figure 17:** Component Dependency Map

## Modules

- [Customer Data Generator](customer_generator.md)
- [Account and Card Data Generator](account_and_card_generator.md)
- [Transactional Data Generator](transaction_generator.md)
- [Batch Orchestrator](batch_generator.md)
- [Streaming Orchestrator](streaming_generator.md)
- [Fraud Injector](adversary_logic_engine.md)
- [Configuration Module](configuration.md)
- [Feature Engineering Pipeline](etl_system.md)
- [Geospatial Reference Pipeline](dbt_models.md)
- [OSM Reference Extractor](reference_preparator.md)
- [Ingestor Service](ingestor_rs.md)
- [Training Pipeline](ml_training.md)
- [Model Calibration](ml_calibration.md)
- [Real-Time Scoring Service](realtime_scorer.md)
- [Model Explainability (SHAP)](ml_explainability.md)
- [Model Robustness & Drift](ml_robustness.md)
- [Infrastructure & Local Stack](infrastructure.md)
- [Django Case Management UI](case_admin.md)
- [Redis Feature Seeder](redis_seeder.md)

