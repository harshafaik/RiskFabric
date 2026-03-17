# Configuration Reference

## Summary
The `config_reference.md` document provide a catalog of the behavioral parameters and system-wide settings available in RiskFabric. It details the schema of the YAML configuration files that define the simulation's behavioral rules, ranging from geographic boundaries to fraud injection rates.

## Design Intent
The configuration system is designed to be **Hierarchical and Domain-Specific**. By splitting settings into five distinct YAML files, researchers can perform comparative testing on simulation behaviors (e.g., comparing different fraud population densities) by swapping configuration files. This decoupling ensures the generator can be tuned without recompiling the Rust binaries.

A critical design choice was the use of **Semantic Weights**. For parameters such as `hourly_weights` and `daily_weights`, relative values are used rather than absolute probabilities. This allows the generator to maintain consistent behavioral ratios (e.g., temporal activity peaks) regardless of the total volume of generated data.

---

## 📄 Core Configuration Files

### `fraud_rules.yaml`
Defines the individual attack profiles and their behavioral biases.
- **`profiles`**: Mapping of fraud types (e.g., `upi_scam`) to amount strategies and geographic anomaly probabilities.
- **`fraud_patterns`**: List of common "test amounts" used by attackers for card validation.

### `customer_config.yaml`
Defines the synthetic population's physical and economic footprint.
- **`control.customer_count`**: Total population size for the batch generation run.
- **`financials.base_spend`**: Expected monthly expenditure per location type (Metro, Urban, Rural).

### `transaction_config.yaml`
Defines the "physics" of the transaction stream.
- **`geo_bounds`**: The lat/long bounding box for transaction events.
- **`temporal_patterns`**: The weighted distribution of activity across the 24-hour day and 7-day week.

---

## Known Issues
The **Lookback Period** (`lookback_days`) can currently be set independently of the customer registration window. This allows for temporal inconsistencies where transaction history precedes a customer's registration date. Implementing cross-configuration validation is necessary to ensure temporal consistency.

Furthermore, the **Streaming Rate** (`streaming_rate`) is a global setting. "Dynamic Throughput," which would allow the generator to simulate peak activity hours (e.g., varying tx/s by time of day), is not yet implemented. Modifying the streaming engine to respect the temporal weights defined in `transaction_config.yaml` is required to create more realistic real-time traffic patterns.
