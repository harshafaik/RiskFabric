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
    classDef defaultNode fill:#22252a,stroke:#4d535b,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    
    subgraph WB["🌍 World Building"]
        PR["reference_preparator"]:::defaultNode
        DBT["dbt_models"]:::defaultNode
        PR --> DBT
    end

    subgraph CFG_GRP["⚙️ Configuration"]
        CFG["central_configuration_engine"]:::defaultNode
    end

    subgraph GEN["⚡ Generators"]
        CG["customer_generator"]:::defaultNode
        AG["account_and_card_generator"]:::defaultNode
        TG["transaction_generator"]:::defaultNode
        ALE["adversary_logic_engine"]:::defaultNode
        BG["batch_generator"]:::defaultNode
        SG["streaming_generator"]:::defaultNode
    end

    subgraph DE["🏛️ Data Engineering"]
        ING["ingestor_rs"]:::defaultNode
        ETL["etl_system"]:::defaultNode
    end

    subgraph ML_GRP["🧠 ML"]
        MT["ml_training"]:::defaultNode
        MC["ml_calibration"]:::defaultNode
        MEX["ml_explainability"]:::defaultNode
        MRB["ml_robustness"]:::defaultNode
        RS["redis_seeder"]:::defaultNode
        RTS["realtime_scorer"]:::defaultNode
    end

    subgraph INFRA["☁️ Infrastructure"]
        INF["infrastructure"]:::defaultNode
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
