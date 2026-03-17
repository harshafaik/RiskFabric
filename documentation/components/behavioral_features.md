# Behavioral Feature Engineering (`src/etl/features/`)

## Summary
The `src/etl/features/` directory contains the core analytical logic of RiskFabric. It defines how raw synthetic transactions are transformed into behavioral features across multiple domains: Customer history, Merchant risk, Transaction sequences, and Network relationships. These features provide the high-dimensional context required for modern fraud detection models to identify subtle adversarial patterns.

## Architectural Decisions
This layer is designed to prioritize **Domain-Specific Modularity**. By separating feature sets into dedicated modules (e.g., `network.rs`, `sequence.rs`), independent iteration on different detection strategies is possible. This modularity ensures that the ETL pipeline can be easily extended with new behavioral signals (like graph-based features or deep-temporal windows) without refactoring the entire transformation engine.

For **Transaction Sequencing**, a window-based approach is implemented using Polars' `shift` and `over` functions. This allows for the calculation of complex stateful features like `spatial_velocity` and `amount_deviation_z_score` without the overhead of row-by-row iteration. The decision to perform these calculations at the "Silver" layer ensures that the final "Gold" master table is pre-enriched with predictive signals, reducing the training time for downstream models.

In the **Network Intelligence** module, a "Proxy Entity" strategy is used. Instead of building a full N:N customer relationship graph (which is memory-intensive), the risk reputation of shared entities like IP addresses and User Agents is calculated. This allows the system to identify "Suspicious Clusters" where multiple customers share a single high-fraud entity, capturing coordinated attack signals with high computational efficiency.

## System Integration
These modules are the primary transformation components of the `etl.rs` binary. They consume "Bronze" tables from ClickHouse and produce "Silver" feature tables. The logic defined here is also mirrored in the `scorer.py` service to ensure training-serving parity during real-time inference.

## Known Issues
A simple Euclidean distance formula is currently used for **Spatial Velocity** calculations. As noted in the `etl_schema.md`, this approximation becomes inaccurate over large distances. Implementation of the Haversine formula within the Polars transformation is required to ensure geographic precision. 

Furthermore, the **Campaign Detection** logic in `campaign.rs` is currently based on a fixed 48-hour time gap. This is a heuristic that may fail to capture long-running, low-frequency attack campaigns. This threshold should be moved to the configuration or a more dynamic "Sessionization" strategy implemented to account for different adversarial behaviors.
