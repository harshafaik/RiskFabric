# Population Generator (`customer_gen.rs`)

## Summary
The `customer_gen.rs` module is responsible for the foundational entity creation in the RiskFabric simulation. It generates a synthetic population of customers by synthesizing demographics, geographic data from OpenStreetMap (OSM) reference points, and financial behavioral profiles. This module ensures that every customer is "anchored" to a realistic physical and economic context.

## Architectural Decisions
This generator is designed around a **Constraint-Based Synthetic Model**. Instead of simple randomization, the engine enforces correlations across different entity domains. For example, it programmatically links Credit Score to Age (using an `age_weight` factor) and Monthly Spend to Location Type (Metro vs. Rural). This ensures that the resulting dataset possesses the structural patterns expected in real-world financial data.

For geographic fidelity, a **Spatial Jittering** strategy is implemented. By adding a ~500m drift (`0.005` degrees) to the original OSM residential nodes, the simulation avoids "clumping" effects where multiple customers would otherwise share identical coordinates. This jittering preserves the overall density of the reference data while providing unique home coordinates for every agent. Note that while transaction-level jitter is deterministic, the initial population jitter is currently stochastic.

The generator uses **Probabilistic Location Typing** to classify customers into Metro, Urban, or Rural categories based on their proximity to city centers in the reference data. This classification serves as the primary driver for the financial heuristics used in the simulation.

## System Integration
`customer_gen.rs` acts as the first stage of the generation pipeline. It consumes the `ref_residential.parquet` file and the `customer_config.yaml` configuration to produce a vector of `Customer` structs. This vector is passed downstream to the account and card generators to complete the entity hierarchy.

## Known Issues
The entire residential reference dataset is currently loaded into memory using Polars' `ParquetReader` for every generation run. While efficient for populations up to 100,000 customers, this creates a significant memory bottleneck when scaling to millions of agents. Moving to a chunked or streaming approach for reading reference data is required. Additionally, the jitter range (0.005) is currently hardcoded in the source code; moving this to the configuration would allow for different levels of spatial precision.
