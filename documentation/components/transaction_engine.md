# Core Simulation Engine (`transaction_gen.rs`)

## Summary
The `transaction_gen.rs` module is the primary logic engine of RiskFabric. It is responsible for simulating the financial lifecycle of every card in the system over a specified lookback period (default 365 days). It transforms static entity data into a high-fidelity stream of behavioral events, incorporating spatial realism, temporal patterns, and adversarial mutations in a single execution pass.

## Architectural Decisions
The engine uses a **One-Pass Parallel Architecture**. By using `rayon` to iterate over cards, all logic—including merchant selection, timestamp generation, amount calculation, and fraud injection—occurs within a single parallelized loop. This eliminates the need for multi-pass joins and is a key factor in the project's performance.

For spatial realism, the system implements a **Hierarchical Selection Strategy** using H3 indices. Merchants are selected based on a probabilistic proximity model: 80% are "super-local" (Res 6), 15% are "district-level" (Res 4), 3% are "state-level," and 2% are "global." This creates realistic spending clusters around a customer's home while allowing for occasional travel or remote spending.

To ensure reproducibility, **Deterministic Seeding** is used at the card level. Every card's random number generator is seeded with a combination of the global seed, a salt, and a hash of the card ID. This ensures that a specific card will always generate the exact same transaction history across different runs, provided the global configuration remains unchanged.

## System Integration
This engine is the central utility consumed by both the **Batch Generator** (`generate.rs`) and the **Streaming Generator** (`stream.rs`). It acts as a pure function that takes configuration, spatial indices, and entity maps as input and produces vectors of `Transaction` and `FraudMetadata` as output.

## Known Issues
Timestamp generation is implemented by sorting a local vector of dates for each card. While this ensures that transactions are chronologically ordered *per card*, it does not guarantee a global chronological order across the entire dataset during batch generation. ClickHouse is currently used to perform the final global sort. 

Additionally, the spatial distribution weights (80/15/3/2) are hardcoded directly into the logic. Moving these to `transaction_config.yaml` would allow users to simulate different mobility profiles—for example, a "commuter" population would require a higher Res 4 weight compared to a "rural" population.
